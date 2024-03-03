use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::context::{Context, EmptyContext};
use crate::context::PipelineContextValue;
use crate::engine::{PipelineEngine, PipelineResult};
use crate::v1::expr::{Expr, FnCallExpr};
use crate::v1::interpreter::EvalError::FunctionUndefined;
use crate::v1::stmt::Stmt;
use crate::v1::types::Dynamic;

pub type EvalFn = fn(Arc<RwLock<dyn Context<PipelineContextValue>>>, Vec<Dynamic>) -> Pin<Box<dyn Future<Output = EvalResult<Dynamic>> + Send + 'static>>;
#[derive(Clone,Debug)]
pub struct Interpreter{
    pub builtin_fn_lib:HashMap<String,EvalFn>,
}

impl Interpreter{
    pub fn new()->Self{
        Self{builtin_fn_lib:HashMap::new()}
    }
    pub fn register_fn(&mut self,name:&str,f:EvalFn){
        self.builtin_fn_lib.insert(String::from(name),f);
    }
    pub async fn eval_stmt(&mut self,stmt:Stmt)->EvalResult<()>{
        match stmt {
            Stmt::FnCall(fc, _) => {
                self.eval_fn_call_expr(*fc).await?;
            }
            Stmt::Let(l,pos)=>{
                let ctx=PipelineEngine::background();
                self.eval_let_stmt(ctx,l).await?;
            }
            Stmt::Noop => {}
        }
        Ok(())
    }
    #[async_recursion::async_recursion]
    pub async fn eval_stmt_with_context(&mut self, ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>, stmt:Stmt) ->EvalResult<()>{
        match stmt {
            Stmt::FnCall(fc, pos) => {
                let ctx=PipelineEngine::with_value(ctx,"$pos",pos.into());
                self.eval_fn_call_expr_with_context(ctx,*fc).await?;
            }
            Stmt::Let(l,pos)=>{
                self.eval_let_stmt(ctx,l).await?;
            }
            Stmt::Noop => {}
        }
        Ok(())
    }
    pub async fn eval_let_stmt(&mut self, ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>, l:Box<(String,Expr)>)->EvalResult<()>{
        let scope=PipelineEngine::context_with_scope(&ctx).await;
        let d=self.eval_expr(ctx,l.1).await?;

        scope.write().await.set(l.0.as_str(),d);
        Ok(())
    }
    pub async fn eval_expr(&mut self,ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>,expr:Expr)->EvalResult<Dynamic>{
        if expr.is_fn_call(){
            let mut ptr=expr.dynamic().as_fn_ptr().unwrap();
            let mut e=PipelineEngine::default();
            e.set_interpreter(self);
            let r=ptr.call(&mut e,ctx).await;
            return match r {
                Ok(d) => {Ok(d)}
                Err(e) => {Err(EvalError::FunctionUndefined(ptr.name))}
            }
        }
        Ok(expr.dynamic())
    }
    pub async fn eval_fn_call_expr(&mut self,f:FnCallExpr)->EvalResult<Dynamic>{
        let c=Arc::new(RwLock::new(EmptyContext::new()));
       self.eval_fn_call_expr_with_context(c,f).await
    }
    #[async_recursion::async_recursion]
    pub async fn eval_fn_call_expr_with_context(&mut self, ctx: Arc<RwLock<dyn Context<PipelineContextValue>>>, f:FnCallExpr) ->EvalResult<Dynamic>{
        let func= self.builtin_fn_lib.get(f.name.as_str());
        let args:Vec<Dynamic>=f.args.iter().map(|e|e.dynamic()).collect();

        let mut v=vec![];
        for d in args{
            if d.is_fn_ptr(){
                let mut ptr=d.as_fn_ptr().unwrap();
                let mut e=PipelineEngine::new_raw();
                e.set_interpreter(self);
                let d=ptr.call(&mut e,ctx.clone()).await.unwrap();
                v.push(d);
                continue
            }
            v.push(d);
        }
        match func {
            None => {Err(FunctionUndefined(f.name))}
            Some(func) => {func(ctx,v).await}
        }
    }
}
pub type EvalResult<T>=Result<T, EvalError>;
#[derive(Debug,Clone)]
pub enum EvalError{
    FunctionUndefined(String),
    VariableUndefined(String)
}