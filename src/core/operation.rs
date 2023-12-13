use std::sync::Arc;
use tokio::sync::RwLock;
use crate::builtin::{cmd, move_file, workspace};
use crate::context::Context;
use crate::core::operation::OperationType::{Cmd, MoveFile, UnKnown, Workspace};
use crate::core::pipeline::PipelineContextValue;

#[derive(Debug,Clone)]
pub struct Operation{
    op_type:OperationType
}
#[derive(Debug,Clone)]
pub enum  OperationType{
    Cmd(String),
    Workspace(String),
    MoveFile(String,String),
    UnKnown(String,Vec<String>)
}
impl OperationType{
    pub fn from_str(s:&str,p:Vec<String>)->Self{
        match s {
            "cmd"=>Cmd(p[0].clone()),
            "workspace"=>Workspace(p[0].clone()),
            "movefile"=>MoveFile(p[0].clone(),p[1].clone()),
            _=>UnKnown(s.to_string(),p)
        }
    }
}

impl Operation{
    pub fn new()->Self{
        Self{ op_type: OperationType::UnKnown("".to_string(),vec![]) }
    }
    pub fn from_str(s:&str,p:Vec<String>)->Self{
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
            MoveFile(source,target)=>{
                move_file(source,target,ctx).await
            }
            UnKnown(_,_) => {}
        }
    }
}