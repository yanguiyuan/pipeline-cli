use crate::v1::expr::{Expr, FnCallExpr};
use crate::v1::position::{NONE, Position};

#[derive(Debug,Clone)]
pub enum Stmt{
    FnCall(Box<FnCallExpr>,Position),
    Let(Box<(String,Expr)>,Position),
    Assign(Box<(Expr,Expr)>,Position),
    Return(Box<Expr>,Position),
    If(Box<IfStmt>,Position),
    While(Box<Expr>,Box<Vec<Stmt>>,Position),
    ForIn(String,Box<Expr>,Box<Vec<Stmt>>,Position),
    /// 第一个Expr表示的是获取Array或者Map的表达式
    /// 第二个Expr表示的是获取索引的表达式
    /// 第三个Expr表示的是对索引处的赋值
    IndexAssign(Box<Expr>,Box<Expr>,Box<Expr>,Position),
    Break(Position),
    Continue(Position),
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
            Stmt::Continue(pos)=>pos.clone(),
            Stmt::FnCall(_, pos) => {
                pos.clone()
            }
            Stmt::ForIn(_,_,_,pos)=>pos.clone(),
            Stmt::Let(_,pos)=>{
                pos.clone()
            }
            Stmt::Assign(_,pos)=>pos.clone(),
            Stmt::Return(_,pos)=>{
                pos.clone()
            }
            Stmt::If(_,pos)=>{
                pos.clone()
            }
            Stmt::While(_,_,pos)=>{
                pos.clone()
            }
            Stmt::IndexAssign(_,_,_,pos)=>{
                pos.clone()
            }
            Stmt::Import(_,pos)=>{
                pos.clone()
            }
            Stmt::Noop => {
                NONE.clone()
            }
            Stmt::Break(pos)=>pos.clone()
        }
    }
}