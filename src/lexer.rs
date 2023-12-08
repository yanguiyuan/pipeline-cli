use std::{fs, io};
use std::cell::{Cell, RefCell};
use crate::token::Token;

pub struct Lexer{

    token_stream:RefCell<Vec<Token>>,
    chars:Vec<char>,
    pos:Cell<usize>
}
impl Lexer{
    pub fn from_path(path:&'static str)->io::Result<Self>{
        let script=fs::read_to_string(path)?;
        return Ok(Self{  token_stream: RefCell::new(vec![]), chars: script.chars().collect(), pos: Cell::new(0) })
    }
    pub fn tokenize(&self)->Result<Vec<Token>,String>{
        loop {
            match self.peek(){
                Some(char)=>{
                    match char{
                        'a'..='z'|'A'..='Z'=>{
                            self.scan_identifier();
                        }
                        '\"'=>{
                            self.scan_string();
                        }
                        '('=>{self.token_stream.borrow_mut().push(Token::ParenthesisLeft);self.next();}
                        ')'=>{self.token_stream.borrow_mut().push(Token::ParenthesisRight);self.next();}
                        '{'=>{self.token_stream.borrow_mut().push(Token::BraceLeft);self.next();}
                        '}'=>{self.token_stream.borrow_mut().push(Token::BraceRight);self.next();}
                        '.'=>{self.token_stream.borrow_mut().push(Token::Dot);self.next();}
                        '\r'|'\n'|' '=>{self.next();}
                        b=>{return Err(format!("未定义的符号‘{b}’"))}
                    }
                }
                None=>{break}
            }
        }
        let tokens=self.token_stream.borrow();
        return Ok(tokens.clone())
    }
    fn next(&self) -> Option<char> {
        if self.chars.len()<=self.pos.get(){return None}
        let res=Some(self.chars[self.pos.get()]);
        self.pos_forward(1);
        return res;
    }
    fn peek(&self)->Option<char>{
        if self.chars.len()<=self.pos.get(){return None}
        let res=Some(self.chars[self.pos.get()]);
        return res;
    }
    fn pos_forward(&self,step:usize){
       self.pos.set(self.pos.get()+step)
    }
    fn scan_identifier(&self){
        let mut token_value=String::new();
        token_value.push(self.next().unwrap());
        loop {
            if let Some(c)=self.peek(){
                match c {
                    '0'..='9'|'a'..='z'|'A'..='Z'=>{
                        token_value.push(c);
                        self.next();
                    }
                    _ => {break}
                }
            }
        }
        self.token_stream.borrow_mut().push(Token::Identifier(token_value))
    }
    fn scan_string(&self){
        let mut token_value=String::new();
        self.next();
        loop {
            if let Some(c)=self.next(){
                match c {
                    '\"'=>{
                        break
                    }
                    _ => {
                        token_value.push(c);
                    }
                }
            }
        }
        self.token_stream.borrow_mut().push(Token::String(token_value))
    }
}