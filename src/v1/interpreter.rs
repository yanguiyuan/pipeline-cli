use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::context::{Context, EmptyContext, Scope};
use crate::context::PipelineContextValue;
use crate::engine::{PipelineEngine, PipelineResult};
use crate::v1::expr::{Expr, FnCallExpr, Op};
use crate::v1::interpreter::EvalError::FunctionUndefined;
use crate::v1::parser::FnDef;
use crate::v1::stmt::Stmt;
use crate::v1::types::Dynamic;
use crate::v1::types::Dynamic::FnPtr;

pub type EvalFn = fn(Arc<RwLock<dyn Context<PipelineContextValue>>>, Vec<Dynamic>) -> Pin<Box<dyn Future<Output = EvalResult<Dynamic>> + Send + 'static>>;
#[derive(Clone,Debug)]
pub struct Interpreter{
    pub builtin_fn_lib:HashMap<String,Function>,
}
#[derive(Clone,Debug)]
pub enum Function{
    Native(Box<EvalFn>),
    Script(Box<FnDef>)
}
impl Interpreter{
    pub fn new()->Self{
        Self{builtin_fn_lib:HashMap::new()}
    }
    pub fn register_fn(&mut self,name:&str,f:EvalFn){
        self.builtin_fn_lib.insert(String::from(name),Function::Native(Box::new(f)));
    }
    pub fn register_script_fn(&mut self,name:&str,f:&FnDef){
        self.builtin_fn_lib.insert(String::from(name),Function::Script(Box::new(f.clone())));
    }
    pub async fn eval_stmt(&mut self,stmt:Stmt)->EvalResult<Dynamic>{
        match stmt {
            Stmt::FnCall(fc, _) => {
                self.eval_fn_call_expr(*fc).await?;
            }
            Stmt::Let(l,pos)=>{
                let ctx=PipelineEngine::background();
                self.eval_let_stmt(ctx,l).await?;
            }
            Stmt::Return(e,pos)=>{
                let ctx=PipelineEngine::background();
                return self.eval_expr(ctx, *e).await
            }
            Stmt::Noop => {}
        }
        Ok(Dynamic::Unit)
    }
    #[async_recursion::async_recursion]
    pub async fn eval_stmt_with_context(&mut self, ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>, stmt:Stmt) ->EvalResult<Dynamic>{
        match stmt {
            Stmt::FnCall(fc, pos) => {
                let ctx=PipelineEngine::with_value(ctx,"$pos",pos.into());
                self.eval_fn_call_expr_with_context(ctx,*fc).await?;
            }
            Stmt::Let(l,pos)=>{
                self.eval_let_stmt(ctx,l).await?;
            }
            Stmt::Return(e,pos)=>{
                return self.eval_expr(ctx, *e).await
            }
            Stmt::Noop => {}
        }
        Ok(Dynamic::Unit)
    }
    pub async fn eval_let_stmt(&mut self, ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>, l:Box<(String,Expr)>)->EvalResult<()>{
        let scope=PipelineEngine::context_with_scope(&ctx).await;
        let d=self.eval_expr(ctx,l.1).await?;
        scope.write().await.set(l.0.as_str(),d);
        Ok(())
    }
    #[async_recursion::async_recursion]
    pub async fn eval_expr(&mut self,ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>,expr:Expr)->EvalResult<Dynamic>{
        match expr.clone() {
            Expr::FnCall(f, pos)=>{
                let mut ptr=expr.dynamic().as_fn_ptr().unwrap();
                let mut e=PipelineEngine::default();
                e.set_interpreter(self);
                let r=ptr.call(&mut e,ctx).await;
                return match r {
                    Ok(d) => {Ok(d)}
                    Err(e) => {Err(EvalError::FunctionUndefined(ptr.name))}
                }
            }
            Expr::Variable(i,pos)=>{
                let d=PipelineEngine::context_with_dynamic(&ctx,i).await;
                Ok(d)
            }
            Expr::BinaryExpr(op,l,r,_)=>{
                match op {
                    Op::Plus => {
                        let l_r=self.eval_expr(ctx.clone(),*l).await?;
                        let r_r=self.eval_expr(ctx.clone(),*r).await?;
                        return Ok(l_r+r_r)
                    }
                    Op::Mul=>{
                        let l_r=self.eval_expr(ctx.clone(),*l).await?;
                        let r_r=self.eval_expr(ctx.clone(),*r).await?;
                        return Ok(l_r*r_r)
                    }
                }
            }
            _=>Ok(expr.dynamic())
        }
    }
    pub async fn eval_fn_call_expr(&mut self,f:FnCallExpr)->EvalResult<Dynamic>{
        let c=Arc::new(RwLock::new(EmptyContext::new()));
       self.eval_fn_call_expr_with_context(c,f).await
    }
    #[async_recursion::async_recursion]
    pub async fn eval_fn_call_expr_with_context(&mut self, ctx: Arc<RwLock<dyn Context<PipelineContextValue>>>, f:FnCallExpr) ->EvalResult<Dynamic>{
        let mut v=vec![];
        for e in &f.args{
            let d=self.eval_expr(ctx.clone(),e.clone()).await.unwrap();
            if d.is_fn_ptr(){
                let mut ptr=d.as_fn_ptr().unwrap();
                if ptr.is_defer(){
                    v.push(d);
                    continue
                }
                let mut e=PipelineEngine::new_raw();
                e.set_interpreter(self);
                let d=ptr.call(&mut e,ctx.clone()).await.unwrap();
                v.push(d);
                continue
            }else if d.is_variable(){
                let d=d.as_variable().unwrap();
                let r=PipelineEngine::context_with_dynamic(&ctx,d.as_str()).await;
                v.push(r);
                continue
            }
            v.push(d);
        }

        let func= self.builtin_fn_lib.get(f.name.as_str()).clone();
        match func {
            None => {Err(FunctionUndefined(f.name))}
            Some(func) => {
                match func {
                    Function::Native(native_func) => {
                        native_func(ctx,v).await
                    }
                    Function::Script(fn_def) => {
                        let mut ptr=crate::v1::types::FnPtr::new(fn_def.name.as_str());
                        ptr.set_params(&f.args);
                        ptr.set_fn_def(fn_def);
                        let mut e=PipelineEngine::new_raw();
                        e.set_interpreter(self);
                        let mut scope=Scope::new();
                        let mut i=0;
                        for param in &fn_def.args{
                            scope.set(param.name.as_str(),v.get(i).unwrap().clone());
                            i+=1;
                        }
                        let ctx=PipelineEngine::with_value(ctx,"$scope",PipelineContextValue::Scope(Arc::new(RwLock::new(scope))));
                        Ok(ptr.call(&mut e,ctx).await.unwrap())
                    }
                }

            }
        }
    }
}
pub type EvalResult<T>=Result<T, EvalError>;
#[derive(Debug,Clone)]
pub enum EvalError{
    FunctionUndefined(String),
    VariableUndefined(String)
}