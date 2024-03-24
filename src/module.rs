
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::{io, thread};
use std::path::Path;
use std::process::exit;
use std::sync::{Arc, RwLock};
use crate::builtin::{cmd, copy, move_file, replace};
use crate::context::{Context, PipelineContextValue};
use crate::engine::{PipelineEngine};
use crate::error::{PipelineError, PipelineResult};
use crate::v1;
use crate::v1::interpreter::Interpreter;

use crate::v1::parser::FnDef;
use crate::v1::types::Dynamic;

trait NativeFunction<Marker>{
    fn into_pipe_function(self) ->Arc<PipeFn>;
}
trait NativeType{}
impl NativeType for String{}
impl NativeType for i64{}
impl NativeType for f64{}

type PipeFn= dyn Send+Sync+ Fn(Arc<RwLock<dyn Context<PipelineContextValue>>>, Vec<Dynamic>) -> PipelineResult<Dynamic>;
#[derive(Clone)]
pub enum Function{
    Native(Arc<PipeFn>),
    Script(Box<FnDef>)
}
impl Debug for Function{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Function::Native(_) => {
                write!(f,"Native Function")
            }
            Function::Script(s) => {
                write!(f,"{s:?}")
            }
        }

    }
}
#[derive(Clone,Debug)]
pub struct Module{
    name:String,
    functions:HashMap<String,Function>
}

impl Function {
    pub fn call(&self, ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>,  args:Vec<Dynamic>) ->PipelineResult<Dynamic>{
        match self {
            Function::Native(n) => {
                let r=(*n)(ctx,args);
                return r

            }
            Function::Script(s) => {
                Ok(Dynamic::Unit)
            }
        }
    }
}
impl Module{
    pub fn new(name:impl Into<String>)->Self{
        Self{
            name:name.into(),
            functions:HashMap::new(),
        }
    }
    pub fn merge(&mut self,module: &Module){
        for (k,v) in &module.functions{
            self.functions.insert(k.clone(),v.clone());
        }
    }
    pub fn with_std_module()->Self{
        let mut std=Module::new("std");
        std.register_pipe_function("print",|ctx,args|{
            for v in args{
                if v.is_variable(){
                    let variable=v.as_variable().unwrap();
                    let v=PipelineEngine::context_with_dynamic(&ctx,variable.as_str());
                    match v {
                        None => {
                            return Err(PipelineError::VariableUndefined(variable))
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
        });
        std.register_pipe_function("println",|ctx,args|{
            for v in args{
                if v.is_variable(){
                    let variable=v.as_variable().unwrap();
                    let v=PipelineEngine::context_with_dynamic(&ctx,variable.as_str());
                    match v {
                        None => {
                            return Err(PipelineError::VariableUndefined(variable))
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
        });
        std.register_pipe_function("readLine",|ctx,args|{
            let mut input = String::new();
            io::stdin().read_line(&mut input).expect("无法读取输入");
            Ok(Dynamic::String(input))
        });
        std.register_pipe_function("cmd",|ctx,args| {
            let c=args.get(0).unwrap().as_string().unwrap();
            return cmd(c.as_str(),ctx);

        });
        std.register_pipe_function("env",|ctx,args| {
            let k=args.get(0).unwrap().as_string().unwrap();
            let v=args.get(1).unwrap().as_string().unwrap();
            let env=PipelineEngine::context_with_env(&ctx);;
            let mut env=env.write().unwrap();
            env.insert(k,v);
            Ok(Dynamic::Unit)

        });
        std.register_pipe_function("workspace",|ctx,args| {
            let global=PipelineEngine::context_with_global_state(&ctx);
            let arg=args.get(0).unwrap().as_string().unwrap();
            if !Path::new(arg.as_str()).exists(){
                let source=PipelineEngine::context_with_global_value(&ctx,"source");
                let pos=PipelineEngine::context_with_position(&ctx);
                let c=source.chars().collect();
                let (row,col)=pos.get_row_col(&c);
                println!("\x1b[31m  {}|{col}   {:}\x1b[0m",row+1,pos.get_raw_string(&c));
                println!("\x1b[31m[Error]:路径\"{arg}\"不存在\x1b[0m");
                exit(0);
            }
            global.write().unwrap().set_value("workspace",arg);
            return Ok(Dynamic::Unit)
        });
        std.register_pipe_function("copy",|ctx,args| {
            let source=args.get(0).unwrap().as_string().unwrap();
            let target=args.get(1).unwrap().as_string().unwrap();
            copy(ctx,source.as_str(),target.as_str());
            return Ok(Dynamic::Unit)
        });
        std.register_pipe_function("replace",|ctx,args| {
            let path=args.get(0).unwrap().as_string().unwrap();
            let regex=args.get(1).unwrap().as_string().unwrap();
            let replace_content=args.get(2).unwrap().as_string().unwrap();
            replace(ctx,path.as_str(),regex.as_str(),replace_content.as_str());
            return Ok(Dynamic::Unit)
        });
        std.register_pipe_function("move",|ctx,args| {
            let source=args.get(0).unwrap().as_string().unwrap();
            let target=args.get(1).unwrap().as_string().unwrap();
            move_file(ctx,source.as_str(),target.as_str());
            return Ok(Dynamic::Unit)
        });
        // e.register_fn("max",|ctx,args|Box::pin(async move {
        //     let first=args.get(0).unwrap();
        //     let mut max=first.convert_float().unwrap();
        //     for a in &args{
        //         let i=a.convert_float().unwrap();
        //         if i>max{
        //             max=i
        //         }
        //     }
        //     return Ok(Dynamic::Float(max))
        // }));
        return std
    }
    pub fn with_pipe_module()->Self{
        let  mut pipe=Module::new("pipe");
        pipe.register_pipe_function("pipeline",|ctx,args| {
            let pipeline_name=args.get(0).unwrap().as_string().unwrap();
            let blocks=args.get(1).unwrap().as_fn_ptr().unwrap().fn_def.unwrap().body;
            let mut e=PipelineEngine::default_with_pipeline();
            let share_module=PipelineEngine::context_with_shared_module(&ctx);
            let i=Interpreter::with_shared_module(share_module);
            e.set_interpreter(&i);
            let pipeline=PipelineEngine::context_with_global_value(&ctx,"path_pipeline");
            let ctx=PipelineEngine::with_value(ctx,"join_set",PipelineContextValue::JoinSet(Arc::new(std::sync::RwLock::new(vec![]))));
            if pipeline==pipeline_name||pipeline=="all"{
                e.eval_stmt_blocks_from_ast_with_context(ctx.clone(),blocks).unwrap();
            }
            let join_set=PipelineEngine::context_with_join_set(&ctx,"join_set");
            let mut join_set=join_set.write().unwrap();
            while !join_set.is_empty(){
               let e= join_set.pop().unwrap();
                e.join().unwrap();
            }
            Ok(Dynamic::Unit)
        });
        pipe.register_pipe_function("parallel",|ctx,args| {
            let pipeline_name=args.get(0).unwrap().as_string().unwrap();
            let blocks=args.get(1).unwrap().as_fn_ptr().unwrap().fn_def.unwrap().body;
            let mut e=PipelineEngine::default();
            let ctx=PipelineEngine::with_value(ctx,"$env",PipelineContextValue::Env(Arc::new(std::sync::RwLock::new(HashMap::new()))));
            let pipeline=PipelineEngine::context_with_global_value(&ctx,"path_task");
            let logger=PipelineEngine::context_with_logger(&ctx,"logger");
            let logger=logger.as_logger().unwrap();
            logger.write().unwrap().set_parallel(true);
            if pipeline==pipeline_name||pipeline.as_str()=="all"{
                let join=PipelineEngine::context_with_join_set(&ctx,"join_set");
                let mut join=join.write().unwrap();
                let handle=thread::spawn(move||{
                    let ctx=PipelineEngine::with_value(ctx,"op_join_set",PipelineContextValue::JoinSet(Arc::new(RwLock::new(vec![]))));
                    let ctx=PipelineEngine::with_value(ctx,"$task_name",PipelineContextValue::Local(pipeline_name.into()));
                    e.eval_stmt_blocks_from_ast_with_context(ctx.clone(),blocks).unwrap();
                    let join_set=PipelineEngine::context_with_join_set(&ctx,"op_join_set");
                    let mut join_set=join_set.write().unwrap();
                    while !join_set.is_empty(){
                        let e= join_set.pop().unwrap();
                        e.join().unwrap();
                    }
                    return Ok(())
                });
                join.push(handle);
            }
            Ok(Dynamic::Unit)
        });
        pipe.register_pipe_function("step",|ctx,args| {
            let pipeline_name=args.get(0).unwrap().as_string().unwrap();
            let mut ptr=args.get(1).unwrap().as_fn_ptr().unwrap();
            let mut e=PipelineEngine::default();
            let ctx=PipelineEngine::with_value(ctx,"$env",PipelineContextValue::Env(Arc::new(std::sync::RwLock::new(HashMap::new()))));
            let pipeline=PipelineEngine::context_with_global_value(&ctx,"path_task");
            if pipeline==pipeline_name||pipeline.as_str()=="all"{
                let ctx=PipelineEngine::with_value(ctx,"op_join_set",PipelineContextValue::JoinSet(Arc::new(std::sync::RwLock::new(vec![]))));
                let ctx=PipelineEngine::with_value(ctx,"$task_name",PipelineContextValue::Local(pipeline_name.into()));
                ptr.call(&mut e,ctx.clone()).unwrap();
                let join_set=PipelineEngine::context_with_join_set(&ctx,"op_join_set");
                let mut join_set=join_set.write().unwrap();
                while !join_set.is_empty(){
                    let e= join_set.pop().unwrap();
                    e.join().unwrap();
                }
            }
            Ok(Dynamic::Unit)
        });
        return pipe
    }
    pub fn register_native_function<A:'static,F>(&mut self, name:impl Into<String>, f:F )
    where F:NativeFunction<A>
    {
        self.functions.insert(name.into(),Function::Native(f.into_pipe_function()));
    }
    pub fn register_pipe_function(&mut self,name:impl Into<String>,f:impl Send+Sync+Fn(Arc<RwLock<dyn Context<PipelineContextValue>>>,Vec<Dynamic>)->PipelineResult<Dynamic> + 'static){
        let a: Arc<PipeFn> = Arc::new(f);
        self.functions.insert(name.into(),Function::Native(a));
    }
    pub fn register_script_function(&mut self,name:impl Into<String>,f:FnDef){
        self.functions.insert(name.into(),Function::Script(Box::new(f)));
    }
    pub fn call(&mut self,ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>,function_name:impl Into<String>,args: Vec<Dynamic>)->PipelineResult<Dynamic>{
        let name=function_name.into();
        let f=self.functions.get(name.clone().as_str());
        match f {
            None => {Err(PipelineError::FunctionUndefined(name))}
            Some(f) => {
                let r=f.call(ctx,args);
                return r
            }
        }
    }
    pub fn get_function(&self,name:impl Into<String>)->Option<Function>{
        let r=self.functions.get(name.into().as_str());
        match r {
            None => {None}
            Some(s) => {Some(s.clone())}
        }
    }
}
impl<
    T:Fn(A,B)->Ret + 'static + std::marker::Send + std::marker::Sync,A:NativeType + From<Dynamic>,
    B:NativeType + From<Dynamic>,
    Ret:NativeType + From<Dynamic>>
NativeFunction<(A,B)> for T
    where Dynamic: From<Ret>
{
    fn into_pipe_function(self) -> Arc<PipeFn> {
        Arc::new(move |ctx:Arc<RwLock<dyn Context< crate::context::PipelineContextValue >>>, args:Vec< crate::v1::types::Dynamic >|->PipelineResult<Dynamic>{
            let mut it=args.iter();
            let A=it.next().unwrap().clone().into();
            let B=it.next().unwrap().clone().into();
            let r=self(A,B);
            return Ok(r.into())
        })
    }
}
impl <T:Fn(A) + 'static + std::marker::Send + std::marker::Sync,A:NativeType + std::convert::From<v1::types::Dynamic>>NativeFunction<(A)> for T {
    fn into_pipe_function(self) -> Arc<PipeFn> {
        Arc::new(move|ctx:Arc<RwLock<dyn Context< crate::context::PipelineContextValue >>>, args:Vec< crate::v1::types::Dynamic >|->PipelineResult<Dynamic>{
            self(args[0].clone().into());
            Ok(Dynamic::Unit)
        })
    }
}
#[test]
fn test_modules(){
    let mut module=Module::new("std");
    module.register_native_function("add",|a:i64,b:i64|{
       return a+b
    });
    module.register_pipe_function("parallel",|ctx,args|{
        let out:PipelineResult<Dynamic>=tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
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
            });
        return out.unwrap();

    });
    let r=module.call("add",vec![12.into(),13.into()]).unwrap();
    println!("{r}")
}