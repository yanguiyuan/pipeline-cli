use std::{fs, io, thread};
use std::process::{Command, exit, Stdio};
use std::io::{ Read};
use std::io::ErrorKind::NotFound;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::thread::Thread;


use encoding_rs::*;
use regex::Regex;
use crate::context::{ Context, PipelineContextValue};
use crate::engine::{PipelineEngine};
use crate::error::PipelineResult;
use crate::v1::types::Dynamic;


pub fn cmd(command:&str, ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>)->PipelineResult<Dynamic>{
    let mut cmd="powershell";
    let mut c="/C";
    let os = std::env::consts::OS;
    if os=="linux"{
        cmd="sh";
        c="-c"
    }
    let global=PipelineEngine::context_with_global_state(&ctx);
    let workspace=global.read().unwrap();
    let workspace=workspace.value("workspace").unwrap();
    let env=PipelineEngine::context_with_env(&ctx);
    let mut env=env.write().unwrap();
    let mut child = Command::new(cmd)
        .current_dir(workspace.as_str())
        .envs(env.iter())
        .args(&[c, command])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn() // 执行命令，并获取输出结果
        .expect("执行命令失败");
    let flag1=is_system_gbk_output_command(command);
    let flag2=is_system_gbk_err_command(command);
    let mut stdout =child.stdout.take().expect("Can not get stderr.");
    let mut stderr =child.stderr.take().expect("Can not get stderr.");
    let join_set=PipelineEngine::context_with_join_set(&ctx,"op_join_set");
    let mut join_set =join_set.write().unwrap();
    let clone=ctx.clone();
    let handle=thread::spawn(move||{
        let ctx=clone.clone();
        let mut binding = PipelineEngine::context_with_logger(&ctx, "logger");
        let  logger=binding.as_logger().unwrap();
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
                        logger.write().unwrap().task_err(&ctx,String::from_utf8(bytes.clone()).unwrap().as_str());
                    }
                    for line in cow.lines() {
                        logger.write().unwrap().task_out(&ctx,line);
                    }
                }else{
                    logger.write().unwrap().task_out(&ctx,String::from_utf8(bytes.clone()).unwrap().as_str());
                }

                bytes.clear();
            }else{
                bytes.append(&mut buffer.to_vec());
            }

        }
        if bytes.len()!=0&&String::from_utf8(bytes.clone()).unwrap()!="\n"{
            logger.write().unwrap().task_out(&ctx,String::from_utf8(bytes.clone()).unwrap().as_str());
        }
        return Ok(())
    });
    join_set.push(handle);
    let handle=thread::spawn(move||{
        let binding = PipelineEngine::context_with_logger(&ctx, "logger");
        let  logger=binding.as_logger().unwrap();
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
                        logger.write().unwrap().task_err(&ctx,String::from_utf8(bytes.clone()).unwrap().as_str());
                    }

                    for line in cow.lines() {
                        logger.write().unwrap().task_err(&ctx,line);
                    }
                }else {
                    logger.write().unwrap().task_err(&ctx,String::from_utf8(bytes.clone()).unwrap().as_str());
                }
                bytes.clear();
            }else{
                bytes.append(&mut buffer.to_vec());
            }

        }
        if bytes.len()!=0{
            logger.write().unwrap().task_err(&ctx,String::from_utf8(bytes.clone()).unwrap().as_str());
        }
        return Ok(())
    });
    join_set.push(handle);

    let _ = child.wait().expect("Failed to wait for command execution");
    return Ok(Dynamic::Unit)
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

pub  fn replace(ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>, source_path:&str, regex:&str, replace:&str){
    let global=PipelineEngine::context_with_global_state(&ctx);
    let global=global.read().unwrap();
    let workspace=global.value("workspace").unwrap();
    let root=Path::new(workspace.as_str());
    let source = root.join(Path::new(source_path));
    let content=fs::read(source.clone()).expect(format!("replace失败,可能文件路径{}不正确",source_path).as_str());
    let re=Regex::new(regex).unwrap();
    let content=String::from_utf8(content).unwrap();
    let replace_content=re.replace_all(content.as_str(),replace);
    fs::write(source.as_path(),replace_content.as_ref()).unwrap();
}


pub  fn copy_all(source:&Path,target:&Path)->io::Result<()>{
    if !source.exists(){
        return Err(io::Error::new(NotFound,format!("{}不存在",source.to_str().unwrap())));
    }
    if !target.exists(){
        fs::create_dir_all(target.parent().unwrap())?;
    }
    if source.is_dir(){
        for entry in source.read_dir().expect("目录读取失败") {
            if let Ok(entry) = entry {
                copy_all(entry.path().as_path(),target.join(entry.path().as_path().file_name().unwrap()).as_path())?;
            }
        }
    }else{
        fs::copy(source,target)?;
    }
    return Ok(())
}

// fn test_copy(){
//     copy_all(Path::new("test/x"),Path::new("test/s")).await.unwrap();
// }
pub fn copy(ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>, source_path:&str, target_path:&str){
    let global=PipelineEngine::context_with_global_state(&ctx);
    let global=global.read().unwrap();
    let workspace=global.value("workspace").unwrap();
    let root=Path::new(workspace.as_str());
    let source = root.join(Path::new(source_path));
    let target = root.join(Path::new(target_path));
    let res=copy_all(source.as_path(),target.as_path());
    if let Err(e) = res {
        println!("\x1b[31m[Error]:文件复制失败:{}\x1b[0m",e);
        exit(0);
    }
}
pub  fn move_file( ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>,source_path:&str, target_path:&str){
    let global=PipelineEngine::context_with_global_state(&ctx);
    let global=global.read().unwrap();
    let workspace=global.value("workspace").unwrap();
    let root=Path::new(workspace.as_str());
    let source = root.join(Path::new(source_path));
    let target = root.join(Path::new(target_path));
    let res = copy_all(source.as_path(), target.as_path());
    match res{
        Ok(_) => {
            let r=fs::remove_dir_all(source.as_path());
            match r{
                Ok(_) => {
                }
                Err(e) => {
                    println!("\x1b[31m[Error]:旧文件移除失败:{}\x1b[0m",e);
                    exit(0);
                }
            }
        }
        Err(e) => {
            println!("\x1b[31m[Error]:文件移动失败:{}\x1b[0m",e);
            exit(0);
        }
    }
}