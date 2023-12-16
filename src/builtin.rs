use std::process::{Command,Stdio};
use std::io::{ Read};
use std::ops::Deref;
use std::path::Path;
use std::sync::{Arc};
use tokio::sync::RwLock;

use encoding_rs::*;
use regex::Regex;
use tokio::fs;
use crate::context::{AppContext, Context};
use crate::core::pipeline::{PipelineContextValue, PipelineRoot};
use crate::core::pipeline::PipelineContextValue::JoinSet;
use crate::core::task::Task;


pub async fn cmd(command:&str, ctx:Arc<RwLock<dyn crate::context::Context<PipelineContextValue>>>){
    if let Some(PipelineContextValue::AppCtx(task_ctx))=ctx.read().await.value("task_ctx").await{
        let mut child = Command::new("powershell")
            .current_dir(task_ctx.read().await.value("workspace").unwrap())
            .args(&["/C", command])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn() // 执行命令，并获取输出结果
            .expect("执行命令失败");
        let flag1=is_system_gbk_output_command(command);
        let flag2=is_system_gbk_err_command(command);
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
                                if flag1{
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
                                if flag2 {
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
fn is_system_gbk_output_command(c: &str) ->bool{
    if c.starts_with("ls"){return true}
    if c.starts_with("mkdir"){return true}
    false
}
fn is_system_gbk_err_command(c:&str)->bool{
    if c.starts_with("ls"){return true}
    if c.starts_with("mkdir"){return true}
    if c.starts_with("move"){return true}
    false
}
async fn read_task(ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>) ->Arc<Task>{
    let c=ctx.read().await;
    if let PipelineContextValue::TaskRef(task)=c.value("task_ref").await.unwrap(){
        return task;
    }
    panic!("task 缺失")
}
async fn read_root(ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>) ->Arc<PipelineRoot>{
    let c=ctx.read().await;
    if let PipelineContextValue::RootRef(root)=c.value("root").await.unwrap(){
        return root;
    }
    panic!("root 缺失")
}
async fn read_task_ctx(ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>)-> Arc<RwLock<AppContext<String>>>{
    if let Some(PipelineContextValue::AppCtx(task_ctx))=ctx.read().await.value("task_ctx").await {
        return task_ctx;
    }
    panic!("task context 缺失")
}
async fn read_workspace(ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>)->String{
    let mut task_ctx=read_task_ctx(ctx).await;
    let mut app=task_ctx.read().await;
    let path=app.value("workspace").unwrap();
    return String::from(path);
}
pub async fn replace(ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>,source_path:&str,regex:&str,replace:&str){
    let workspace=read_workspace(ctx.clone()).await;
    let root=Path::new(workspace.as_str());
    let source = root.join(Path::new(source_path));
    let content=fs::read(source.clone()).await.expect(format!("replace失败,可能文件路径{}不正确",source_path).as_str());
    let task=read_task(ctx.clone()).await;
    let root=read_root(ctx).await;
    let re=Regex::new(regex).unwrap();
    let content=String::from_utf8(content).unwrap();
    let replace_content=re.replace_all(content.as_str(),replace);
    fs::write(source.as_path(),replace_content.as_ref()).await.unwrap();
    task.add_output_log(format!("替换匹配的`{regex}`为`{replace}`成功！").as_str()).await;
    root.flush().await
}
pub async fn copy(ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>,source_path:&str,target_path:&str){
    let workspace=read_workspace(ctx.clone()).await;
    let root=Path::new(workspace.as_str());
    let source = root.join(Path::new(source_path));
    let target = root.join(Path::new(target_path));

    let res=fs::try_exists(target.clone()).await.expect("系统错误");
    if !res{
        fs::create_dir_all(target.parent().unwrap()).await.expect("创建目录失败");
    }
    let res = fs::copy(source.clone(), target.as_path()).await.unwrap();
}
pub async fn move_file(source_path:&str,target_path:&str,ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>){
    let workspace=read_workspace(ctx.clone()).await;
    let root=Path::new(workspace.as_str());
    let source = root.join(Path::new(source_path));
    let target = root.join(Path::new(target_path));
    let task=read_task(ctx.clone()).await;
    let root=read_root(ctx).await;
    let res=fs::try_exists(target.clone()).await.expect("系统错误");
    if !res{
        fs::create_dir_all(target.parent().unwrap()).await.expect("创建目录失败");
    }
    let res = fs::copy(source.clone(), target.as_path()).await;
    if let Ok(res)=res{
        let _=fs::remove_dir(source.as_path()).await;
    }
    match res{
        Ok(_) => {
            let r=fs::remove_file(source.as_path()).await;
            match r{
                Ok(_) => {
                    task.add_output_log(format!("文件{}移动成功!",source_path).as_str()).await;
                    root.flush().await;
                }
                Err(e) => {
                    task.add_err_log(format!("旧文件移除失败：{}",e).as_str()).await;
                    root.flush().await;
                }
            }
        }
        Err(e) => {
            task.add_err_log(format!("文件移动失败：{}",e).as_str()).await;
            root.flush().await;
        }
    }
}