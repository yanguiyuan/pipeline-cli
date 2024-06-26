use std::any::Any;
use std::collections::HashMap;
use crate::v1::parser::FnDef;
use crate::v1::position::Position;
use crate::v1::types::{Dynamic};

#[derive(Debug,Clone)]
pub enum Expr{
    StringConstant(String, Position),
    IntConstant(i64,Position),
    FloatConstant(f64,Position),
    FnClosure(FnClosureExpr,Position),
    FnCall(FnCallExpr,Position),
    Variable(String,Position),
    BinaryExpr(Op,Box<Expr>,Box<Expr>,Position),
    Array(Vec<Expr>,Position),
    Map(Vec<(Expr,Expr)>,Position),
    Index(Box<Expr>,Box<Expr>,Position),
    Struct(StructExpr,Position),
    /// a=Person::new()
    /// a.name  -> MemberAccess
    MemberAccess(Box<Expr>,String,Position),
    None(Position)
}
#[derive(Debug,Clone)]
pub struct StructExpr{
    name:String,
    props:HashMap<String,Expr>
}

impl StructExpr {
    pub fn new(name:String,props:HashMap<String,Expr>)->Self{
        Self{name,props}
    }
    pub fn get_name(&self)->&str{
        &self.name
    }
    pub fn get_props(&self)->&HashMap<String,Expr>{
        &self.props
    }
}
#[derive(Debug,Clone)]
pub enum Op{
    Plus,
    Minus,
    Mul,
    Div,
    Mod,
    Greater,
    Less,
    Equal,
    NotEqual

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
    pub fn try_get_variable_name(&self)->Option<String>{
        match self {
            Expr::Variable(s,_)=>Some(s.clone()),
            _=>None
        }
    }
    pub fn try_get_member_access(&self)->Option<(Box<Expr>,String)>{
        match self {
            Expr::MemberAccess(b,name,_)=>Some((b.clone(),name.clone())),
            _=>None
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
            Expr::BinaryExpr(_,_,_,pos)=>{pos.clone()}
            Expr::Array(_,pos)=>{pos.clone()}
            Expr::Index(_,_,pos)=>{pos.clone()}
            Expr::Map(_,pos)=>{pos.clone()}
            Expr::None(pos)=>{pos.clone()}
            Expr::Struct(_,pos)=>{pos.clone()}
            Expr::MemberAccess(_,_,pos)=>pos.clone()
        }
    }

    // pub fn any(&self)-> Box<dyn Any> {
    //     match self  {
    //         Expr::StringConstant(s, _) => {
    //             Box::new(s.clone())
    //         }
    //         Expr::IntConstant(i, _) => Box::new(i.clone()),
    //         Expr::FloatConstant(f, _) => Box::new(f.clone()),
    //         Expr::Variable(v,_)=>Box::new(Variable{name:v.clone()}),
    //         _=>Box::new(()),
    //     }
    // }
    pub fn dynamic(&self)->Dynamic{
        match self {
            Expr::StringConstant(std, _) => Dynamic::String(std.clone()),
            Expr::IntConstant(i, _) => Dynamic::Integer(i.clone()),
            Expr::FloatConstant(f, _) => Dynamic::Float(f.clone()),
            // Expr::Variable(s, _) => Dynamic::Variable(s.clone()),
            Expr::FnClosure(f,_)=>{
                let mut ptr=crate::v1::types::FnPtr::new(f.def.name.as_str());
                ptr.set_fn_def(&f.def);
                ptr.set_defer(true);
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