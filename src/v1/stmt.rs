use crate::v1::expr::{Expr, FnCallExpr};
use crate::v1::position::{NONE, Position};

#[derive(Debug,Clone)]
pub enum Stmt{
    FnCall(Box<FnCallExpr>,Position),
    Let(Box<(String,Expr)>,Position),
    Return(Box<Expr>,Position),
    If(Box<IfStmt>,Position),
    While(Box<Expr>,Box<Vec<Stmt>>,Position),
    ArrayAssign(String,Box<Expr>,Box<Expr>,Position),
    Import(String,Position),
    Noop
}
#[derive(Debug,Clone)]
pub struct IfStmt{
    branches:Vec<IfBranchStmt>,
    else_body:Option<Vec<Stmt>>
}

impl IfStmt {
    pub fn new(branches:Vec<IfBranchStmt>,else_body:Option<Vec<Stmt>>)->Self{
        Self{
            branches,else_body
        }
    }
    pub fn get_branches(&self)->&Vec<IfBranchStmt>{
        &self.branches
    }
    pub fn get_else_body(&self)->Option<Vec<Stmt>>{
        self.else_body.clone()
    }
}
#[derive(Debug,Clone)]
pub struct IfBranchStmt{
    condition:Expr,
    body:Vec<Stmt>
}

impl IfBranchStmt {
    pub fn new(condition:Expr,body:Vec<Stmt>)->Self{
        Self{condition,body}
    }
    pub fn get_condition(&self)->&Expr{
        &self.condition
    }
    pub fn get_body(&self)->&Vec<Stmt>{
        &self.body
    }
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
            Stmt::If(_,pos)=>{
                pos.clone()
            }
            Stmt::While(_,_,pos)=>{
                pos.clone()
            }
            Stmt::ArrayAssign(_,_,_,pos)=>{
                pos.clone()
            }
            Stmt::Import(_,pos)=>{
                pos.clone()
            }
            Stmt::Noop => {
                NONE.clone()
            }
        }
    }
}