use std::cell::{Cell, RefCell};
use crate::ast::{Argument, AST, Closure, FunctionCall};

use crate::token::Token;

pub struct Parser{
    token_stream:RefCell<Vec<Token>>,
    pos:Cell<usize>
}
impl Parser{
    pub fn from_token_stream(token_stream:Vec<Token>)->Self{
        return Self{ token_stream: RefCell::new(token_stream), pos: Cell::new(0) }
    }
    pub fn generate_ast(&self)->AST{
        let mut ast=AST::new();
        while let Some(fc)=self.try_parse_function_call(){
            ast.add(fc);
        }
        return ast
    }
    fn parse_function_call(&self)->FunctionCall{
        let name=self.consume_identifier();
        let mut res=FunctionCall::new(name);
        self.consume_parenthesis_left();
        while let Some(a)=self.try_consume_argument(){
            res.add_argument(a)
        }
        self.consume_parenthesis_right();
        if let Some(argument)=self.try_parse_outside_closure(){
            res.add_argument(argument);
        }
        return res
    }
    fn try_parse_function_call(&self)->Option<FunctionCall>{
        if let Some(name)=self.try_consume_identifier(){
            let mut res=FunctionCall::new(name);
            self.consume_parenthesis_left();
            while let Some(a)=self.try_consume_argument(){
                res.add_argument(a)
            }
            self.consume_parenthesis_right();
            if let Some(argument)=self.try_parse_outside_closure(){
                res.add_argument(argument);
            }
            return Some(res)
        }
        return None

    }
    fn pos_forward(&self,step:usize){
        self.pos.set(self.pos.get()+step)
    }
    fn consume_identifier(&self)->String{
        if let Token::Identifier(s) = self.token_stream.borrow().get(self.pos.get()).expect("缺少标识符"){
            self.pos_forward(1);
            return s.clone()
        }
        panic!("缺少标识符")
    }
    fn try_consume_identifier(&self)->Option<String>{
        if let Some(Token::Identifier(s)) = self.token_stream.borrow().get(self.pos.get()){
            self.pos_forward(1);
            return Some(s.clone());
        }
        return None
    }
    fn try_consume_argument(&self)->Option<Argument>{
        let cur_pos=self.pos.get();
        match self.token_stream.borrow().get(cur_pos){
            Some(Token::String(s))=>{
                self.pos_forward(1);
                return Some(Argument::StringArgument(s.clone()));
            }
            Some(Token::Identifier(_))=>{
                let fc=self.parse_function_call();
                return Some(Argument::FunctionCallArgument(Box::new(fc)));
            }
            Some(Token::Comma)=>{
                self.pos_forward(1);
                return self.try_consume_argument();
            }
            _=>return None
        }
        return None
    }
    fn consume_parenthesis_left(&self){
        if let Some(Token::ParenthesisLeft) = self.token_stream.borrow().get(self.pos.get()){
            self.pos_forward(1);
            return;
        }
        panic!("缺少'('")
    }
    fn consume_parenthesis_right(&self){
        if let Token::ParenthesisRight = self.token_stream.borrow().get(self.pos.get()).expect("缺少')'"){
            self.pos_forward(1);
            return;
        }
        panic!("缺少')'")
    }
    fn try_parse_outside_closure(&self)->Option<Argument>{
        if self.try_consume_brace_left(){
            let mut closure=Closure::new();
            while let Some(fc)=self.try_parse_function_call(){
                closure.add_expression(fc)
            }
            self.consume_brace_right();
            return Some(Argument::ClosureArgument(Box::new(closure)))
        }
        return None
    }
    #[allow(unused)]
    fn consume_brace_left(&self){
        if let Token::BraceLeft= self.token_stream.borrow().get(self.pos.get()).expect("缺少'{'"){
            self.pos_forward(1);
            return;
        }
        panic!("缺少'{{'")
    }
    fn try_consume_brace_left(&self)->bool{
        if let Token::BraceLeft= self.token_stream.borrow().get(self.pos.get()).expect("缺少'{'"){
            self.pos_forward(1);
            return true;
        }
       return false
    }
    fn consume_brace_right(&self){
        if let Token::BraceRight = self.token_stream.borrow().get(self.pos.get()).expect("缺少'{'"){
            self.pos_forward(1);
            return;
        }
        panic!("缺少'{{'")
    }

}