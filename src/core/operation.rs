use std::sync::Arc;
use tokio::sync::RwLock;
use crate::builtin::{cmd, copy, move_file, replace, workspace};
use crate::context::Context;
use crate::core::operation::OperationType::{Cmd, Copy, Move, Replace, UnKnown, Workspace};
use crate::core::pipeline::PipelineContextValue;

#[derive(Debug,Clone)]
pub struct Operation{
    op_type:OperationType
}
#[derive(Debug,Clone)]
pub enum  OperationType{
    Cmd(String),
    Workspace(String),
    Move(String,String),
    Copy(String,String),
    Replace(String,String,String),
    UnKnown(String,Vec<String>)
}
impl OperationType{
    pub fn from_str(s:&str,p:Vec<String>)->Self{
        match s {
            "cmd"=>Cmd(p[0].clone()),
            "workspace"=>Workspace(p[0].clone()),
            "move"=>Move(p[0].clone(),p[1].clone()),
            "replace"=>Replace(p[0].clone(),p[1].clone(),p[2].clone()),
            "copy"=>Copy(p[0].clone(),p[1].clone()),
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
            Move(source,target)=>{
                move_file(source,target,ctx).await
            }
            Copy(source,target)=>{
                copy(ctx,source,target).await
            }
            Replace(path,regex,replace_content)=>{
                replace(ctx,path,regex,replace_content).await
            }
            UnKnown(_,_) => {}
        }
    }
}