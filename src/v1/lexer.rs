use std::{fs, io};
use std::ops::{Add, Deref};
use crate::v1::position::Position;
use crate::v1::token::Token;

#[derive(Debug,Clone)]
pub struct Lexer{
    chars:Vec<char>,
    index:usize,
    col:usize,
    row:usize,
    keywords:Vec<&'static str>
}
pub struct TokenStream{
    tokenizer:Lexer,
    index:usize,
    peek:Option<(Token,Position)>
}

impl Iterator for TokenStream {

    type Item = (Token,Position);

    fn next(&mut self) -> Option<Self::Item> {
        self.next()
    }
}
impl TokenStream{
    pub fn new()->Self{
        Self{
            tokenizer:Lexer::new(),
            index:0,
            peek:None
        }
    }
    pub fn set_lexer(&mut self,lexer: Lexer){
        self.tokenizer=lexer;
    }
    pub fn next(&mut self)->Option<(Token,Position)>{
        if self.peek.is_some(){
            return self.peek.take()
        }
        return self.tokenizer.next()
    }
    pub fn peek(&mut self)->Option<(Token,Position)>{
        if self.peek.is_some(){
            return self.peek.clone()
        }
        let o=self.tokenizer.next();
        self.peek=o.clone();
        return o
    }
}

impl IntoIterator for Lexer {
    type Item = (Token,Position);
    type IntoIter = TokenStream;

    fn into_iter(self) -> Self::IntoIter {
        TokenStream{
            index: 0,
            tokenizer:self,
            peek:None
        }
    }
}
impl Lexer{
    pub fn new()->Self{
        Self{
            chars: vec![],
            index: 0,
            col: 0,
            row: 0,
            keywords: vec!["fn","let","return"],
        }
    }
    pub fn set_chars(&mut self,chars:Vec<char>){
        self.chars=chars;
    }
    pub fn next(&mut self)->Option<(Token,Position)>{
        let one=self.chars.get(self.index);
        loop{
            match self.current_char() {
                None => { return None}
                Some(c) => {
                    let peek=self.peek_char().unwrap_or('\0');
                    match (c,peek) {
                        ('0'..='9'|'.',_)=>{
                            return self.scan_number()
                        },
                        ('a'..='z'|'A'..='Z',_)=>{
                            let ident= self.scan_identifier();
                            let clone=ident.clone().unwrap();
                            let ident_str=clone.0.get_identifier_value();
                            if self.keywords.contains(&ident_str){
                                return Some((Token::Keyword(String::from(ident_str)),clone.1))
                            }
                            return ident
                        },
                        ('(',_)=>{
                            let r= Some((Token::BraceLeft,Position::new(self.index,1)));
                            self.next_char();
                            return r
                        },
                        (')',_)=>{
                            let r= Some((Token::BraceRight,Position::new(self.index,1)));
                            self.next_char();
                            return r
                        },
                        ('{',_)=>{
                            let r= Some((Token::ParenthesisLeft,Position::new(self.index,1)));
                            self.next_char();
                            return r
                        },
                        ('}',_)=>{
                            let r= Some((Token::ParenthesisRight,Position::new(self.index,1)));
                            self.next_char();
                            return r
                        }
                        (':',_)=>{
                            let r= Some((Token::Colon,Position::new(self.index,1)));
                            self.next_char();
                            return r
                        }
                        (',',_)=>{
                            let r= Some((Token::Comma,Position::new(self.index,1)));
                            self.next_char();
                            return r
                        }
                        ('=',_)=>{
                            let r= Some((Token::Assign,Position::new(self.index,1)));
                            self.next_char();
                            return r
                        }
                        ('+',_)=>{
                            let r= Some((Token::Plus,Position::new(self.index,1)));
                            self.next_char();
                            return r
                        }
                        ('"',_)=>{
                            return self.scan_string()
                        }
                        (' '|'\n'|'\r',_)=>{
                            self.next_char();
                        },
                        ('/','/')=>{
                            while self.peek_char()!=Some('\n') {
                                self.next_char();
                            }
                        }
                        _ => {
                            return None
                        }
                    }
                }
            }
        }

    }
    fn peek_char(&self)->Option<char>{
        self.chars.get(self.index+1).map(|c|c.clone())
    }
    fn next_char(&mut self)->Option<char>{
        let c=self.peek_char();
        self.increase_index();
        c
    }
    fn current_char(&self)->Option<&char>{
        self.chars.get(self.index)
    }
    fn increase_index(&mut self){
        if self.chars.get(self.index).unwrap()==&'\n'{
            self.row+=1;
            self.col=0;
        }
        self.index=self.index.add(1);
        self.col+=1;
    }
    fn scan_number(&mut self)->Option<(Token,Position)>{
        let mut v=String::new();
        let mut pos=Position::with_pos(self.index);
        let mut is_decimal=false;
        while let  Some(c ) =self.current_char(){
            if c==&'.'&&!is_decimal{
                v.push(c.clone());
                self.increase_index();
                is_decimal=true;
                continue
            }
            if !c.is_numeric(){
                break
            }
            v.push(c.clone());
            self.increase_index();
        }
        pos.set_span(v.len());
        if is_decimal{
            let f:f64=v.parse().unwrap();
            return Some((Token::Float(f),pos))
        }
        let i:i64=v.parse().unwrap();
        Some((Token::Int(i),pos))
    }
    fn scan_identifier(&mut self)->Option<(Token,Position)>{
        let mut v=String::new();
        let mut pos=Position::with_pos(self.index);
        while let  Some(c ) =self.current_char(){
            if !c.is_alphabetic(){
                break
            }
            v.push(c.clone());
            self.increase_index();
        }
        pos.set_span(v.len());
        return Some((Token::Identifier(v),pos))
    }
    fn scan_string(&mut self)->Option<(Token,Position)>{
        let mut v=String::new();
        let mut pos=Position::with_pos(self.index);
        self.increase_index();
        while let  Some(c ) =self.current_char(){
            if c==&'"'{
                break
            }
            v.push(c.clone());
            self.increase_index();
        }
        self.increase_index();
        pos.set_span(v.len()+2);
        return Some((Token::String(v),pos))
    }
    pub fn get_source(&self) -> Vec<char> {
        return self.chars.clone()
    }
    pub fn from_path(path:impl AsRef<str>) ->Self{
        let script=fs::read_to_string(path.as_ref()).unwrap();
        return Self{  chars: script.chars().collect(), index: 0, col: 0, row: 0, keywords: vec!["let","fn","return"] }
    }
    pub fn from_script(script:impl AsRef<str>)->Self{
        return Self{  chars: script.as_ref().chars().collect(), index: 0, col: 0, row: 0, keywords: vec!["let","fn","return"] }
    }

}