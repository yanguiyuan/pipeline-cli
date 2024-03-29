use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::context::{Context, EmptyContext};
use crate::context::PipelineContextValue;
use crate::engine::{PipelineEngine};
use crate::error::{PipelineError, PipelineResult};
use crate::module::{ Module};
use crate::v1::expr::{Expr, FnCallExpr, Op};
use crate::v1::stmt::Stmt;
use crate::v1::types::Dynamic;

#[derive(Clone,Debug)]
pub struct Interpreter{
    pub modules:HashMap<String,Module>,
    pub main_module:Arc<RwLock<Module>>
}

impl Interpreter{
    pub fn new()->Self{
        // Self{builtin_fn_lib:HashMap::new()}
        let mut m=HashMap::new();
        Self{modules:m,main_module:Arc::new(RwLock::new(Module::new("main")))}
    }
    pub fn with_shared_module(sm:Arc<RwLock<Module>>)->Self{
        let mut m=HashMap::new();
        Self{modules:m,main_module:sm}
    }
    pub fn register_module(&mut self,name:impl Into<String>,module:Module){
        self.modules.insert(name.into(),module);
    }
    pub fn merge_into_main_module(&mut self,module_name: impl AsRef<str>){
        let mut target=self.modules.get(module_name.as_ref()).unwrap();
        self.main_module.write().unwrap().merge(target)
    }
    pub fn get_mut_module(&mut self,name:impl Into<String>)->Option<&mut Module>{
        let m=self.modules.get_mut(name.into().as_str());
        return m
    }
    pub  fn eval_stmt(&mut self,stmt:Stmt)->PipelineResult<Dynamic>{
        let ctx=PipelineEngine::background();
        return self.eval_stmt_with_context(ctx,stmt)
    }

    pub fn eval_stmt_with_context(&mut self, ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>, stmt:Stmt) ->PipelineResult<Dynamic>{
        match stmt {
            Stmt::FnCall(fc, pos) => {
                let ctx=PipelineEngine::with_value(ctx,"$pos",pos.into());
                self.eval_fn_call_expr_with_context(ctx,*fc)?;
            }
            Stmt::Import(s,_)=>{
                self.merge_into_main_module(s)
            }
            Stmt::Let(l,_)=>{
                self.eval_let_stmt(ctx,l)?;
            }
            Stmt::Return(e,_)=>{
                return self.eval_expr(ctx, *e)
            }
            Stmt::If(b,_)=>{
                for if_branch in b.get_branches(){
                    let d=self.eval_expr(ctx.clone(),if_branch.get_condition().clone())?;
                    let d=d.as_bool();
                    match d {
                        None => {
                            return Err(PipelineError::ExpectedDataType("bool".into()))
                        }
                        Some(d) => {
                            let mut l=Dynamic::Unit;
                            if d {
                                for i in if_branch.get_body() {
                                    l=self.eval_stmt_with_context(ctx.clone(), i.clone())?;
                                }
                                return Ok(l)
                            }
                        }
                    }
                }
               if let Some(else_body)=b.get_else_body(){
                   let mut l=Dynamic::Unit;
                   for i in else_body {
                       l=self.eval_stmt_with_context(ctx.clone(), i.clone())?;
                   }
                   return Ok(l)
               }

            }
            Stmt::ArrayAssign(s,i,v,_)=>{
                let scope=PipelineEngine::context_with_scope(&ctx);
                let a=PipelineEngine::context_with_dynamic(&ctx,s.clone()).unwrap();
                let mut a=a.as_array().unwrap();
                let i=self.eval_expr(ctx.clone(),*i)?;
                let i=i.as_integer().unwrap();
                let v=self.eval_expr(ctx,*v)?;
                a[i as usize]=v;
                scope.write().unwrap().set(s.as_str(),Dynamic::Array(a));

            }
            Stmt::While(b,blocks,_)=>{
                let d=self.eval_expr(ctx.clone(),*b.clone())?;
                let d=d.as_bool();
                return match d {
                    None => {
                        Err(PipelineError::ExpectedDataType("bool".into()))
                    }
                    Some(d) => {
                        let mut condition=d;
                        while condition {
                            for i in &*blocks {
                                self.eval_stmt_with_context(ctx.clone(), i.clone())?;
                            }
                             let d0=self.eval_expr(ctx.clone(),*b.clone())?;
                            condition=d0.as_bool().unwrap();
                        }
                        Ok(Dynamic::Unit)
                    }
                }
            }
            Stmt::Noop => {}
        }
        Ok(Dynamic::Unit)
    }
    pub  fn eval_let_stmt(&mut self, ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>, l:Box<(String,Expr)>)->PipelineResult<()>{
        let scope=PipelineEngine::context_with_scope(&ctx);
        let d=self.eval_expr(ctx,l.1)?;
        scope.write().unwrap().set(l.0.as_str(),d);
        Ok(())
    }

    pub  fn eval_expr(&mut self,ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>,expr:Expr)->PipelineResult<Dynamic>{
        match expr.clone() {
            Expr::FnCall(_, _)=>{
                let mut ptr=expr.dynamic().as_fn_ptr().unwrap();
                let mut e=PipelineEngine::default();
                e.set_interpreter(self);
                return ptr.call(&mut e,ctx);
            }
            Expr::Variable(i,_)=>{
                let d=PipelineEngine::context_with_dynamic(&ctx,i.clone());
                match d {
                    None => {
                        Err(PipelineError::VariableUndefined(i))
                    }
                    Some(d) => {Ok(d.clone())}
                }

            }
            Expr::Array(v,_)=>{
                let mut dv=vec![];
                for e in v{
                    let d=self.eval_expr(ctx.clone(), e)?;
                    dv.push(d)
                }
                Ok(Dynamic::Array(dv))
            }
            Expr::Index(s,e,_)=>{
                let d=PipelineEngine::context_with_dynamic(&ctx,s.clone());
                match d {
                    None => {
                        Err(PipelineError::VariableUndefined(s))
                    }
                    Some(d) => {
                        let a=d.as_array().unwrap();
                        let index=self.eval_expr(ctx,*e)?;
                        let index=index.as_integer().unwrap();
                        Ok(a[index as usize].clone())
                    }
                }
            }
            Expr::BinaryExpr(op,l,r,_)=>{
                match op {
                    Op::Plus => {
                        let l_r=self.eval_expr(ctx.clone(),*l)?;
                        let r_r=self.eval_expr(ctx.clone(),*r)?;
                        return Ok(l_r+r_r)
                    }
                    Op::Minus => {
                        let l_r=self.eval_expr(ctx.clone(),*l)?;
                        let r_r=self.eval_expr(ctx.clone(),*r)?;
                        return Ok(l_r-r_r)
                    }
                    Op::Mul=>{
                        let l_r=self.eval_expr(ctx.clone(),*l)?;
                        let r_r=self.eval_expr(ctx.clone(),*r)?;
                        return Ok(l_r*r_r)
                    }
                    Op::Greater=>{
                        let l_r=self.eval_expr(ctx.clone(),*l)?;
                        let r_r=self.eval_expr(ctx.clone(),*r)?;
                        return Ok((l_r>r_r).into())
                    }
                    Op::Less=>{
                        let l_r=self.eval_expr(ctx.clone(),*l)?;
                        let r_r=self.eval_expr(ctx.clone(),*r)?;
                        return Ok((l_r<r_r).into())
                    }
                    Op::Equal=>{
                        let l_r=self.eval_expr(ctx.clone(),*l)?;
                        let r_r=self.eval_expr(ctx.clone(),*r)?;
                        return Ok((l_r==r_r).into())
                    }
                    Op::Div=>{
                        let l_r=self.eval_expr(ctx.clone(),*l)?;
                        let r_r=self.eval_expr(ctx.clone(),*r)?;
                        return Ok(l_r/r_r)
                    }
                    Op::Mod=>{
                        let l_r=self.eval_expr(ctx.clone(),*l)?;
                        let r_r=self.eval_expr(ctx.clone(),*r)?;
                        return Ok(l_r%r_r)
                    }
                }
            }
            _=>Ok(expr.dynamic())
        }
    }
    pub fn eval_fn_call_expr(&mut self,f:FnCallExpr)->PipelineResult<Dynamic>{
        let c=Arc::new(RwLock::new(EmptyContext::new()));
       self.eval_fn_call_expr_with_context(c,f)
    }
    pub  fn eval_fn_call_expr_with_context(&mut self, ctx: Arc<RwLock<dyn Context<PipelineContextValue>>>, f:FnCallExpr) ->PipelineResult<Dynamic>{
        let mut v=vec![];
        for e in &f.args{
            let d=self.eval_expr(ctx.clone(),e.clone())?;
            if d.is_fn_ptr(){
                let mut ptr=d.as_fn_ptr().unwrap();
                if ptr.is_defer(){
                    v.push(d);
                    continue
                }
                let mut e=PipelineEngine::new_raw();
                e.set_interpreter(self);
                let d=ptr.call(&mut e,ctx.clone()).unwrap();
                v.push(d);
                continue
            }
            v.push(d);
        }
        let ctx=PipelineEngine::with_value(ctx,"$shared_module",PipelineContextValue::SharedModule(self.main_module.clone()));
        let mut r=None;
        if f.name.contains("::"){
            let mut l=f.name.split("::");
            let module_name=l.next().unwrap();
            let m=self.modules.get(module_name);
            match m {
                None => {
                    return Err(PipelineError::UnknownModule(module_name.into()))
                }
                Some(module) => {
                    let function_name=l.next().unwrap();
                    r=module.get_function(function_name);
                }
            }
        }else{
            r=self.main_module.read().unwrap().get_function(f.name.clone());
        }
        return match r {
            None => { Err(PipelineError::FunctionUndefined(f.name)) }
            Some(f) => { f.call(ctx, v) }
        }
    }
}
