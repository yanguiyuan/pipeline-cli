use std::sync::Arc;
use tokio::sync::RwLock;
use crate::builtin::{cmd, workspace};
use crate::context::Context;
use crate::core::operation::OperationType::{Cmd, UnKnown, Workspace};
use crate::core::pipeline::PipelineContextValue;

#[derive(Debug,Clone)]
pub struct Operation{
    op_type:OperationType
}
#[derive(Debug,Clone)]
pub enum  OperationType{
    Cmd(String),
    Workspace(String),
    UnKnown(String)
}
impl OperationType{
    pub fn from_str(s:&str,p:&str)->Self{
        match s {
            "cmd"=>Cmd(p.to_string()),
            "workspace"=>Workspace(p.to_string()),
            _=>UnKnown(s.to_string())
        }
    }
}

impl Operation{
    pub fn new()->Self{
        Self{ op_type: OperationType::UnKnown("".to_string()) }
    }
    pub fn from_str(s:&str,p:&str)->Self{
        Self{op_type:OperationType::from_str(s,p)}
    }
    //op的execute
    pub async fn execute(&self, ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>){
        match &self.op_type {
            Cmd(s) => {
                cmd(s,ctx.clone()).await
            }
            Workspace(s) => {
                workspace(s,ctx.clone()).await
            }
            UnKnown(_) => {}
        }
    }
}