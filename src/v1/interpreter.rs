use std::collections::HashMap;
use std::sync::{Arc, RwLock, RwLockWriteGuard, Weak};
use crate::context::{Context, EmptyContext};
use crate::context::PipelineContextValue;
use crate::engine::{PipelineEngine};
use crate::error::{PipelineError, PipelineResult};
use crate::module::{Function, Module};
use crate::v1::expr::{Expr, FnCallExpr, Op};
use crate::v1::stmt::Stmt;
use crate::v1::types::{Dynamic, SignalType, Struct, Value};

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
            Stmt::Break(_)=>{
                return Ok(Value::Signal(SignalType::Break))
            }
            Stmt::Continue(_)=>{
                return Ok(Value::Signal(SignalType::Continue))
            }
            Stmt::Assign(e,_)=>{
                let target=self.eval_expr(ctx.clone(),e.0)?;
                let value=self.eval_expr(ctx,e.1)?;
                if !target.can_mutable(){
                    panic!("it must be mutable")
                }
                let target=target.get_mut_arc();
                let mut target=target.write().unwrap();
                *target=value.as_dynamic();
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
                                if let None=l{
                                    return Ok(().into())
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
            Stmt::IndexAssign(target,i,v,_)=>{
                let i=self.eval_expr(ctx.clone(),*i)?;
                let v=self.eval_expr(ctx.clone(),*v)?;
                let target=self.eval_expr(ctx,*target)?;
                let target=target.get_mut_arc();
                let mut target=target.write().unwrap();
                if target.is_array(){
                    let a=target.as_mut_array().unwrap();
                    let index=i.as_dynamic().as_integer().unwrap();
                    a[index as usize]=v;
                }else if target.is_map(){
                    let m=target.as_mut_map().unwrap();
                    let key=i.as_dynamic();
                    m.insert(key,v);
                }else{
                    panic!("{} cannot support index assign",target.type_name())
                }
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
                        'outer:while condition {
                            'inner:for i in &*blocks {
                                let r=self.eval_stmt_with_context(ctx.clone(), i.clone())?;
                                if let Value::Signal(s)=r{
                                    match s {
                                        SignalType::Break => {
                                            break 'outer
                                        }
                                        SignalType::Continue => {
                                            break 'inner
                                        }
                                    }
                                }
                            }
                             let d0=self.eval_expr(ctx.clone(),*b.clone())?;
                            condition=d0.as_dynamic().as_bool().unwrap();
                        }
                        Ok(().into())
                    }
                }
            }
            Stmt::ForIn(one,other ,target, blocks, ..)=> {
                let target = self.eval_expr(ctx.clone(), *target.clone())?;
                let target = target.as_dynamic().as_array().unwrap();
                let mut count=0;
                'outer:for i in target{
                    let scope=PipelineEngine::context_with_scope(&ctx);
                    let mut scope=scope.write().unwrap();
                    match other.clone() {
                        None => {
                            scope.set(one.as_str(),i);
                        }
                        Some(s) => {
                            scope.set(one.as_str(),count.into());
                            count+=1;
                            scope.set(s.as_str(),i);
                        }
                    }
                    drop(scope);
                    'inner: for i in &*blocks {
                        let r = self.eval_stmt_with_context(ctx.clone(), i.clone())?;
                        if let Value::Signal(s) = r {
                            match s {
                                SignalType::Break => {
                                    break 'outer
                                }
                                SignalType::Continue => {
                                    break 'inner
                                }
                            }
                        }
                    }
                }
            }
            Stmt::Noop => {}
        }
        Ok(().into())
    }
    pub  fn eval_let_stmt(&mut self, ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>, l:Box<(String,Expr)>)->PipelineResult<Value>{
        // let d=self.eval_expr(ctx.clone(),l.0)?;

        let mut d =self.eval_expr(ctx.clone(), l.1)?;
        if !d.is_mutable(){
            d=Value::Mutable(d.as_arc());
        }
        let scope=PipelineEngine::context_with_scope(&ctx);
        let mut scope=scope.write().unwrap();
        scope.set(l.0.as_str(),d);
        Ok(().into())
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
                    // if d.is_mutable(){
                    //     panic!("不能持有所有权")
                    // }
                    dv.push(d)
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
                let d=self.eval_expr(ctx.clone(),*s)?;
                let d=d.as_dynamic();
                match d {
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
                    let mut v=self.eval_expr(ctx.clone(),i.clone())?;
                    if v.is_immutable()||v.is_mutable(){
                        let scope=PipelineEngine::context_with_scope(&ctx);
                        let mut scope=scope.write().unwrap();
                        let  v0=Value::Mutable(v.as_arc());
                        v=Value::Refer(v0.as_weak());
                        scope.set(format!("{}.{}",e.get_name(),k).as_str(),v0)
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
                // println!("{:?}",obj);
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
        let fist_param_type=v[0].as_dynamic().type_name();
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
            let class_function_result=self.main_module.read().unwrap().get_class_function(&fist_param_type,f.name.as_str());
            match class_function_result {
                None => {
                    r=self.main_module.read().unwrap().get_function(f.name.clone());
                }
                Some(class_function) => {
                    let scope=PipelineEngine::context_with_scope(&ctx);
                    let mut scope=scope.write().unwrap();
                    scope.set("this",v[0].clone());
                    r=Some(class_function)
                }
            }
        }
        return match r {
            None => { Err(PipelineError::FunctionUndefined(f.name)) }
            Some(f) => { f.call(ctx, v) }
        }
    }
}
