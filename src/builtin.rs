use std::process::{Command,Stdio};
use std::io::{ Read};
use std::ops::Deref;
use std::sync::{Arc};
use tokio::sync::RwLock;

use encoding_rs::*;
use crate::context::Context;
use crate::core::pipeline::{PipelineContextValue};
use crate::core::pipeline::PipelineContextValue::JoinSet;


pub async fn cmd(command:&str, ctx:Arc<RwLock<dyn crate::context::Context<PipelineContextValue>>>){
    if let Some(PipelineContextValue::AppCtx(task_ctx))=ctx.read().await.value("task_ctx").await{
        let mut child = Command::new("powershell")
            .current_dir(task_ctx.read().await.value("workspace").unwrap())
            .args(&["/C", command])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn() // 执行命令，并获取输出结果
            .expect("执行命令失败");
        let flag=is_system_command(command);
        let mut stdout =child.stdout.take().expect("Can not get stderr.");
        let mut stderr =child.stderr.take().expect("Can not get stderr.");
        let a=ctx.read().await;
        if let PipelineContextValue::TaskRef(task1)=a.value("task_ref").await.unwrap(){
            if let PipelineContextValue::RootRef(root1)=ctx.read().await.value("root").await.unwrap(){
                let task2=task1.clone();
                let root2=root1.clone();
                if let JoinSet(j)=ctx.read().await.value("op_join_set").await.unwrap(){
                    let mut js=j.write().await;
                    js.spawn(async move {
                        let mut buffer = [0; 1];
                        let mut bytes =vec![];
                        while let Ok(size)=stdout.read(&mut buffer){
                            if size<=0{
                                break
                            }
                            //如果最后一个字节是中文的第一个字节
                            if buffer[0]== u8::try_from('\n').unwrap(){
                                if flag{
                                    let (cow, _encoding_used, had_errors) = GBK.decode(bytes.as_slice());
                                    if had_errors {
                                        // 如果出现解码错误，可以按照需要进行处理
                                        task1.add_err_log(String::from_utf8(bytes.clone()).unwrap().as_str()).await;
                                        root1.flush().await
                                    }
                                    for line in cow.lines() {
                                        task1.add_output_log(line).await;
                                        root1.flush().await
                                    }
                                }else{
                                    task1.add_output_log(String::from_utf8(bytes.clone()).unwrap().as_str()).await;
                                    root1.flush().await
                                }

                                bytes.clear();
                            }else{
                                bytes.append(&mut buffer.to_vec());
                            }

                        }
                        if bytes.len()!=0&&String::from_utf8(bytes.clone()).unwrap()!="\n"{
                            task1.add_output_log(String::from_utf8(bytes.clone()).unwrap().as_str()).await;
                            root1.flush().await
                        }
                    });
                    js.spawn(async move{
                        let mut buffer = [0; 1];
                        let mut bytes =vec![];
                        while let Ok(size)=stderr.read(&mut buffer){
                            if size<=0{
                                break
                            }
                            //如果最后一个字节是中文的第一个字节
                            if buffer[0]== u8::try_from('\n').unwrap() {
                                if flag {
                                    let (cow, _encoding_used, had_errors) = GBK.decode(bytes.as_slice());

                                    if had_errors {
                                        task2.add_err_log(String::from_utf8(bytes.clone()).unwrap().as_str()).await;
                                        root2.flush().await
                                    }

                                    for line in cow.lines() {
                                        task2.add_err_log(line).await;
                                        root2.flush().await
                                    }
                                }else {
                                    // step.lock().unwrap().add_err_log(String::from_utf8(bytes.clone()).unwrap().as_str());
                                    task2.add_err_log(String::from_utf8(bytes.clone()).unwrap().as_str()).await;
                                    root2.flush().await
                                }
                                bytes.clear();
                            }else{
                                bytes.append(&mut buffer.to_vec());
                            }

                        }
                        if bytes.len()!=0{
                            // println!("{}\x1b[31m╰─▶[Err]: {}\x1b[0m", blank, String::from_utf8(bytes.clone()).unwrap());
                            // step.lock().unwrap().add_err_log(String::from_utf8(bytes.clone()).unwrap().as_str());
                            task2.add_err_log(String::from_utf8(bytes.clone()).unwrap().as_str()).await;
                            root2.flush().await
                        }
                    });
                }
            }
        }
        let _ = child.wait().expect("Failed to wait for command execution");
    }
}
pub async fn workspace(path:&str, ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>){
    if let PipelineContextValue::AppCtx(task_ctx)=ctx.read().await.value("task_ctx").await.unwrap(){
        task_ctx.write().await.set_value("workspace",path.to_string());
    }

}
fn is_system_command(c: &str) ->bool{
    if c.starts_with("ls"){return true}
    if c.starts_with("mkdir"){return true}
    false
}