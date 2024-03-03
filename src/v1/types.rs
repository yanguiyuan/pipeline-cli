use std::fmt::{Display, Formatter};
use std::ptr::write;
use std::sync::{Arc, RwLock};
use crate::context::{Context, PipelineContextValue};
use crate::engine::{PipelineEngine, PipelineResult};
use crate::v1::expr::{Expr, FnCallExpr};
use crate::v1::parser::FnDef;
#[derive(Debug,Clone)]
pub enum Dynamic{
    Unit,
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Variable(String),
    FnPtr(Box<FnPtr>)
}
#[derive(Debug,Clone)]
pub struct FnPtr{
    pub name:String,
    pub params:Vec<Expr>,
    is_defer:bool,
    pub fn_def:Option<FnDef>
}

impl FnPtr {
    pub fn new(name:&str)->Self{
        Self{
            name:name.into(),
            params:vec![],
            fn_def:None,
            is_defer:false
        }
    }
    pub fn is_defer(&self)->bool{
        return self.is_defer
    }
    pub fn set_defer(&mut self,defer:bool){
        self.is_defer=defer
    }
    pub fn set_params(&mut self,params:&Vec<Expr>){
        self.params=params.clone();
    }
    pub fn set_fn_def(&mut self,fn_def:&FnDef){
        self.fn_def=Some(fn_def.clone())
    }
    pub async fn call(&mut self, engine:&mut PipelineEngine, ctx:Arc<tokio::sync::RwLock<dyn Context<PipelineContextValue>>>)->PipelineResult<Dynamic>{
        let fn_def= self.fn_def.clone();
        match fn_def {
            None => {
                let expr=FnCallExpr{ name: self.name.clone(), args: self.params.clone() };
                engine.eval_fn_call_expr(ctx,expr).await
            },
            Some(f) => {
                let blocks=f.body;
                engine.eval_stmt_blocks_with_context(ctx,blocks).await
            }
        }

    }
}
impl From<&str> for Dynamic{
    fn from(value: &str) -> Self {
        Dynamic::String(String::from(value))
    }
}
impl From<String> for Dynamic{
    fn from(value: String) -> Self {
        Dynamic::String(value)
    }
}
impl From<i64> for Dynamic{
    fn from(value: i64) -> Self {
        Dynamic::Integer(value)
    }
}
impl Display for Dynamic {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Dynamic::Unit => {write!(f,"Unit")}
            Dynamic::Integer(i) => {write!(f,"{i}")}
            Dynamic::Float(f0) => {write!(f,"{f0}")}
            Dynamic::String(s) => {write!(f,"{s}")}
            Dynamic::Boolean(b) => {write!(f,"{b}")}
            Dynamic::Variable(a) => {write!(f,"Variable({a})")}
            Dynamic::FnPtr(p)=>write!(f,"function {:}",p.name)
        }
    }
}
impl Dynamic{
    pub fn is_variable(&self)->bool{
        match self {
            Dynamic::Variable(_) => true,
            _=>false,
        }
    }
    pub fn is_fn_ptr(&self)->bool{
        match self {
            Dynamic::FnPtr(_) => true,
            _=>false,
        }
    }
    pub fn is_integer(&self)->bool{
        match self {
            Dynamic::Integer(_) => true,
            _=>false,
        }
    }
    pub fn is_float(&self)->bool{
        match self {
            Dynamic::Float(_) => true,
            _=>false,
        }
    }
    pub fn as_variable(&self)->Option<String>{
        match self {
            Dynamic::Variable(s)=>Some(s.clone()),
            _=>None
        }
    }
    pub fn as_string(&self)->Option<String>{
        match self {
            Dynamic::String(s)=>Some(s.clone()),
            _=>None
        }
    }
    pub fn as_fn_ptr(&self)->Option<Box<FnPtr>>{
        match self {
            Dynamic::FnPtr(f)=>Some(f.clone()),
            _=>None
        }
    }
    pub fn as_integer(&self)->Option<i64>{
        match self {
            Dynamic::Integer(i)=>Some(i.clone()),
            _=>None
        }
    }
    pub fn as_float(&self)->Option<f64>{
        match self {
            Dynamic::Float(i)=>Some(i.clone()),
            _=>None
        }
    }
    pub fn convert_float(&self)->Option<f64>{
        match self {
            Dynamic::Integer(i)=>Some(i.clone() as f64),
            Dynamic::Float(i)=>Some(i.clone()),
            _=>None
        }
    }
}