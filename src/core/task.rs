use std::cell::RefCell;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::context::{AppContext, Context, ValueContext};
use crate::core::operation::Operation;
use crate::core::pipeline::PipelineContextValue;
use crate::core::pipeline::PipelineContextValue::JoinSet;

#[derive(Debug)]
pub struct Task{
    name:String,
    task_type:TaskType,
    logs:RwLock<Vec<String>>,
    operations:Vec<Operation>
}
#[derive(Debug)]
pub enum TaskType{
    Step,
    Parallel,
    UnDefine
}
impl TaskType{
    pub fn from_str(s:&str)->Self{
        match s {
            "step"=>TaskType::Step,
            "parallel"=> TaskType::Parallel,
            _=> TaskType::UnDefine
        }
    }
}

impl Task{
    pub fn new(name:&str,task_type:TaskType)->Self{
        Self{ name: name.to_string(), task_type: task_type, logs:RwLock::new(vec![]), operations: vec![] }
    }
    pub fn get_type(&self)->&TaskType{
        &self.task_type
    }
    pub fn get_name(&self)->&str{
        &self.name
    }
    pub fn add_operation(&mut self,op:Operation){
        self.operations.push(op)
    }
    pub async fn add_err_log(&self, s:&str){
        self.logs.write().await.push(format!("      \x1b[31m╰─▶[err]:{}\x1b[0m",s))
    }
    pub async fn add_output_log(&self, s:&str){
        self.logs.write().await.push(format!("      \x1b[32m╰─▶[output]:{}\x1b[0m",s))
    }
    pub async fn output_log(&self){
        self.logs.read().await.iter().for_each(|l|{
            println!("{}",l)
        })
    }
    //task的execute
    pub async fn execute(&self, ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>){
        let mut task_ctx=AppContext::new();
        task_ctx.set_value("workspace","./".to_string());
        let task_ctx=RwLock::new(task_ctx);
        let task_ctx=Arc::from(task_ctx);
        let ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>=Arc::new(RwLock::new(ValueContext::with_value(ctx,"task_ctx",PipelineContextValue::AppCtx(task_ctx))));
        match self.task_type{
            TaskType::Step => {
                let ctx=Arc::new(RwLock::new(ValueContext::with_value(ctx.clone(),"op_join_set",JoinSet(Arc::new(RwLock::new(tokio::task::JoinSet::new()))))));
                for op in self.operations.clone(){
                    op.execute(ctx.clone()).await;
                }
                if let JoinSet(j)=ctx.read().await.value("op_join_set").await.unwrap(){
                    while let Some(r)=j.write().await.join_next().await{
                        r.expect("错误");
                    }
                };
            }
            TaskType::Parallel => {
                let clone=ctx.clone();
                let ops=self.operations.clone();
                if let JoinSet(j)=ctx.read().await.value("join_set").await.unwrap(){
                    j.write().await.spawn(async move {
                        let ctx=Arc::new(RwLock::new(ValueContext::with_value(clone.clone(),"op_join_set",JoinSet(Arc::new(RwLock::new(tokio::task::JoinSet::new()))))));
                        for op in ops{
                            op.execute(ctx.clone()).await;
                        }
                        if let JoinSet(j)=ctx.read().await.value("op_join_set").await.unwrap(){
                            while let Some(r)=j.write().await.join_next().await{
                                r.expect("错误");
                            }
                        };
                    });
                }

            }
            TaskType::UnDefine => {}
        }
    }
}