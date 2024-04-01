
pub type PipelineResult<T>=Result<T,PipelineError>;
#[derive(Debug,Clone)]
pub enum PipelineError{
    FunctionUndefined(String),
    VariableUndefined(String),
    ExpectedType(String),
    UnexpectedType(String),
    UnexpectedToken(crate::v1::token::Token),
    UnusedKeyword(String),
    UnknownModule(String),
    UndefinedOperation(String)
}

