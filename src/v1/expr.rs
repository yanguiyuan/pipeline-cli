use std::any::Any;
use crate::v1::parser::FnDef;
use crate::v1::position::Position;
use crate::v1::types::Dynamic;

#[derive(Debug,Clone)]
pub enum Expr{
    StringConstant(String, Position),
    IntConstant(i64,Position),
    FloatConstant(f64,Position),
    FnClosure(FnClosureExpr,Position),
    FnCall(FnCallExpr,Position),
    Variable(String,Position)
}
#[derive(Debug,Clone)]
pub struct FnCallExpr{
    pub name:String,
    pub args:Vec<Expr>
}
#[derive(Debug,Clone)]
pub struct FnClosureExpr{
    pub(crate) def:FnDef
}
impl Expr {
    pub fn is_fn_call(&self)->bool{
        match self {
            Expr::FnCall(_,_)=>true,
            _=>false
        }
    }
    pub fn position(&self)->Position{
        match self  {
            Expr::StringConstant(_, pos) => {pos.clone()}
            Expr::IntConstant(_, pos) => {pos.clone()}
            Expr::FloatConstant(_, pos) => {pos.clone()}
            Expr::Variable(_, pos) => {pos.clone()}
            Expr::FnClosure(_,pos)=>{pos.clone()}
            Expr::FnCall(_,pos)=>{pos.clone()}
        }
    }
    pub fn any(&self)-> Box<dyn Any> {
        match self  {
            Expr::StringConstant(s, _) => {
                Box::new(s.clone())
            }
            Expr::IntConstant(i, _) => Box::new(i.clone()),
            Expr::FloatConstant(f, _) => Box::new(f.clone()),
            Expr::Variable(v,_)=>Box::new(Variable{name:v.clone()}),
            _=>Box::new(()),
        }
    }
    pub fn dynamic(&self)->Dynamic{
        match self {
            Expr::StringConstant(std, _) => Dynamic::String(std.clone()),
            Expr::IntConstant(i, _) => Dynamic::Integer(i.clone()),
            Expr::FloatConstant(f, _) => Dynamic::Float(f.clone()),
            Expr::Variable(s, _) => Dynamic::Variable(s.clone()),
            Expr::FnClosure(f,_)=>{
                let mut ptr=crate::v1::types::FnPtr::new(f.def.name.as_str());
                ptr.set_fn_def(&f.def);
                Dynamic::FnPtr(Box::new(ptr))
            },
            Expr::FnCall(f,_)=>{
                let mut ptr=crate::v1::types::FnPtr::new(f.name.as_str());
                ptr.set_params(&f.args);
                Dynamic::FnPtr(Box::new(ptr))
            }
            _=>Dynamic::Unit
        }
    }
}
pub struct Variable {
    pub name:String
}