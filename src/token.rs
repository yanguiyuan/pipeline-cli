#[allow(unused)]
#[derive(Debug, Clone)]
pub enum  Token{
    String(String),
    Int(i64),
    Float(f64),
    Identifier(String),
    BraceLeft,
    BraceRight,
    SquareBracketLeft,
    SquareBracketRight,
    ParenthesisLeft,
    ParenthesisRight,
    Dot,
    Comma
}