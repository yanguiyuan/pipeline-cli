use std::collections::HashMap;
use std::process::Command;
use std::sync::{Arc, RwLock};
use crate::context::{Context, PipelineContextValue};
use crate::engine::PipelineEngine;
#[derive(Debug,Clone)]
pub struct PipelineLogger{
    contents:HashMap<String,Vec<String>>,
    is_parallel:bool
}

impl PipelineLogger {
    pub fn new()->Self{
        Self{contents:HashMap::new(),is_parallel:false}
    }
    pub fn set_parallel(&mut self,b:bool){
        self.is_parallel=b;
    }
    pub  fn task_out(&mut self,ctx:&Arc<RwLock<dyn Context<PipelineContextValue>>>,content:&str){
        let task_name=PipelineEngine::context_with_local(ctx,"$task_name");
        if !self.contents.contains_key(task_name.as_str()){
            if !self.is_parallel{
                println!("\x1b[32mRunning Task {}\x1b[0m",task_name);
            }
            self.contents.insert(task_name.clone(),vec![]);
        }
        let mut c=String::from("[Out]:".to_owned() +content);
        self.contents.get_mut(task_name.as_str()).unwrap().push(c.clone());
        if self.is_parallel{
            self.flush();
            return;
        }
        let c=c.replace("\r","");
        let c=c.replace("                       ","");
        println!("  ╰─▶{}",c);
    }
    pub  fn task_err(&mut self,ctx:&Arc<RwLock<dyn Context<PipelineContextValue>>>,content:&str){
        let task_name=PipelineEngine::context_with_local(ctx,"$task_name");
        if !self.contents.contains_key(task_name.as_str()){
            if !self.is_parallel{
                println!("\x1b[31mRunning Task {}\x1b[0m",task_name);
            }
            self.contents.insert(task_name.clone(),vec![]);
        }
        let mut c=String::from("\x1b[31m[Err]:".to_owned() +content+"\x1b[0m");
        self.contents.get_mut(task_name.as_str()).unwrap().push(c.clone());
        if self.is_parallel{
            self.flush();
            return;
        }
        let c=c.replace("\r","");
        let c=c.replace("                       ","");
        println!("  ╰─▶{}",c);

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