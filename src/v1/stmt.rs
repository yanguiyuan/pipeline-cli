use crate::v1::expr::{Expr, FnCallExpr};
use crate::v1::position::{NONE, Position};

#[derive(Debug,Clone)]
pub enum Stmt{
    FnCall(Box<FnCallExpr>,Position),
    Let(Box<(String,Expr)>,Position),
    Return(Box<Expr>,Position),
    If(Box<Expr>,Box<Vec<Stmt>>,Position),
    While(Box<Expr>,Box<Vec<Stmt>>,Position),
    ArrayAssign(String,Box<Expr>,Box<Expr>,Position),
    Noop
}

impl Stmt{
    pub fn is_noop(&self)->bool{
        return match self {
            Stmt::Noop => true,
            _=>false
        }
    }
    pub fn position(&self)->Position{
        match self {
            Stmt::FnCall(_, pos) => {
                pos.clone()
            }
            Stmt::Let(_,pos)=>{
                pos.clone()
            }
            Stmt::Return(_,pos)=>{
                pos.clone()
            }
            Stmt::If(_,_,pos)=>{
                pos.clone()
            }
            Stmt::While(_,_,pos)=>{
                pos.clone()
            }
            Stmt::ArrayAssign(_,_,_,pos)=>{
                pos.clone()
            }
            Stmt::Noop => {
                NONE.clone()
            }
        }
    }
}