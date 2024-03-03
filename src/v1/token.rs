#[allow(unused)]
#[derive(Debug, Clone,PartialEq)]
pub enum  Token{
    String(String),
    Int(i64),
    Float(f64),
    Identifier(String),
    /// 关键字
    Keyword(String),
    /// (
    BraceLeft,
    /// )
    BraceRight,
    /// [
    SquareBracketLeft,
    /// ]
    SquareBracketRight,
    /// {
    ParenthesisLeft,
    /// }
    ParenthesisRight,
    /// .
    Dot,
    /// :
    Colon,
    /// =
    Assign,
    /// ,
    Comma,
    EOF
}

impl Token {
    pub fn token_id(&self)->i8{
        match self {
            Token::String(_) => 0,
            Token::Int(_) => 1,
            Token::Float(_) => 2,
            Token::Identifier(_) => 3,
            Token::Keyword(_) => 4,
            Token::BraceLeft => 5,
            Token::BraceRight => 6,
            Token::SquareBracketLeft => 7,
            Token::SquareBracketRight => 8,
            Token::ParenthesisLeft => 9,
            Token::ParenthesisRight => 10,
            Token::Dot => 11,
            Token::Comma => 12,
            Token::EOF => 13,
            Token::Colon=>14,
            Token::Assign=>15,
        }
    }
    pub fn get_identifier_value(&self)->&str{
        return match self {
            Token::Identifier(s) => {
                s.as_str()
            }
            _ => {
                ""
            }
        }
    }
}