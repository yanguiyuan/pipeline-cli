use std::collections::HashMap;
use std::io;
use std::io::Read;
use std::path::Path;
use std::process::exit;
use std::sync::{Arc};
use tokio::sync::RwLock;
use crate::builtin::{cmd, copy, move_file, replace};
use crate::context::{AppContext, Context, EmptyContext, Scope, ValueContext};
use crate::context::PipelineContextValue;
use crate::logger::PipelineLogger;
use crate::v1::ast::AST;
use crate::v1::expr::{Expr, FnCallExpr};
use crate::v1::interpreter::{EvalError, EvalFn, EvalResult, Interpreter};
use crate::v1::interpreter::EvalError::VariableUndefined;
use crate::v1::lexer::Lexer;
use crate::v1::parser::{FnDef, ParseError, ParseResult, PipelineParser};
use crate::v1::position::Position;
use crate::v1::stmt::Stmt;
use crate::v1::types::Dynamic;

pub struct PipelineEngine{
    source:String,
    parser:PipelineParser,
    interpreter:Interpreter,
    fn_lib:Vec<FnDef>
}
pub type PipelineResult<T>=Result<T,PipelineError>;
#[derive(Debug,Clone)]
pub enum PipelineError{
    EvalFailed(EvalError),
    ParseFailed(ParseError)
}

impl Default for PipelineEngine {
    fn default() -> Self {
        let mut e=PipelineEngine::new_raw();
        e.register_fn("println",|ctx,args|Box::pin(async move {
            for v in args{
                if v.is_variable(){
                    let variable=v.as_variable().unwrap();
                    let v=PipelineEngine::context_with_dynamic(&ctx,variable.as_str()).await;
                    match v {
                        None => {
                            return Err(VariableUndefined(variable))
                        }
                        Some(v) => {
                            print!("{v}");
                            continue
                        }
                    }
                }
                print!("{v}");
            }
            println!();
            Ok(Dynamic::Unit)
        }));
        e.register_fn("print",|ctx,args|Box::pin(async move {
            for v in args{
                if v.is_variable(){
                    let variable=v.as_variable().unwrap();
                    let v=PipelineEngine::context_with_dynamic(&ctx,variable.as_str()).await;
                    match v {
                        None => {
                            return Err(VariableUndefined(variable))
                        }
                        Some(v) => {
                            print!("{v}");
                            continue
                        }
                    }
                }
                print!("{v}");
            }
            Ok(Dynamic::Unit)
        }));
        e.register_fn("readLine",|ctx,args|Box::pin(async move {
            let mut input = String::new();
            io::stdin().read_line(&mut input).expect("无法读取输入");
            Ok(Dynamic::String(input))
        }));
        e.register_fn("cmd",|ctx,args|Box::pin(async move {
            let c=args.get(0).unwrap().as_string().unwrap();
            return cmd(c.as_str(),ctx).await;

        }));
        e.register_fn("env",|ctx,args|Box::pin(async move {
            let k=args.get(0).unwrap().as_string().unwrap();
            let v=args.get(1).unwrap().as_string().unwrap();
            let env=PipelineEngine::context_with_env(&ctx).await;
            let mut env=env.write().await;
            env.insert(k,v);
            Ok(Dynamic::Unit)
        }));
        e.register_fn("workspace",|ctx,args|Box::pin(async move {
            let global=PipelineEngine::context_with_global_state(&ctx).await;
            let arg=args.get(0).unwrap().as_string().unwrap();
            if !Path::new(arg.as_str()).exists(){
                let source=PipelineEngine::context_with_global_value(&ctx,"source").await;
                let pos=PipelineEngine::context_with_position(&ctx).await;
                let c=source.chars().collect();
                let (row,col)=pos.get_row_col(&c);
                println!("\x1b[31m  {}|{col}   {:}\x1b[0m",row+1,pos.get_raw_string(&c));
                println!("\x1b[31m[Error]:路径\"{arg}\"不存在\x1b[0m");
                exit(0);
            }
            global.write().await.set_value("workspace",arg);
            return Ok(Dynamic::Unit)
        }));
        e.register_fn("copy",|ctx,args|Box::pin(async move {
            let source=args.get(0).unwrap().as_string().unwrap();
            let target=args.get(1).unwrap().as_string().unwrap();
            copy(ctx,source.as_str(),target.as_str()).await;
            return Ok(Dynamic::Unit)
        }));
        e.register_fn("replace",|ctx,args|Box::pin(async move {
            let path=args.get(0).unwrap().as_string().unwrap();
            let regex=args.get(1).unwrap().as_string().unwrap();
            let replace_content=args.get(2).unwrap().as_string().unwrap();
            replace(ctx,path.as_str(),regex.as_str(),replace_content.as_str()).await;
            return Ok(Dynamic::Unit)
        }));
        e.register_fn("move",|ctx,args|Box::pin(async move {
            let source=args.get(0).unwrap().as_string().unwrap();
            let target=args.get(1).unwrap().as_string().unwrap();
            move_file(ctx,source.as_str(),target.as_str()).await;
            return Ok(Dynamic::Unit)
        }));
        e.register_fn("max",|ctx,args|Box::pin(async move {
            let first=args.get(0).unwrap();
            let mut max=first.convert_float().unwrap();
            for a in &args{
                let i=a.convert_float().unwrap();
                if i>max{
                    max=i
                }
            }
            return Ok(Dynamic::Float(max))
        }));
        return e
    }
}
impl PipelineEngine{
    pub fn new_raw()->Self{
        Self{
            parser:PipelineParser::new(),
            interpreter:Interpreter::new(),
            source:String::new(),
            fn_lib:vec![]
        }
    }
    pub fn set_interpreter(&mut self,interpreter: &Interpreter){
        self.interpreter= interpreter.clone()
    }
    #[allow(unused)]
    pub fn get_fn_lib(&self)->Vec<FnDef>{
        self.fn_lib.clone()
    }
    #[allow(unused)]
    pub fn set_source(&mut self,source:&str){
        self.source=source.into();
    }
   fn default_with_parallel()->Self{
        let mut default =PipelineEngine::default();
        default.register_fn("parallel",  |ctx, args|Box::pin (async move {
            let pipeline_name=args.get(0).unwrap().as_string().unwrap();
            let blocks=args.get(1).unwrap().as_fn_ptr().unwrap().fn_def.unwrap().body;
            let mut e=PipelineEngine::default();
            let ctx=PipelineEngine::with_value(ctx,"$env",PipelineContextValue::Env(Arc::new(RwLock::new(HashMap::new()))));
            let pipeline=PipelineEngine::context_with_global_value(&ctx,"path_task").await;
            let logger=PipelineEngine::context_with_logger(&ctx,"logger").await;
            let logger=logger.as_logger().unwrap();
            logger.write().await.set_parallel(true);
            if pipeline==pipeline_name||pipeline.as_str()=="all"{
                let join=PipelineEngine::context_with_join_set(&ctx,"join_set").await;
                join.write().await.spawn(async move{
                    let ctx=PipelineEngine::with_value(ctx,"op_join_set",PipelineContextValue::JoinSet(Arc::new(RwLock::new(tokio::task::JoinSet::new()))));
                    let ctx=PipelineEngine::with_value(ctx,"$task_name",PipelineContextValue::Local(pipeline_name.into()));
                    e.eval_stmt_blocks_from_ast_with_context(ctx.clone(),blocks).await.unwrap();
                    let join_set=PipelineEngine::context_with_join_set(&ctx,"op_join_set").await;
                    while let Some(r)=join_set.write().await.join_next().await{
                        r.expect("错误").expect("TODO: panic message");
                    }
                    return Ok(())
                });
            }
            Ok(Dynamic::Unit)
        }));
       default.register_fn("step",  |ctx, args|Box::pin (async move {
           let pipeline_name=args.get(0).unwrap().as_string().unwrap();
           let mut ptr=args.get(1).unwrap().as_fn_ptr().unwrap();
           let mut e=PipelineEngine::default();
           let ctx=PipelineEngine::with_value(ctx,"$env",PipelineContextValue::Env(Arc::new(RwLock::new(HashMap::new()))));
           let pipeline=PipelineEngine::context_with_global_value(&ctx,"path_task").await;
           if pipeline==pipeline_name||pipeline.as_str()=="all"{
               let ctx=PipelineEngine::with_value(ctx,"op_join_set",PipelineContextValue::JoinSet(Arc::new(RwLock::new(tokio::task::JoinSet::new()))));
               let ctx=PipelineEngine::with_value(ctx,"$task_name",PipelineContextValue::Local(pipeline_name.into()));
                ptr.call(&mut e,ctx.clone()).await.unwrap();
               let join_set=PipelineEngine::context_with_join_set(&ctx,"op_join_set").await;
               while let Some(r)=join_set.write().await.join_next().await{
                   r.expect("错误").expect("TODO: panic message");
               }
           }
           Ok(Dynamic::Unit)
       }));
        return default
    }
    pub fn default_with_pipeline()->Self{
        let mut default=PipelineEngine::default_with_parallel();
        default.register_fn("pipeline",  |ctx, args|Box::pin (async move {
            let pipeline_name=args.get(0).unwrap().as_string().unwrap();
            let blocks=args.get(1).unwrap().as_fn_ptr().unwrap().fn_def.unwrap().body;
            let mut e=PipelineEngine::default_with_parallel();
            let pipeline=PipelineEngine::context_with_global_value(&ctx,"path_pipeline").await;
            let ctx=PipelineEngine::with_value(ctx,"join_set",PipelineContextValue::JoinSet(Arc::new(RwLock::new(tokio::task::JoinSet::new()))));
            if pipeline==pipeline_name||pipeline=="all"{
                e.eval_stmt_blocks_from_ast_with_context(ctx.clone(),blocks).await.unwrap();
            }
            let join_set=PipelineEngine::context_with_join_set(&ctx,"join_set").await;
            while let Some(r)=join_set.write().await.join_next().await{
                let _=r.expect("错误");
            }
            Ok(Dynamic::Unit)
        }));
        return default
    }
    pub async fn context_with_dynamic(ctx:&Arc<RwLock<dyn Context<PipelineContextValue>>>,key:impl AsRef<str>)->Option<Dynamic>{
        let scope=  ctx.read().await.value("$scope").await.unwrap();
        let scope=scope.as_scope().unwrap();
        let d=scope.read().await;
        let d=d.get(key.as_ref()).await;
        match d {
            None => {None}
            Some(d) => {
                Some(d.clone())
            }
        }
    }
    pub async fn context_with_global_value(ctx:&Arc<RwLock<dyn Context<PipelineContextValue>>>,key:impl AsRef<str>)->String{
        let global=PipelineEngine::context_with_global_state(ctx).await;
        let global=global.read().await;
        let res=global.value(key.as_ref()).unwrap();
        return res.clone()
    }
    pub async fn context_with_join_set(ctx:&Arc<RwLock<dyn Context<PipelineContextValue>>>,key:&str)->Arc<RwLock<tokio::task::JoinSet<PipelineResult<()>>>>{
        let join=ctx.read().await.value(key).await.unwrap();
        join.as_join_set().unwrap()
    }
    pub async fn context_with_scope(ctx:&Arc<RwLock<dyn Context<PipelineContextValue>>>)->Arc<RwLock<Scope>>{
        let join=ctx.read().await.value("$scope").await.unwrap();
        let scope=join.as_scope().unwrap();
        scope
    }
    pub async fn context_with_logger(ctx:&Arc<RwLock<dyn Context<PipelineContextValue>>>,key:&str)->PipelineContextValue{
        let mut join =ctx.read().await.value(key).await.unwrap();
        return join
    }
    pub async fn context_with_position(ctx:&Arc<RwLock<dyn Context<PipelineContextValue>>>)->Position{
        let pos=ctx.read().await.value("$pos").await.unwrap();
        pos.as_position().unwrap()
    }
    pub async fn context_with_global_state(ctx:&Arc<RwLock<dyn Context<PipelineContextValue>>>)->Arc<RwLock<AppContext<String>>>{
        let mut join =ctx.read().await.value("$global_state").await.unwrap();
        return join.as_global_state().unwrap()
    }
    pub async fn context_with_local(ctx:&Arc<RwLock<dyn Context<PipelineContextValue>>>,key:&str)->String{
        let mut join =ctx.read().await.value(key).await.unwrap();
        return join.as_local().unwrap()
    }
    pub async fn context_with_env(ctx:&Arc<RwLock<dyn Context<PipelineContextValue>>>)->Arc<RwLock<HashMap<String,String>>>{
        let mut join =ctx.read().await.value("$env").await;
        match join {
            None => {panic!("未设置$env,影响cmd运行")}
            Some(j) => {j.as_env().unwrap()}
        }
    }
    pub fn background()->Arc<RwLock<dyn Context<PipelineContextValue>>>{
        let empty=EmptyContext::new();
        let empty=Arc::new(RwLock::new(empty));
        //全局系统参数，不可被作为Dynamic变量读取
        let mut global=AppContext::new();
        global.set_value("workspace","./".into());
        global.set_value("path_pipeline","all".into());
        global.set_value("path_task","all".into());
        let ctx=Arc::new(RwLock::new(ValueContext::with_value(empty,"$global_state",PipelineContextValue::GlobalState(Arc::new(RwLock::new(global))))));
        let ctx=Arc::new(RwLock::new(ValueContext::with_value(ctx,"logger",PipelineContextValue::Logger(Arc::new(RwLock::new(PipelineLogger::new()))))));
        //全局作用域
        let mut scope=Scope::new();
        scope.set("true",Dynamic::Boolean(true));
        scope.set("false",Dynamic::Boolean(false));
        let ctx=PipelineEngine::with_value(ctx,"$scope",PipelineContextValue::Scope(Arc::new(RwLock::new(scope))));
        // let ctx=PipelineEngine::with_value(ctx,"$env",PipelineContextValue::Env(Arc::new(RwLock::new(HashMap::new()))));

        return ctx
    }
    pub  fn with_value(ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>,key:&'static str,value:PipelineContextValue)->Arc<RwLock<dyn Context<PipelineContextValue>>>{
        let ctx=Arc::new(RwLock::new(ValueContext::with_value(ctx,key,value)));
        return ctx
    }
    #[allow(unused)]
    pub fn compile_file(&mut self, path: impl AsRef<str>) ->PipelineResult<AST>{
        let lexer=Lexer::from_path(path);
        self.parser.set_lexer(lexer);
        let a=self.parser.compile_from_token_stream().unwrap();
        Ok(a)
    }
    pub fn compile_expr(&mut self,script:impl AsRef<str>)->PipelineResult<Expr>{
        let lexer=Lexer::from_script(script);
        self.parser.set_lexer(lexer);
        let expr=self.parser.parse_expr().unwrap();
        return Ok(expr)
    }
    #[allow(unused)]
    pub fn compile_stmt(&mut self,script:impl AsRef<str>)->PipelineResult<Stmt>{
        self.source=script.as_ref().into();
        let lexer=Lexer::from_script(script);
        self.parser.set_lexer(lexer);
        let stmt=self.parser.parse_stmt().unwrap();
        return Ok(stmt)
    }
    #[allow(unused)]
    pub fn compile_stmt_blocks(&mut self,script:impl AsRef<str>)->PipelineResult<Vec<Stmt>>{
        let lexer=Lexer::from_script(script);
        self.parser.set_lexer(lexer);
        let stmts=self.parser.parse_stmt_blocks();
        match stmts {
            Ok(s) => {
                self.fn_lib=self.parser.get_fn_lib();
                for lib in &self.fn_lib{
                    self.interpreter.register_script_fn(lib.name.as_str(),lib);
                }
                return Ok(s)
            }
            Err(e )=> {
                Err(PipelineError::ParseFailed(e))
            }
        }

    }
    #[allow(unused)]
    pub fn register_fn(&mut self,name:&str,func:EvalFn){
        self.interpreter.register_fn(name,func)
    }
    #[allow(unused)]
    pub async fn eval_stmt_from_ast(&mut self,stmt:Stmt)->PipelineResult<()>{
        self.interpreter.eval_stmt(stmt).await.unwrap();
        Ok(())
    }
    #[allow(unused)]
    pub async fn eval_stmt_from_ast_with_context(&mut self,ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>,stmt:Stmt)->PipelineResult<Dynamic>{
        let a=self.interpreter.eval_stmt_with_context(ctx,stmt).await;
        match a {
            Ok(a) => {Ok(a)}
            Err(e) => {
                Err(PipelineError::EvalFailed(e))
            }
        }

    }
    #[allow(unused)]
    pub async fn eval_stmt_blocks_from_ast(&mut self,stmts:Vec<Stmt>)->PipelineResult<()>{
        let ctx=PipelineEngine::background();
        for stmt in stmts{
            self.eval_stmt_from_ast_with_context(ctx.clone(),stmt).await?;
        }
        Ok(())
    }
    #[allow(unused)]
    pub async fn eval_expr_from_ast(&mut self,expr:Expr)->PipelineResult<Dynamic>{
        let ctx=PipelineEngine::background();
        let a=self.interpreter.eval_expr(ctx,expr).await.unwrap();;
        return Ok(a)
    }
    #[allow(unused)]
    pub async fn eval_expr(&mut self,script:impl AsRef<str>)->PipelineResult<Dynamic>{
        let lexer=Lexer::from_script(script);
        self.parser.set_lexer(lexer);
        let ast=self.parser.parse_expr().expect("解析错误");

        let r=self.eval_expr_from_ast(ast).await;
        return r
    }
    #[allow(unused)]
    pub async fn eval_stmt(&mut self,script:impl AsRef<str>)->PipelineResult<Dynamic>{
        let lexer=Lexer::from_script(script);
        self.parser.set_lexer(lexer);
        let ast=self.parser.parse_stmt().expect("解析错误");
        let r=self.eval_stmt_from_ast(ast).await;
        return Ok(Dynamic::Unit)
    }
    #[allow(unused)]
    pub async fn eval_stmt_blocks(&mut self,script:impl AsRef<str>)->PipelineResult<Dynamic>{
        let lexer=Lexer::from_script(script);
        self.parser.set_lexer(lexer);
        let ast=self.parser.parse_stmt_blocks().expect("解析错误");
        let r=self.eval_stmt_blocks_from_ast(ast).await;
        return Ok(Dynamic::Unit)
    }
    #[allow(unused)]
    pub async fn eval_stmt_blocks_from_ast_with_context(&mut self,ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>,stmts:Vec<Stmt>)->PipelineResult<Dynamic>{
        for stmt in stmts{
            let r=self.eval_stmt_from_ast_with_context(ctx.clone(),stmt).await?;
            if let Dynamic::Unit=r{
                continue
            }
            return Ok(r);
        }
        Ok(Dynamic::Unit)
    }
    #[allow(unused)]
    pub async fn eval_fn_call_expr_from_ast(&mut self,ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>,expr:FnCallExpr)->PipelineResult<Dynamic>{
        let r=self.interpreter.eval_fn_call_expr_with_context(ctx,expr).await;
        match r {
            Ok(d) => {Ok(d)}
            Err(e) => {
                Err(PipelineError::EvalFailed(e))
            }
        }
    }
}