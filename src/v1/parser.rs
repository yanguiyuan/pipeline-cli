use std::{env, fs};
use crate::error::{PipelineError, PipelineResult};
use crate::error::PipelineError::UnknownModule;
use crate::module::Module;
use crate::v1::lexer::{Lexer, TokenStream};
use crate::v1::stmt::{IfBranchStmt, IfStmt, Stmt};

use crate::v1::token::Token;
use crate::v1::ast::AST;
use crate::v1::expr::{Expr, FnCallExpr, FnClosureExpr, Op};
use crate::v1::expr::Expr::BinaryExpr;
use crate::v1::position::{NONE, Position};

pub struct PipelineParser{
    token_stream: TokenStream,
    fn_lib:Vec<FnDef>,
    modules:Vec<Module>
}
impl PipelineParser{
    pub fn new()->Self{
        Self{token_stream:TokenStream::new(),fn_lib:vec![],modules:vec![]}
    }
    pub fn set_lexer(&mut self,lexer: Lexer){
        self.token_stream.set_lexer(lexer)
    }
    pub fn get_fn_lib(&self)->Vec<FnDef>{
        self.fn_lib.clone()
    }
    pub fn get_modules(&self)->&Vec<Module>{
        &self.modules
    }
    pub fn compile_from_token_stream(&mut self)->PipelineResult<AST>{
        let stmts=self.parse_stmt_blocks()?;
        return Ok(AST{ body: stmts })
    }
    pub fn parse_stmt_blocks(&mut self)->PipelineResult<Vec<Stmt>>{
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

    pub fn parse_stmt(&mut self)->PipelineResult<Stmt>{
        loop {
            let (token,pos)=self.token_stream.peek();
            return match token {
                Token::Keyword(k)=>{
                    match k.as_str() {
                        "let"=>{
                            return self.parse_let_stmt()
                        }
                        "fn"|"fun"=>{
                            let (fn_def,pos)=self.parse_fn_def()?;
                            self.fn_lib.push(fn_def);
                            continue
                        }
                        "return"=>{
                            self.parse_return_stmt()
                        }
                        "if"=>{
                            self.parse_if_stmt()
                        }
                        "while"=>{
                            self.parse_while_stmt()
                        }
                        "import"=>{

                            self.parse_import_stmt()
                        }
                        t=>Err(PipelineError::UnusedKeyword(t.into()))
                    }
                },
                Token::ParenthesisRight=>Ok(Stmt::Noop),
                _=>return self.parse_expr_stmt()
            }
        }

    }
    pub fn parse_module(&mut self,module_name:impl AsRef<str>)->PipelineResult<Module>{
        let mut current_dir = match env::current_dir() {
            Ok(path) => path,
            Err(e) => {
                return Err(UnknownModule(module_name.as_ref().into()));
            }
        };
        current_dir.push(format!("{}.kts",module_name.as_ref()));
        // 打印当前工作目录
        let script=fs::read_to_string(current_dir).unwrap();
        let mut parser=PipelineParser::new();
        let lexer=Lexer::from_script(script);
        parser.set_lexer(lexer);
        parser.parse_stmt_blocks()?;
        let lib=parser.get_fn_lib();
        let mut m=Module::new(module_name.as_ref());
        for l in lib{
            m.register_script_function(l.name.clone(),l)
        }
        return Ok(m);
    }
    pub fn parse_import_stmt(&mut self,)->PipelineResult<Stmt>{
        let (ret,mut pos)=self.token_stream.next();
        if let Token::Keyword(s)=ret.clone(){
            if s!="import"{
                return Err(PipelineError::UnusedKeyword(s));
            }
            let (next,pos1)=self.token_stream.next();
            return match next {
               Token::Identifier(id)=>{
                   pos.add_span(pos1.span);
                    let m =self.parse_module(id.clone())?;
                    self.modules.push(m);
                   Ok(Stmt::Import(id,pos))
               }
               t=>Err(PipelineError::UnexpectedToken(t))
            }
        }
        return Err(PipelineError::UnexpectedToken(ret));
    }
    fn parse_if_branch(&mut self)->PipelineResult<(IfBranchStmt,Position)>{
        let (ret,mut pos)=self.token_stream.next();
        if let Token::Keyword(s)=ret.clone(){
            if s!="if"{
                return Err(PipelineError::UnusedKeyword(s));
            }
            let expr=self.parse_expr()?;
            pos.add_span(expr.position().span);
            self.parse_special_token(Token::ParenthesisLeft)?;
            let blocks=self.parse_stmt_blocks()?;
            self.parse_special_token(Token::ParenthesisRight)?;
            for i in &blocks{
                pos.add_span(i.position().span)
            }
            return Ok((IfBranchStmt::new(expr,blocks),pos))
        }
        return Err(PipelineError::UnexpectedToken(ret));
    }

    pub fn parse_if_stmt(&mut self)->PipelineResult<Stmt>{
        let mut branches=vec![];
        let mut else_body=None;
        let ( b,pos)=self.parse_if_branch()?;
        branches.push(b);

        loop{
            let (peek,pos1)=self.token_stream.peek();
            match peek.clone() {
                Token::Keyword(k) if k=="else" =>{
                    self.token_stream.next();
                    let (peek0,pos01)=self.token_stream.peek();
                    if let Token::ParenthesisLeft=peek0{
                        self.parse_special_token(Token::ParenthesisLeft)?;
                        let blocks=self.parse_stmt_blocks()?;
                        self.parse_special_token(Token::ParenthesisRight)?;
                        else_body=Some(blocks);
                        break
                    }
                    let (b0,pos00)=self.parse_if_branch()?;
                    branches.push(b0);
                }
                _=>{
                    break
                }
            }
        }

        return Ok(Stmt::If(Box::new(IfStmt::new(branches,else_body)),pos))

    }
    pub fn parse_while_stmt(&mut self)->PipelineResult<Stmt>{
        let (ret,mut pos)=self.token_stream.next();
        if let Token::Keyword(s)=ret.clone(){
            if s!="while"{
                return Err(PipelineError::UnusedKeyword(s));
            }
            let expr=self.parse_expr()?;
            pos.add_span(expr.position().span);
            self.parse_special_token(Token::ParenthesisLeft)?;
            let blocks=self.parse_stmt_blocks()?;
            self.parse_special_token(Token::ParenthesisRight)?;
            for i in &blocks{
                pos.add_span(i.position().span)
            }
            return Ok(Stmt::While(Box::new(expr),Box::new(blocks), pos))
        }
        return Err(PipelineError::UnexpectedToken(ret));
    }
    pub fn parse_return_stmt(&mut self)->PipelineResult<Stmt>{
        let (ret,mut pos)=self.token_stream.next();
        if let Token::Keyword(s)=ret.clone(){
            if s!="return"{
                return Err(PipelineError::UnusedKeyword(s));
            }
            let expr=self.parse_expr()?;
            pos.add_span(expr.position().span);
            return Ok(Stmt::Return(Box::new(expr), pos))
        }
        return Err(PipelineError::UnexpectedToken(ret));
    }
     fn parse_let_stmt(&mut self)->PipelineResult<Stmt>{
        let (token,mut pos)=self.token_stream.next();
        if let Token::Keyword(_)= token{
            let (token1,pos0)=self.token_stream.next();
            if let Token::Identifier(ident)=token1{
                pos.add_span(pos0.span);
                self.parse_special_token(Token::Assign)?;
                pos.add_span(1);
                let expr=self.parse_expr()?;
                pos.add_span(expr.position().span);
                return Ok(Stmt::Let(Box::new((ident, expr)), pos));
            }
            return Err(PipelineError::UnexpectedToken(token1))
        }
        return Err(PipelineError::UnexpectedToken(token))
    }
    pub fn parse_fn_def(&mut self)->PipelineResult<(FnDef,Position)>{
        let (next,mut pos)=self.token_stream.next();
        match next {
            Token::Keyword(s) if s.as_str()=="fn"||s.as_str()=="fun"=>{
                pos.add_span(2);
                let (next1,pos1)=self.token_stream.next();
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
                return Err(PipelineError::UnexpectedToken(next1))
            },
            _=>{
                Err(PipelineError::UnexpectedToken(next))
            }
        }
    }
    pub fn parse_fn_def_args(&mut self)->PipelineResult<(Vec<VariableDeclaration>,Position)>{
       let start= self.parse_special_token(Token::BraceLeft)?;
        let mut v=vec![];
        let mut p=start.1;
        loop {
            let (token,_)=self.token_stream.peek();
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
    pub fn parse_variable_declaration(&mut self)->PipelineResult<(VariableDeclaration,Position)>{
        let (next,mut pos)=self.token_stream.next();
        if let Token::Identifier(s)=next.clone(){
            self.parse_special_token(Token::Colon)?;
            let (next1,pos1)=self.token_stream.next();
            if let Token::Identifier(s1)=next1{
                pos.add_span(1+pos1.span);
                return Ok((VariableDeclaration::new(s,s1),pos))
            }
            return Err(PipelineError::UnexpectedToken(next1))
        }
        return Err(PipelineError::UnexpectedToken(next))
    }

    pub fn parse_expr_stmt(&mut self)->PipelineResult<Stmt>{
        let (token,mut pos0)=self.token_stream.next();
        return match token {
            Token::Identifier(s) => {
                let (peek, pos) = self.token_stream.peek();
                match peek {
                    Token::BraceLeft => {
                        let (fn_call_expr, pos) = self.parse_fn_call_expr(&s, pos0)?;
                        Ok(Stmt::FnCall(Box::new(fn_call_expr), pos))
                    }
                    Token::Assign => {
                        let (_, mut pos) = self.token_stream.next();
                        let expr = self.parse_expr()?;
                        pos.add_span(expr.position().span);
                        return Ok(Stmt::Let(Box::new((s, expr)), pos))
                    }
                    Token::SquareBracketLeft=>{
                        self.token_stream.next();
                        let expr=self.parse_math_expr()?;
                        self.parse_special_token(Token::SquareBracketRight)?;
                        self.parse_special_token(Token::Assign)?;
                        let value=self.parse_expr()?;
                        pos0.add_span(1+1+expr.position().span+value.position().span);
                        return Ok(Stmt::ArrayAssign(s,Box::new(expr),Box::new(value),pos0))

                    }
                    Token::ScopeSymbol=>{
                        self.token_stream.next();
                        let (next,pos1)=self.token_stream.next();
                        let fc_name=next.get_identifier_value();
                        let (args,pos2)=self.parse_fn_call_args().unwrap();
                        let fn_expr=FnCallExpr{name:s+"::"+fc_name,args};
                        pos0.add_span(pos1.span+pos2.span+2);
                        return Ok(Stmt::FnCall(Box::new(fn_expr),pos));
                    }
                    Token::Dot=>{
                        self.token_stream.next();
                        let (token1,mut pos1)=self.token_stream.next();
                        let (mut fn_call_expr, pos) = self.parse_fn_call_expr(&token1.get_identifier_value(), pos1)?;
                        fn_call_expr.args.insert(0,Expr::Variable(s,pos0));
                        Ok(Stmt::FnCall(Box::new(fn_call_expr), pos))
                    }
                    _ => Ok(Stmt::Noop)
                }
            }
            _ => Ok(Stmt::Noop)
        }
    }
    pub fn parse_fn_call_expr(&mut self,s:&str,pos:Position) -> PipelineResult<(FnCallExpr,Position)> {
        let mut pos=pos;
        let (mut args,args_pos)=self.parse_fn_call_args()?;
        pos.add_span(args_pos.span);
        if let (peek,mut pos1)=self.token_stream.peek(){
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
            name: s.into(),
            args
        },pos))
    }
    pub fn parse_fn_call_args(&mut self)->PipelineResult<(Vec<Expr>,Position)>{
        self.parse_special_token(Token::BraceLeft)?;
        let mut v =vec![];
        let mut p=NONE.clone();
        p.add_span(1);
        loop {
            let (peek,_)=self.token_stream.peek();
            if peek==Token::BraceRight{
                self.token_stream.next();
                break
            }
            let expr=self.parse_expr()?;
            v.push(expr.clone());
            let expr_pos=expr.position();
            if p.is_none(){
                p.set_pos(expr_pos.pos);
            }
            p.add_span(expr_pos.span);

            let (token,_)=self.token_stream.peek();

            match token {
                Token::BraceRight => {
                    p.add_span(1);
                    self.token_stream.next();
                    break
                }
                Token::Comma => {
                    self.token_stream.next();
                }
                _=>return Err(PipelineError::UnexpectedToken(token))
            }
        }
        return Ok((v,p))
    }

    fn parse_term(&mut self)->PipelineResult<Expr>{
        let (token,mut pos)=self.token_stream.next();
        match token {
            Token::String(s) => {
                Ok(Expr::StringConstant(s,pos))
            }
            Token::Int(i) => {
                let (peek,_)=self.token_stream.peek();
                match peek {
                    Token::Mul=>{
                        self.token_stream.next();
                        let right=self.parse_term()?;
                        let left=Box::new(Expr::IntConstant(i,pos.clone()));
                        pos.add_span(1+right.position().span);
                        Ok(BinaryExpr(Op::Mul,left,Box::new(right),pos))
                    }
                    Token::Div=>{
                        self.token_stream.next();
                        let right=self.parse_term()?;
                        let left=Box::new(Expr::IntConstant(i,pos.clone()));
                        pos.add_span(1+right.position().span);
                        Ok(BinaryExpr(Op::Div,left,Box::new(right),pos))
                    }
                    Token::Mod=>{
                        self.token_stream.next();
                        let right=self.parse_term()?;
                        let left=Box::new(Expr::IntConstant(i,pos.clone()));
                        pos.add_span(1+right.position().span);
                        Ok(BinaryExpr(Op::Mod,left,Box::new(right),pos))
                    }
                    _=>Ok(Expr::IntConstant(i,pos))
                }

            }
            Token::Float(f) => {
                Ok(Expr::FloatConstant(f,pos))
            }
            Token::Identifier(ident) => {
                let (peek,mut pos1)=self.token_stream.peek();
                match peek {
                    Token::Mul=>{
                        self.token_stream.next();
                        let right=self.parse_term()?;
                        let left=Box::new(Expr::Variable(ident,pos.clone()));
                        pos.add_span(1+right.position().span);
                        Ok(BinaryExpr(Op::Mul,left,Box::new(right),pos))
                    }
                    Token::Div=>{
                        self.token_stream.next();
                        let right=self.parse_term()?;
                        let left=Box::new(Expr::Variable(ident,pos.clone()));
                        pos.add_span(1+right.position().span);
                        Ok(BinaryExpr(Op::Div,left,Box::new(right),pos))
                    }
                    Token::Mod=>{
                        self.token_stream.next();
                        let right=self.parse_term()?;
                        let left=Box::new(Expr::Variable(ident,pos.clone()));
                        pos.add_span(1+right.position().span);
                        Ok(BinaryExpr(Op::Mod,left,Box::new(right),pos))
                    }
                    Token::BraceLeft=>{
                        let (args,pos2)=self.parse_fn_call_args().unwrap();
                        let fn_expr=FnCallExpr{name:ident,args};
                        pos.add_span(pos2.span);
                        return Ok(Expr::FnCall(fn_expr,pos));
                    }
                    Token::ScopeSymbol=>{
                        self.token_stream.next();
                        let (next,pos1)=self.token_stream.next();
                        let fc_name=next.get_identifier_value();
                        let (args,pos2)=self.parse_fn_call_args().unwrap();
                        let fn_expr=FnCallExpr{name:ident+"::"+fc_name,args};
                        pos.add_span(pos1.span+pos2.span+2);
                        return Ok(Expr::FnCall(fn_expr,pos));
                    }
                    Token::SquareBracketLeft=>{
                        self.token_stream.next();
                        let e=self.parse_math_expr()?;
                        self.parse_special_token(Token::SquareBracketRight)?;
                        pos1.add_span(1+e.position().span+1);
                        return Ok(Expr::Index(ident,Box::new(e),pos1))
                    }
                    _=> Ok(Expr::Variable(ident,pos))
                }
            }
            _=>Err(PipelineError::UnexpectedToken(token))
        }
    }
    fn parse_math_expr(&mut self)->PipelineResult<Expr>{
        let lhs=self.parse_term()?;
        let next=self.token_stream.peek();
        return match next.0 {
            Token::Plus=>{
                self.token_stream.next();
                let mut pos=lhs.position();
                let rhs=self.parse_expr()?;
                pos.add_span(1+rhs.position().span);
                Ok(Expr::BinaryExpr(Op::Plus,Box::new(lhs),Box::new(rhs),pos))
            }
            Token::Minus=>{
                self.token_stream.next();
                let mut pos=lhs.position();
                let rhs=self.parse_expr()?;
                pos.add_span(1+rhs.position().span);
                Ok(Expr::BinaryExpr(Op::Minus,Box::new(lhs),Box::new(rhs),pos))
            }
            _=>Ok(lhs)
        }
    }
    fn parse_array(&mut self)->PipelineResult<Expr>{
        let (next,mut pos)=self.token_stream.next();
        let mut v=vec![];
        loop{
            let e=self.parse_expr()?;
            v.push(e.clone());
            pos.add_span(e.position().span);
            let (peek,pos0)=self.token_stream.peek();
            match peek {
                Token::Comma=>{
                    self.token_stream.next();
                    pos.add_span(1);
                    continue
                }
                Token::SquareBracketRight=>{
                    self.token_stream.next();
                    pos.add_span(1);
                    break
                }
                t=>return Err(PipelineError::UnexpectedToken(t))
            }
        }
        return Ok(Expr::Array(v,pos))

    }
    pub fn parse_expr(&mut self)->PipelineResult<Expr>{
        let (peek,pos)=self.token_stream.peek();
        if peek==Token::SquareBracketLeft{
            return self.parse_array()
        }
        let lhs=self.parse_math_expr()?;
        let next=self.token_stream.peek();
        return match next.0 {
            Token::Greater=>{
                self.token_stream.next();
                let mut pos=lhs.position();
                let rhs=self.parse_math_expr()?;
                pos.add_span(1+rhs.position().span);
                Ok(Expr::BinaryExpr(Op::Greater,Box::new(lhs),Box::new(rhs),pos))
            }
            Token::Less=>{
                self.token_stream.next();
                let mut pos=lhs.position();
                let rhs=self.parse_math_expr()?;
                pos.add_span(1+rhs.position().span);
                Ok(Expr::BinaryExpr(Op::Less,Box::new(lhs),Box::new(rhs),pos))
            }
            Token::Equal=>{
                self.token_stream.next();
                let mut pos=lhs.position();
                let rhs=self.parse_math_expr()?;
                pos.add_span(1+rhs.position().span);
                Ok(Expr::BinaryExpr(Op::Equal,Box::new(lhs),Box::new(rhs),pos))
            }
            _=>Ok(lhs)
        }
    }
    pub fn parse_special_token(&mut self,rhs: Token)->PipelineResult<(Token,Position)>{
        let (token,pos)=self.token_stream.next();
        match token {
            t if t.token_id()==rhs.token_id()=>{
                return Ok((t,pos))
            }
            _=>Err(PipelineError::UnexpectedToken(token))

        }
    }
    #[allow(unused)]
    pub fn from_token_stream(token_stream:TokenStream)->Self{
        return Self{ token_stream,fn_lib:vec![],modules:vec![] }
    }

}

pub trait Parser{
    fn ident()->String;
    fn parse(p:&mut PipelineParser)->PipelineResult<Stmt>;
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