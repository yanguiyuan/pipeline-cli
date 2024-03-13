use std::any::Any;

use crate::v1::lexer::{Lexer, TokenStream};
use crate::v1::stmt::Stmt;

use crate::v1::token::Token;
use crate::v1::ast::AST;
use crate::v1::expr::{Expr, FnCallExpr, FnClosureExpr, Op};
use crate::v1::expr::Expr::BinaryExpr;
use crate::v1::parser::ParserError::UnexpectedToken;
use crate::v1::position::{NONE, Position};

pub struct PipelineParser{
    token_stream: TokenStream,
    fn_lib:Vec<FnDef>
}
impl PipelineParser{
    pub fn new()->Self{
        Self{token_stream:TokenStream::new(),fn_lib:vec![]}
    }
    pub fn set_lexer(&mut self,lexer: Lexer){
        self.token_stream.set_lexer(lexer)
    }
    pub fn get_fn_lib(&self)->Vec<FnDef>{
        self.fn_lib.clone()
    }
    pub fn compile_from_token_stream(&mut self)->ParserResult<AST>{
        let stmts=self.parse_stmt_blocks()?;
        return Ok(AST{ body: stmts })
    }
    pub fn parse_stmt_blocks(&mut self)->ParserResult<Vec<Stmt>>{
        let mut v=vec![];
        loop {
            let stmt=self.parse_stmt()?;
            if stmt.is_noop(){
                break
            }
            v.push(stmt)
        }
        return Ok(v)
    }

    pub fn parse_stmt(&mut self)->ParserResult<Stmt>{
        loop {
            let token=self.token_stream.peek();
            return match token {
                None => {
                    Ok(Stmt::Noop)
                }
                Some((token0,pos)) => {
                    match token0 {
                        Token::Keyword(k)=>{
                            match k.as_str() {
                                "let"=>{
                                    return self.parse_let_stmt()
                                }
                                "fn"=>{
                                    let (fn_def,pos)=self.parse_fn_def()?;
                                    self.fn_lib.push(fn_def);
                                    continue
                                }
                                "return"=>{
                                    self.parse_return_stmt()
                                }
                                t=>Err(ParserError::UnusedKeyword(t.into()))
                            }
                        },
                        _=>return self.parse_expr_stmt()
                    }
                }
            }
        }

    }
    pub fn parse_return_stmt(&mut self)->ParserResult<Stmt>{
        let (ret,mut pos)=self.token_stream.next().unwrap();
        if let Token::Keyword(s)=ret.clone(){
            if s!="return"{
                return Err(ParserError::UnusedKeyword(s));
            }
            let expr=self.parse_expr()?;
            pos.add_span(expr.position().span);
            return Ok(Stmt::Return(Box::new(expr), pos))
        }
        return Err(ParserError::UnexpectedToken(ret));
    }
    pub fn parse_let_stmt(&mut self)->ParserResult<Stmt>{
        let (token,mut pos)=self.token_stream.next().unwrap();
        if let Token::Keyword(s)= token{
            let (token1,pos0)=self.token_stream.next().unwrap();
            if let Token::Identifier(ident)=token1{
                pos.add_span(pos0.span);
                self.parse_special_token(Token::Assign)?;
                pos.add_span(1);
                let expr=self.parse_expr()?;
                pos.add_span(expr.position().span);
                return Ok(Stmt::Let(Box::new((ident, expr)), pos));
            }
            return Err(UnexpectedToken(token1))
        }
        return Err(UnexpectedToken(token))
    }
    pub fn parse_fn_def(&mut self)->ParserResult<(FnDef,Position)>{
        let (next,mut pos)=self.token_stream.next().unwrap();
        match next {
            Token::Keyword(s) if s.as_str()=="fn"=>{
                pos.add_span(2);
                let (next1,pos1)=self.token_stream.next().unwrap();
                if let Token::Identifier(ident)=next1{
                    pos.add_span(pos1.span);
                    let (dec_args,pos2)=self.parse_fn_def_args()?;
                    pos.add_span(pos2.span);
                    self.parse_special_token(Token::ParenthesisLeft)?;
                    pos.add_span(1);
                    let stmts=self.parse_stmt_blocks()?;
                    for s in &stmts{
                        let pos_t=s.position();
                        pos.add_span(pos_t.span);
                    }
                    self.parse_special_token(Token::ParenthesisRight)?;
                    pos.add_span(1);
                    return Ok((FnDef::new(ident,dec_args,stmts),pos))
                }
                return Err(UnexpectedToken(next1))
            },
            _=>{
                Err(UnexpectedToken(next))
            }
        }
    }
    pub fn parse_fn_def_args(&mut self)->ParserResult<(Vec<VariableDeclaration>,Position)>{
       let start= self.parse_special_token(Token::BraceLeft)?;
        let mut v=vec![];
        let mut p=start.1;
        loop {
            let (token,pos)=self.token_stream.peek().unwrap();
            if Token::BraceRight==token{
               break
            }
            if Token::Comma==token {
                self.parse_special_token(Token::Comma)?;
                p.add_span(1);
                continue
            }
            let (dec,pos)=self.parse_variable_declaration()?;
            v.push(dec);
            p.add_span(pos.span)
        }
        self.parse_special_token(Token::BraceRight)?;
        p.add_span(1);
        return Ok((v,p))
    }
    pub fn parse_variable_declaration(&mut self)->ParserResult<(VariableDeclaration,Position)>{
        let (next,mut pos)=self.token_stream.next().unwrap();
        if let Token::Identifier(s)=next.clone(){
            self.parse_special_token(Token::Colon)?;
            let (next1,pos1)=self.token_stream.next().unwrap();
            if let Token::Identifier(s1)=next1{
                pos.add_span(1+pos1.span);
                return Ok((VariableDeclaration::new(s,s1),pos))
            }
            return Err(ParserError::UnexpectedToken(next1))
        }
        return Err(ParserError::UnexpectedToken(next))
    }

    pub fn parse_expr_stmt(&mut self)->ParserResult<Stmt>{
        let token=self.token_stream.peek();
        return Ok(match token {
            None => {
                Stmt::Noop
            }
            Some((token, pos)) => {
                match token {
                    Token::Identifier(s) => {
                        let (fn_call_expr, pos) = self.parse_fn_call_expr()?;
                        Stmt::FnCall(Box::new(fn_call_expr), pos)
                    }
                    _ => Stmt::Noop
                }
            }
        })
    }
    pub fn parse_fn_call_expr(&mut self) -> ParserResult<(FnCallExpr,Position)> {
        let  (token,mut pos)=self.token_stream.next().unwrap();
        match token {
            Token::Identifier(s)=>{
                let (mut args,args_pos)=self.parse_fn_call_args()?;
                pos.add_span(args_pos.span);
                if let Some((peek,mut pos1))=self.token_stream.peek(){
                    if Token::ParenthesisLeft==peek{
                        self.parse_special_token(Token::ParenthesisLeft).unwrap();
                        let blocks=self.parse_stmt_blocks()?;
                        for stmt in &blocks{
                            pos1.add_span(stmt.position().span)
                        }
                        let fn_def=FnDef::new("".to_string(),vec![],blocks);
                        pos1.add_span(1);
                        pos.add_span(pos1.span);
                        args.push(Expr::FnClosure(FnClosureExpr { def: fn_def },pos1));
                        self.parse_special_token(Token::ParenthesisRight).unwrap();
                    }
                }

                Ok((FnCallExpr{
                    name: s,
                    args
                },pos))
            }
            _=>Err(ParserError::UnexpectedToken(token))
        }
    }
    pub fn parse_fn_call_args(&mut self)->ParserResult<(Vec<Expr>,Position)>{
        self.parse_special_token(Token::BraceLeft)?;
        let mut v =vec![];
        let mut p=NONE.clone();
        p.add_span(1);
        loop {
            let expr=self.parse_expr()?;
            v.push(expr.clone());
            let expr_pos=expr.position();
            if p.is_none(){
                p.set_pos(expr_pos.pos);
            }
            p.add_span(expr_pos.span);

            let (token,pos)=self.token_stream.peek().unwrap();

            match token {
                Token::BraceRight => {
                    p.add_span(1);
                    self.token_stream.next();
                    break
                }
                Token::Comma => {
                    self.token_stream.next();
                }
                _=>return Err(ParserError::UnexpectedToken(token))
            }
        }
        return Ok((v,p))
    }
    pub fn parse_expr(&mut self)->ParserResult<Expr>{
        let (token,mut pos)=self.token_stream.next().unwrap();
        match token {
            Token::String(s) => {
                Ok(Expr::StringConstant(s,pos))
            }
            Token::Int(i) => {
                let (peek,pos0)=self.token_stream.peek().unwrap_or((Token::EOF,NONE.clone()));
                match peek {
                    Token::Plus => {
                        self.token_stream.next();
                        let right=self.parse_expr()?;
                        let left=Box::new(Expr::IntConstant(i,pos.clone()));
                        pos.add_span(1+right.position().span);
                        Ok(BinaryExpr(Op::Plus,left,Box::new(right),pos))
                    }
                    Token::Mul=>{
                        self.token_stream.next();
                        let right=self.parse_expr()?;
                        let left=Box::new(Expr::IntConstant(i,pos.clone()));
                        pos.add_span(1+right.position().span);
                        Ok(BinaryExpr(Op::Mul,left,Box::new(right),pos))
                    }
                    _=>Ok(Expr::IntConstant(i,pos))
                }

            }
            Token::Float(f) => {
                Ok(Expr::FloatConstant(f,pos))
            }
            Token::Identifier(i) => {
                let (peek,pos1)=self.token_stream.peek().unwrap();
                match peek {
                    Token::Plus => {
                        self.token_stream.next();
                        let right=self.parse_expr()?;
                        let left=Box::new(Expr::Variable(i,pos.clone()));
                        pos.add_span(1+right.position().span);
                        Ok(BinaryExpr(Op::Plus,left,Box::new(right),pos))
                    }
                    Token::BraceLeft=>{
                        let (args,pos2)=self.parse_fn_call_args().unwrap();
                        let fn_expr=FnCallExpr{name:i,args};
                        pos.add_span(pos2.span);
                        return Ok(Expr::FnCall(fn_expr,pos));
                    }
                    _=> Ok(Expr::Variable(i,pos))
                }
            }
            _=>Err(ParserError::UnexpectedToken(token))
        }
    }
    pub fn parse_special_token(&mut self,rhs: Token)->ParserResult<(Token,Position)>{
        let (token,pos)=self.token_stream.next().unwrap();
        match token {
            t if t.token_id()==rhs.token_id()=>{
                return Ok((t,pos))
            }
            _=>Err(ParserError::UnexpectedToken(token))

        }
    }
    pub fn from_token_stream(token_stream:TokenStream)->Self{
        return Self{ token_stream,fn_lib:vec![] }
    }

}
pub type ParserResult<T>=Result<T,ParserError>;
#[derive(Debug)]
pub enum ParserError{
    UnexpectedToken(Token),
    UnusedKeyword(String)
}
pub trait Parser{
    fn ident()->String;
    fn parse(p:&mut PipelineParser)->ParserResult<Stmt>;
}
#[derive(Debug,Clone)]
pub struct  VariableDeclaration{
    pub name:String,
    pub declaration_type:String
}

impl VariableDeclaration {
    pub fn new(name:String,dec:String)->Self{
        Self{name,declaration_type:dec}
    }
}
#[derive(Debug,Clone)]
pub struct FnDef{
    pub name:String,
    pub args:Vec<VariableDeclaration>,
    pub body:Vec<Stmt>
}

impl FnDef {
    pub fn new(name:String,args:Vec<VariableDeclaration>,body:Vec<Stmt>)->Self{
        Self{name,args,body}
    }
}
pub struct FnParser;
impl Parser for FnParser{
    fn ident() -> String {
        return "fn".to_string()
    }

    fn parse(p: &mut PipelineParser) -> ParserResult<Stmt> {
        return Ok(Stmt::Noop)
    }
}