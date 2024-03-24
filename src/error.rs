use crate::v1::types::Dynamic;



pub type PipelineResult<T>=Result<T,PipelineError>;
#[derive(Debug,Clone)]
pub enum PipelineError{
    FunctionUndefined(String),
    VariableUndefined(String),
    ExpectedDataType(String),
    UnexpectedToken(crate::v1::token::Token),
    UnusedKeyword(String),
    UnknownModule(String)
}

