use crate::v1::stmt::Stmt;

#[derive(Debug,Clone)]
pub struct AST{
    pub body:Vec<Stmt>
}