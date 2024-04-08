use std::any::Any;
use std::collections::HashMap;
use std::io;
use std::io::Read;
use std::rc::Weak;
use std::sync::{Arc,RwLock};
use std::thread::JoinHandle;
use scanner_rust::Scanner;
use crate::context::{AppContext, Context, EmptyContext, Scope, ValueContext};
use crate::context::PipelineContextValue;
use crate::error::{ PipelineResult};
use crate::logger::PipelineLogger;
use crate::module::Module;
use crate::v1::ast::AST;
use crate::v1::expr::{Expr, FnCallExpr};
use crate::v1::interpreter::{  Interpreter};
use crate::v1::lexer::Lexer;
use crate::v1::parser::{FnDef, PipelineParser};
use crate::v1::position::Position;
use crate::v1::stmt::Stmt;
use crate::v1::types::{Dynamic, Value};

pub struct PipelineEngine{
    source:String,
    parser:PipelineParser,
    interpreter:Interpreter,
    fn_lib:Vec<FnDef>
}


impl Default for PipelineEngine {
    fn default() -> Self {
        let mut e=PipelineEngine::new_raw();
        return e
    }
}
impl PipelineEngine{
    pub fn new_raw()->Self{
        let mut i=Interpreter::new();
        let std=Module::with_std_module();
        i.register_module("std",std);
        i.merge_into_main_module("std");


        Self{
            parser:PipelineParser::new(),
            interpreter:i,
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
   pub(crate) fn default_with_parallel() ->Self{
        let mut default =PipelineEngine::default();
        return default
    }
    pub fn register_module(&mut self,module: Module){
        self.interpreter.register_module(module.get_name(),module)
    }
    pub fn default_with_pipeline()->Self{
        let mut default=PipelineEngine::default_with_parallel();
        let pipe=Module::with_pipe_module();
        default.interpreter.register_module("pipe",pipe);
        return default
    }
    pub fn context_with_dynamic(ctx:&Arc<RwLock<dyn Context<PipelineContextValue>>>,key:impl AsRef<str>)->Option<Value>{
        let scope=  ctx.read().unwrap().value("$scope").unwrap();
        let scope=scope.as_scope().unwrap();
        let d=scope.read().unwrap();
        let d=d.get(key.as_ref());
       return d
    }
    pub fn context_with_shared_module(ctx:&Arc<RwLock<dyn Context<PipelineContextValue>>>)->Arc<RwLock<Module>>{
        let module=ctx.read().unwrap().value("$shared_module").unwrap();
        let module=module.as_shared_module().unwrap();
        return module
    }
    pub fn context_with_modules(ctx:&Arc<RwLock<dyn Context<PipelineContextValue>>>)->Arc<RwLock<HashMap<String,Module>>>{
        let module=ctx.read().unwrap().value("$modules").unwrap();
        let module=module.as_modules().unwrap();
        return module
    }
    pub  fn context_with_global_value(ctx:&Arc<RwLock<dyn Context<PipelineContextValue>>>,key:impl AsRef<str>)->String{
        let global=PipelineEngine::context_with_global_state(ctx);
        let global=global.read().unwrap();
        let res=global.value(key.as_ref()).unwrap();
        return res.clone()
    }
    pub  fn context_with_join_set(ctx:&Arc<RwLock<dyn Context<PipelineContextValue>>>,key:&str)->Arc<RwLock<Vec<JoinHandle<PipelineResult<()>>>>>{
        let join=ctx.read().unwrap().value(key).unwrap();
        join.as_join_set().unwrap()
    }
    pub  fn context_with_scope(ctx:&Arc<RwLock<dyn Context<PipelineContextValue>>>)->Arc<RwLock<Scope>>{
        let join=ctx.read().unwrap().value("$scope").unwrap();
        let scope=join.as_scope().unwrap();
        scope
    }
    pub  fn context_with_logger(ctx:&Arc<RwLock<dyn Context<PipelineContextValue>>>,key:&str)->PipelineContextValue{
        let  join =ctx.read().unwrap().value(key).unwrap();
        return join
    }
    pub fn context_with_position(ctx:&Arc<RwLock<dyn Context<PipelineContextValue>>>)->Position{
        let pos=ctx.read().unwrap().value("$pos").unwrap();
        pos.as_position().unwrap()
    }
    pub  fn context_with_global_state(ctx:&Arc<RwLock<dyn Context<PipelineContextValue>>>)->Arc<RwLock<AppContext<String>>>{
        let  join =ctx.read().unwrap().value("$global_state").unwrap();
        return join.as_global_state().unwrap()
    }
    pub  fn context_with_local(ctx:&Arc<RwLock<dyn Context<PipelineContextValue>>>,key:&str)->String{
        let  join =ctx.read().unwrap().value(key).unwrap();
        return join.as_local().unwrap()
    }
    pub  fn context_with_native(ctx:&Arc<RwLock<dyn Context<PipelineContextValue>>>,key:&str)->Arc<RwLock<dyn Any+Send+Sync>>{
        let  join =ctx.read().unwrap().value(key).unwrap();
        return join.as_native().unwrap()
    }
    pub  fn context_with_env(ctx:&Arc<RwLock<dyn Context<PipelineContextValue>>>)->Arc<RwLock<HashMap<String,String>>>{
        let  join =ctx.read().unwrap().value("$env");
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
        scope.set("true",true.into());
        scope.set("false",false.into());
        let ctx=PipelineEngine::with_value(ctx,"$scope",PipelineContextValue::Scope(Arc::new(RwLock::new(scope))));
        let ctx=PipelineEngine::with_value(ctx,"$sc",PipelineContextValue::Native(Arc::new(RwLock::new(Scanner::new(io::stdin())))));
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
        for (_,class) in self.interpreter.main_module.read().unwrap().get_classes(){
            self.parser.register_predefined_class(class.clone());
        }
        let stmts=self.parser.parse_stmt_blocks()?;
        let classes=self.parser.get_classes();
        for (_,class) in classes{
            self.interpreter.main_module.write().unwrap().register_class(class.clone())
        }
        // println!("{:#?}",classes);
        self.fn_lib=self.parser.get_fn_lib();
        let m=self.parser.get_modules();
        for m0 in m{
            self.interpreter.register_module(m0.get_name(),m0.clone())
        }
        for lib in &self.fn_lib{
            self.interpreter.main_module.write().unwrap().register_script_function(lib.clone().name,lib.clone());
        }
        return Ok(stmts)
    }
    #[allow(unused)]
    pub  fn eval_stmt_from_ast(&mut self,stmt:Stmt)->PipelineResult<()>{
        self.interpreter.eval_stmt(stmt).unwrap();
        Ok(())
    }
    #[allow(unused)]
    pub  fn eval_stmt_from_ast_with_context(&mut self,ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>,stmt:Stmt)->PipelineResult<Value>{
        let a=self.interpreter.eval_stmt_with_context(ctx,stmt);
        return a

    }
    #[allow(unused)]
    pub fn eval_stmt_blocks_from_ast(&mut self,stmts:Vec<Stmt>)->PipelineResult<()>{
        let ctx=PipelineEngine::background();
        for stmt in stmts{
            self.eval_stmt_from_ast_with_context(ctx.clone(),stmt)?;
        }
        Ok(())
    }
    #[allow(unused)]
    pub  fn eval_expr_from_ast(&mut self,expr:Expr)->PipelineResult<Value>{
        let ctx=PipelineEngine::background();
        let a=self.interpreter.eval_expr(ctx,expr).unwrap();;
        return Ok(a)
    }
    #[allow(unused)]
    pub  fn eval_expr(&mut self,script:impl AsRef<str>)->PipelineResult<Value>{
        let lexer=Lexer::from_script(script);
        self.parser.set_lexer(lexer);
        let ast=self.parser.parse_expr().expect("解析错误");

        let r=self.eval_expr_from_ast(ast);
        return r
    }
    #[allow(unused)]
    pub  fn eval_stmt(&mut self,script:impl AsRef<str>)->PipelineResult<Value>{
        let lexer=Lexer::from_script(script);
        self.parser.set_lexer(lexer);
        let ast=self.parser.parse_stmt().expect("解析错误");
        let r=self.eval_stmt_from_ast(ast);
        return Ok(().into())
    }
    #[allow(unused)]
    pub  fn eval_stmt_blocks(&mut self,script:impl AsRef<str>)->PipelineResult<Value>{
        let lexer=Lexer::from_script(script);
        self.parser.set_lexer(lexer);
        let ast=self.parser.parse_stmt_blocks().expect("解析错误");
        let r=self.eval_stmt_blocks_from_ast(ast);
        return Ok(().into())
    }
    #[allow(unused)]
    pub  fn eval_stmt_blocks_from_ast_with_context(&mut self,ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>,stmts:Vec<Stmt>)->PipelineResult<Value>{
        for stmt in stmts{
            let r=self.eval_stmt_from_ast_with_context(ctx.clone(),stmt)?;
            if let Value::Immutable(Dynamic::Unit)=r{
                continue
            }
            return Ok(r);
        }
        Ok(().into())
    }
    #[allow(unused)]
    pub fn eval_fn_call_expr_from_ast(&mut self,ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>,expr:FnCallExpr)->PipelineResult<Value>{
        let r=self.interpreter.eval_fn_call_expr_with_context(ctx,expr);
        return r
    }
}