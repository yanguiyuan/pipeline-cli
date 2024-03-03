use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::context::{Context, PipelineContextValue};
use crate::engine::PipelineEngine;
#[derive(Debug,Clone)]
pub struct PipelineLogger{
    contents:HashMap<String,Vec<String>>
}

impl PipelineLogger {
    pub fn new()->Self{
        Self{contents:HashMap::new()}
    }
    pub async fn task_out(&mut self,ctx:&Arc<RwLock<dyn Context<PipelineContextValue>>>,content:&str){
        let task_name=PipelineEngine::context_with_local(ctx,"$task_name").await;
        if !self.contents.contains_key(task_name.as_str()){
            self.contents.insert(task_name.clone(),vec![]);
        }
        self.contents.get_mut(task_name.as_str()).unwrap().push(String::from("[Out]:".to_owned() +content));
        self.flush()
    }
    pub async fn task_err(&mut self,ctx:&Arc<RwLock<dyn Context<PipelineContextValue>>>,content:&str){
        let task_name=PipelineEngine::context_with_local(ctx,"$task_name").await;
        if !self.contents.contains_key(task_name.as_str()){
            self.contents.insert(task_name.clone(),vec![]);
        }
        self.contents.get_mut(task_name.as_str()).unwrap().push(String::from("\x1b[31m[Err]:".to_owned() +content+"\x1b[0m"));
        self.flush()
    }
    fn flush(&mut self){
        clear_screen();
        for (name,content) in &self.contents{

            println!("\x1b[32mRunning Task {}\x1b[0m",name);
            for c in content{
                let c=c.replace("\r","");
                let c=c.replace("                       ","");
                println!("  ╰─▶{}",c);
            }
        }
    }
}
fn clear_screen() {
    #[cfg(target_os = "windows")]
    Command::new("cmd")
        .args(&["/C", "cls"])
        .status()
        .unwrap();

    #[cfg(not(target_os = "windows"))]
    Command::new("clear")
        .status()
        .unwrap();
}