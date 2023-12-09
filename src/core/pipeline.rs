use std::fmt::{Display, Formatter};
use std::process::Command;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::context::{AppContext, Context, EmptyContext, ValueContext};
use crate::core::pipeline::PipelineContextValue::{ JoinSet, TaskRef};
use crate::core::task::{Task, TaskType};

#[derive(Debug,Clone)]
pub enum PipelineContextValue{
    AppCtx(Arc<RwLock<AppContext<String>>>),
    TaskRef(Arc<Task>),
    RootRef(Arc<PipelineRoot>),
    JoinSet(Arc<RwLock<tokio::task::JoinSet<()>>>)
}

#[derive(Debug,Clone)]
pub struct PipelineRoot{
    pipelines:Vec<Pipeline>,
    path:Vec<String>
}
#[derive(Debug,Clone)]
pub struct Pipeline{
    name:String,
    tasks:Vec<Arc<Task>>
}

impl Pipeline{
    pub fn new(name:&str)->Self{
        Self{ name: name.to_string(), tasks: vec![] }
    }
    pub fn add_task(&mut self, task: Task){
        self.tasks.push(Arc::new(task))
    }
}
impl Display for PipelineRoot{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f,"{:#?}",self.pipelines)
    }
}

impl PipelineRoot{
    pub fn new()->Self{
        Self{ pipelines: vec![], path: vec![] }
    }
    pub fn add_pipeline(&mut self,p:Pipeline){
        self.pipelines.push(p)
    }
    pub fn set_path(&mut self,paths:Vec<String>){
        self.path=paths;
    }
    pub fn list(&self){
        self.pipelines.iter().for_each(|p|{
            println!("{}",p.name);
            p.tasks.iter().for_each(|t|{
                println!("   ╰─▶{}",t.get_name())
            })
        })
    }
    pub async fn flush(&self){
        clear_screen();
        print_path(&self.path);
        for p in &self.pipelines{
            if self.path.len()>0&&p.name!=self.path[0]{
                continue
            }
            println!("╰─▶Running Pipeline {}",p.name);
            for t in p.clone().tasks{
                if self.path.len()>1&&t.get_name()!=self.path[1]{
                    continue
                }
                match t.get_type() {
                    TaskType::Step => {
                        println!("   ╰─▶Running Step {}",t.get_name())
                    }
                    TaskType::Parallel => {
                        println!("   ╰─▶Running Parallel {}",t.get_name())
                    }
                    TaskType::UnDefine => {
                        println!("   \x1b[31m╰─▶Running Undefined Task {},it will do nothing\x1b[0m",t.get_name())
                    }
                }
                t.output_log().await;
            }
        }
    }
    pub async fn execute(&self, ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>){
        for p in self.pipelines.clone(){
            if !(self.path.len()>0&&p.name!=self.path[0]){
                let ctx=Arc::new(RwLock::new(ValueContext::with_value(ctx.clone(),"join_set",JoinSet(Arc::new(RwLock::new(tokio::task::JoinSet::new()))))));
                for t in p.tasks.clone(){
                    if !(self.path.len()>1&&t.get_name()!=self.path[1]){
                        let ctx=Arc::new(RwLock::new(ValueContext::with_value(ctx.clone(),"task_ref",TaskRef(t.clone()))));
                        t.execute(ctx).await;
                    }

                }
                if let JoinSet(j)=ctx.read().await.value("join_set").await.unwrap(){
                   while let Some(r)=j.write().await.join_next().await{
                       r.expect("错误");
                   }
                };
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
fn print_path(path:&Vec<String>){
    match path.len() {
        0=>{
            println!("Run All:")
        }
        1=>{
            println!("Run {}:",path.get(0).unwrap())
        }
        2=>{
            println!("Run {}.{}",path.get(0).unwrap(),path.get(1).unwrap())
        }
        _ => {
            panic!("路径错误")
        }
    }
}