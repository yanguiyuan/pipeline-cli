use std::collections::HashMap;
use std::sync::{Arc, RwLock, Weak};
use crate::context::{Context, EmptyContext};
use crate::context::PipelineContextValue;
use crate::engine::{PipelineEngine};
use crate::error::{PipelineError, PipelineResult};
use crate::module::{ Module};
use crate::v1::expr::{Expr, FnCallExpr, Op};
use crate::v1::stmt::Stmt;
use crate::v1::types::{Dynamic, Struct, Value};

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
    pub fn merge_into_main_module(&mut self,module_name: impl AsRef<str>)->PipelineResult<()>{
        let mut target=self.modules.get(module_name.as_ref());
        match target {
            None => {
                return Err(PipelineError::UnknownModule(module_name.as_ref().into()));
            }
            Some(target) => {self.main_module.write().unwrap().merge(target);}
        }

        Ok(())
    }
    pub fn get_mut_module(&mut self,name:impl Into<String>)->Option<&mut Module>{
        let m=self.modules.get_mut(name.into().as_str());
        return m
    }
    pub  fn eval_stmt(&mut self,stmt:Stmt)->PipelineResult<Value>{
        let ctx=PipelineEngine::background();
        return self.eval_stmt_with_context(ctx,stmt)
    }

    pub fn eval_stmt_with_context(&mut self, ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>, stmt:Stmt) ->PipelineResult<Value>{
        match stmt {
            Stmt::FnCall(fc, pos) => {
                let ctx=PipelineEngine::with_value(ctx,"$pos",pos.into());
                self.eval_fn_call_expr_with_context(ctx,*fc)?;
            }
            Stmt::Import(s,_)=>{
                self.merge_into_main_module(s)?;
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
                    let d=d.as_dynamic().as_bool();
                    match d {
                        None => {
                            return Err(PipelineError::ExpectedType("bool".into()))
                        }
                        Some(d) => {
                            let mut l=None;
                            if d {
                                for i in if_branch.get_body() {
                                    let t=self.eval_stmt_with_context(ctx.clone(), i.clone())?;
                                    l=Some(t)
                                }
                                return Ok(l.unwrap())
                            }
                        }
                    }
                }
               if let Some(else_body)=b.get_else_body(){
                   let mut l=None;
                   for i in else_body {
                       l=Some(self.eval_stmt_with_context(ctx.clone(), i.clone())?);
                   }
                   return Ok(l.unwrap())
               }

            }
            Stmt::ArrayAssign(s,i,v,_)=>{
                let scope=PipelineEngine::context_with_scope(&ctx);
                let  a=PipelineEngine::context_with_dynamic(&ctx,s.clone()).unwrap();
                let mut a=a.get_mut_arc();
                let mut a=a.write().unwrap();
                let mut a=a.as_mut_array().unwrap();
                let i=self.eval_expr(ctx.clone(),*i)?;
                let i=i.as_dynamic().as_integer().unwrap();
                let v=self.eval_expr(ctx,*v)?;
                a[i as usize]=v.as_weak().into()
            }
            Stmt::While(b,blocks,_)=>{
                let d=self.eval_expr(ctx.clone(),*b.clone())?;
                let d=d.as_dynamic().as_bool();
                return match d {
                    None => {
                        Err(PipelineError::ExpectedType("bool".into()))
                    }
                    Some(d) => {
                        let mut condition=d;
                        while condition {
                            for i in &*blocks {
                                self.eval_stmt_with_context(ctx.clone(), i.clone())?;
                            }
                             let d0=self.eval_expr(ctx.clone(),*b.clone())?;
                            condition=d0.as_dynamic().as_bool().unwrap();
                        }
                        Ok(().into())
                    }
                }
            }
            Stmt::Noop => {}
        }
        Ok(().into())
    }
    pub  fn eval_let_stmt(&mut self, ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>, l:Box<(String,Expr)>)->PipelineResult<()>{
        let scope=PipelineEngine::context_with_scope(&ctx);
        let d=self.eval_expr(ctx,l.1)?;
        if !d.is_mutable(){
            panic!("必须是mutable")
        }
        scope.write().unwrap().set(l.0.as_str(),d);
        Ok(())
    }

    pub  fn eval_expr(&mut self,ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>,expr:Expr)->PipelineResult<Value>{
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
                    Some(d) => {Ok(d)}
                }

            }
            Expr::Array(v,_)=>{
                let mut dv=vec![];
                for e in v{
                    let d=self.eval_expr(ctx.clone(), e)?;
                    dv.push(d.as_weak().into())
                }
                Ok(Value::Mutable(Arc::new(RwLock::new(Dynamic::Array(dv)))))
            }
            Expr::Map(v,_)=>{
                let mut dv=HashMap::new();
                for e in v{
                    let key=self.eval_expr(ctx.clone(), e.0)?;
                    let value=self.eval_expr(ctx.clone(), e.1)?;
                    dv.insert(key.as_dynamic().clone(),value);
                }
                Ok(Value::Mutable(Arc::new(RwLock::new(Dynamic::Map(dv)))))
            }
            Expr::Index(s,e,_)=>{
                let d=PipelineEngine::context_with_dynamic(&ctx,s.clone());
                match d {
                    None => {
                        Err(PipelineError::VariableUndefined(s))
                    }
                    Some(d) => {
                        match d.as_dynamic() {
                            Dynamic::Array(a) => {
                                let index=self.eval_expr(ctx,*e)?;
                                let index=index.as_dynamic().as_integer().unwrap();
                                Ok(a[index as usize].clone())
                            }
                            Dynamic::Map(m) => {
                                let index=self.eval_expr(ctx,*e)?;
                                let index=index.as_dynamic();
                                Ok(m[&index].clone())
                            }
                            Dynamic::String(s)=>{
                                let index=self.eval_expr(ctx,*e)?;
                                let index=index.as_dynamic().as_integer().unwrap();
                                let r=String::from(s.chars().nth(index as usize).unwrap());
                                Ok(r.into())
                            }
                            t=>{
                                return Err(PipelineError::UndefinedOperation(format!("index [] to {}",t.type_name())))
                            }
                        }

                    }
                }
            }
            Expr::BinaryExpr(op,l,r,_)=>{
                match op {
                    Op::Plus => {
                        let l_r=self.eval_expr(ctx.clone(),*l)?;
                        let l_r=l_r.as_dynamic();
                        let r_r=self.eval_expr(ctx.clone(),*r)?;
                        let r_r=r_r.as_dynamic();
                        return Ok((l_r+r_r).into())
                    }
                    Op::Minus => {
                        let l_r=self.eval_expr(ctx.clone(),*l)?;
                        let l_r=l_r.as_dynamic();
                        let r_r=self.eval_expr(ctx.clone(),*r)?;
                        let r_r=r_r.as_dynamic();
                        return Ok((l_r-r_r).into())
                    }
                    Op::Mul=>{
                        let l_r=self.eval_expr(ctx.clone(),*l)?;
                        let l_r=l_r.as_dynamic();
                        let r_r=self.eval_expr(ctx.clone(),*r)?;
                        let r_r=r_r.as_dynamic();
                        return Ok((l_r*r_r).into())
                    }
                    Op::Greater=>{
                        let l_r=self.eval_expr(ctx.clone(),*l)?;
                        let l_r=l_r.as_dynamic();
                        let r_r=self.eval_expr(ctx.clone(),*r)?;
                        let r_r=r_r.as_dynamic();
                        return Ok((l_r>r_r).into())
                    }
                    Op::Less=>{
                        let l_r=self.eval_expr(ctx.clone(),*l)?;
                        let l_r=l_r.as_dynamic();
                        let r_r=self.eval_expr(ctx.clone(),*r)?;
                        let r_r=r_r.as_dynamic();
                        return Ok((l_r<r_r).into())
                    }
                    Op::Equal=>{
                        let l_r=self.eval_expr(ctx.clone(),*l)?;
                        let l_r=l_r.as_dynamic();
                        let r_r=self.eval_expr(ctx.clone(),*r)?;
                        let r_r=r_r.as_dynamic();
                        return Ok((l_r==r_r).into())
                    }
                    Op::NotEqual=>{
                        let l_r=self.eval_expr(ctx.clone(),*l)?;
                        let l_r=l_r.as_dynamic();
                        let r_r=self.eval_expr(ctx.clone(),*r)?;
                        let r_r=r_r.as_dynamic();
                        return Ok((l_r!=r_r).into())
                    }
                    Op::Div=>{
                        let l_r=self.eval_expr(ctx.clone(),*l)?;
                        let l_r=l_r.as_dynamic();
                        let r_r=self.eval_expr(ctx.clone(),*r)?;
                        let r_r=r_r.as_dynamic();
                        return Ok((l_r/r_r).into())
                    }
                    Op::Mod=>{
                        let l_r=self.eval_expr(ctx.clone(),*l)?;
                        let l_r=l_r.as_dynamic();
                        let r_r=self.eval_expr(ctx.clone(),*r)?;
                        let r_r=r_r.as_dynamic();
                        return Ok((l_r%r_r).into())
                    }
                }
            }
            Expr::Struct(e,_)=>{
                let mut props=HashMap::new();
                for (k,i) in e.get_props(){
                    let v=self.eval_expr(ctx.clone(),i.clone())?;
                    if v.is_mutable(){
                        panic!("不能持有其所有权")
                    }
                    props.insert(k.clone(),v);
                }
                Ok(
                    Value::Mutable(
                        Arc::new(
                            RwLock::new(
                                Dynamic::Struct(
                                    Box::new(
                                        Struct::new(e.get_name().into(),props)
                                    )
                                )
                            )
                        )
                    )
                )
            }
            Expr::MemberAccess(father,prop,_)=>{
                let obj=self.eval_expr(ctx,*father)?;
                let obj=obj.as_dynamic().as_struct().unwrap();
                let r=obj.get_prop(&prop).unwrap();
                return Ok(r)
            }
            _=>Ok(expr.dynamic().into())
        }
    }
    pub fn eval_fn_call_expr(&mut self,f:FnCallExpr)->PipelineResult<Value>{
        let c=Arc::new(RwLock::new(EmptyContext::new()));
       self.eval_fn_call_expr_with_context(c,f)
    }
    pub  fn eval_fn_call_expr_with_context(&mut self, ctx: Arc<RwLock<dyn Context<PipelineContextValue>>>, f:FnCallExpr) ->PipelineResult<Value>{
        let mut v=vec![];
        for e in &f.args{
            let d=self.eval_expr(ctx.clone(),e.clone())?;
            if d.as_dynamic().is_fn_ptr(){
                let mut ptr=d.as_dynamic().as_fn_ptr().unwrap();
                if ptr.is_defer(){
                    v.push(d);
                    continue
                }
                let mut e=PipelineEngine::new_raw();
                e.set_interpreter(self);
                let d=ptr.call(&mut e,ctx.clone()).unwrap();
                v.push(d);
                continue
            }else if d.as_dynamic().is_variable(){
                let d=d.as_dynamic().as_variable().unwrap();
                let r=PipelineEngine::context_with_dynamic(&ctx,d.as_str()).unwrap();
                v.push(r);
                continue
            }
            v.push(d);
        }
        let ctx=PipelineEngine::with_value(ctx,"$shared_module",PipelineContextValue::SharedModule(self.main_module.clone()));
        let ctx=PipelineEngine::with_value(ctx,"$modules",PipelineContextValue::Modules(Arc::new(RwLock::new(self.modules.clone()))));
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
