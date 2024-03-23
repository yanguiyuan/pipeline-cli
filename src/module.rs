use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc};
use clap::builder::Str;
use tokio::sync::RwLock;
use crate::context::{Context, PipelineContextValue};
use crate::engine::{PipelineEngine, PipelineError, PipelineResult};
use crate::v1;
use crate::v1::expr::Op;
use crate::v1::interpreter::EvalError;
use crate::v1::parser::FnDef;
use crate::v1::types::Dynamic;

trait NativeFunction<Marker>{
    fn into_pipe_function(self) ->Box<dyn Fn(Arc<RwLock<dyn Context<PipelineContextValue>>>,Vec<Dynamic>)->Dynamic>;
}
trait NativeType{}
impl NativeType for String{}
impl NativeType for i64{}
impl NativeType for f64{}
pub enum Function{
    Native(Box<dyn Fn(Arc<RwLock<dyn Context<PipelineContextValue>>>,Vec<Dynamic>)->Dynamic>),
    Script(Box<FnDef>)
}
pub struct Module{
    name:String,
    functions:HashMap<String,Function>
}

impl Function {
    pub fn call(&self,ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>,args:Vec<Dynamic>)->PipelineResult<Dynamic>{
        match self {
            Function::Native(n) => {
                let r=(*n)(ctx,args);
                Ok(r)

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
    pub fn register_native_function<A:'static,F>(&mut self, name:impl Into<String>, f:F )
    where F:NativeFunction<A>
    {
        self.functions.insert(name.into(),Function::Native(f.into_pipe_function()));
    }
    pub fn register_pipe_function(&mut self,name:impl Into<String>,f:impl Fn(Arc<RwLock<dyn Context<PipelineContextValue>>>,Vec<Dynamic>)->Dynamic + 'static){
        let a: Box<dyn  Fn(Arc<RwLock<dyn Context<PipelineContextValue>>>,Vec<Dynamic>)->Dynamic> = Box::new(f);
        self.functions.insert(name.into(),Function::Native(a));
    }
    pub fn call(&mut self,function_name:impl Into<String>,args: Vec<Dynamic>)->PipelineResult<Dynamic>{
        let name=function_name.into();
        let f=self.functions.get(name.clone().as_str());
        match f {
            None => {Err(PipelineError::EvalFailed(EvalError::FunctionUndefined(name)))}
            Some(f) => {
                let ctx=PipelineEngine::background();
                let r=f.call(ctx,args);
                return r
            }
        }
    }
    pub fn get_function(&self,name:impl Into<String>)->Option<Function>{
        return None
    }
}
impl<
    T:Fn(A,B)->Ret + 'static,A:NativeType + From<Dynamic>,
    B:NativeType + From<Dynamic>,
    Ret:NativeType + From<Dynamic>>
NativeFunction<(A,B)> for T
    where Dynamic: From<Ret>
{
    fn into_pipe_function(self) -> Box<dyn Fn(Arc<RwLock<dyn Context<PipelineContextValue>>>, Vec<Dynamic>) -> Dynamic> {
        Box::new(move |ctx:Arc<RwLock<dyn Context< crate::context::PipelineContextValue >>>, args:Vec< crate::v1::types::Dynamic >|->Dynamic{
            let mut it=args.iter();
            let A=it.next().unwrap().clone().into();
            let B=it.next().unwrap().clone().into();
            let r=self(A,B);
            return r.into()
        })
    }
}
impl <T:Fn(A) + 'static,A:NativeType + std::convert::From<v1::types::Dynamic>>NativeFunction<(A)> for T {
    fn into_pipe_function(self) -> Box<dyn Fn(Arc<RwLock<dyn Context<PipelineContextValue>>>, Vec<Dynamic>) -> Dynamic> {
        Box::new(move|ctx:Arc<RwLock<dyn Context< crate::context::PipelineContextValue >>>, args:Vec< crate::v1::types::Dynamic >|->Dynamic{
            self(args[0].clone().into());
            Dynamic::Unit
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