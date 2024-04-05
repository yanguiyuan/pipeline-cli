

mod v1;
mod builtin;
mod context;
mod engine;
mod logger;
mod module;
mod error;

use std::any::Any;
use std::{fs, thread};
use clap::{Args, Parser, Subcommand};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use crate::context::Context;
use crate::engine::{PipelineEngine};
use crate::error::{PipelineError, PipelineResult};
use crate::module::Module;


#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli{
    #[command(subcommand)]
    command: Commands,

}
#[derive(Subcommand)]
enum Commands {
    /// init project script.
    Init(InitArgs),
    /// Run special project script.
    Run(RunArgs),
    /// List tasks which can execute.
    // List,
    Template(TemplateArgs),
    // using special layout to generate project struct.
    Layout(LayoutArgs)
}
#[derive(Args)]
struct RunArgs{
    path:Option<String>
}
#[derive(Args)]
struct LayoutArgs{
    layout:Option<String>
}
#[derive(Args)]
struct InitArgs{
    ///Specifies the template to be initialized.
    #[arg(short, long, value_name = "default")]
    template:Option<String>
}
#[derive(Args)]
struct TemplateArgs{
    ///Specifies the template to be initialized.
    #[arg(short, long, value_name = "default")]
    add:Option<String>,
    #[arg(short, long, value_name = "default")]
    remove:Option<String>
}
fn handle_init(t:&str){
    let home_dir = dirs::home_dir().expect("无法获取用户根目录");
    let file_path=home_dir.join(".pipeline").join(format!("{}.kts",t));
    let mut file = match File::open(&file_path) {
        Ok(file) => file,
        Err(err) => {
            println!("无法打开文件: {:?}", err.to_string());
            return;
        }
    };
    let mut file_content = String::new();
    if let Err(err)= file.read_to_string(&mut file_content) {
        println!("无法读取文件内容: {:?}", err);
    }
    let file_path = PathBuf::from("pipeline.kts");
    let mut file = match File::create(&file_path) {
        Ok(file) => file,
        Err(err) => {
            println!("无法创建文件: {:?}", err);
            return;
        }
    };
    match file.write_all(file_content.as_bytes()) {
        Ok(_) => {
            println!("Successful initialization!");
        }
        Err(err) => {
            println!("Failed:无法写入文件内容: {:?}", err);
        }
    }
}
fn handle_pipeline_err(e:PipelineError){
    match e {
        PipelineError::FunctionUndefined(name) => {
            println!("\x1b[31m[Error]:eval failed,function {name} undefined.\x1b[0m");
            if name=="pipeline"{
                println!("\x1b[31m[Error]:You can try to add 'import pipe' to use pipeline.\x1b[0m")
            }
        }
        PipelineError::VariableUndefined(name) => {
            println!("\x1b[31m[Error]:eval failed,variable \"{name}\" undefined.\x1b[0m")
        }
        PipelineError::ExpectedType(s) => {
            println!("\x1b[31m[Error]:eval failed,expected type \"{s}\".\x1b[0m")
        }
        PipelineError::UnexpectedType(s)=>{
            println!("\x1b[31m[Error]:eval failed,unexpected type \"{s}\".\x1b[0m")
        }
        PipelineError::UnexpectedToken(t)=> {
            println!("\x1b[31m[Error]:parse failed,due to an unexpected token \"{t:?}\".\x1b[0m")
        }
        PipelineError::UnusedKeyword(k)=>{
            println!("\x1b[31m[Error]:parse failed,due to an reserved and unimplemented keyword \"{k}\".\x1b[0m")
        }
        PipelineError::UnknownModule(m)=>{
            println!("\x1b[31m[Error]:unknown module \"{m:}\".\x1b[0m")
        }
        PipelineError::UndefinedOperation(msg)=>{
            println!("\x1b[31m[Error]:undefined operation \"{msg:}\".\x1b[0m")
        }
    }
}

fn cli(){
    let cli=Cli::parse();
    match &cli.command {
        Commands::Layout(s)=>{
            match &s.layout {
                None => {}
                Some(layout_name) => {
                    let mut engine=PipelineEngine::default();
                    let layout=Module::with_layout_module();
                    engine.register_module(layout);
                    let home_dir = dirs::home_dir().expect("无法获取用户根目录");
                    let path=home_dir.join(".pipeline").join(format!("layout/{}/layout.kts",layout_name));
                    let script=fs::read_to_string(path).unwrap();
                    let stmt=engine.compile_stmt_blocks(script.clone()).unwrap();
                    let background=PipelineEngine::background();
                    let r=engine.eval_stmt_blocks_from_ast_with_context(background,stmt);
                    match r {
                        Ok(_) => {}
                        Err(e) => {
                            handle_pipeline_err(e);
                        }
                    }
                }
            }

        }
        Commands::Init(t) => {
            handle_init(t.template.clone().unwrap().as_str());
        }
        Commands::Run(path)=>{
            let mut paths=vec![];
            if let Some(p)=path.path.clone(){
                paths=path.path.clone().unwrap().split(".").map(|s|s.to_string()).collect();
            }
            if paths.len()<2{
                paths.push("all".into());
            }
            if paths.len()<2{
                paths.push("all".into());
            }
            let mut engine=PipelineEngine::default_with_pipeline();
            let math=Module::with_math_module();
            let layout=Module::with_layout_module();
            engine.register_module(math);
            engine.register_module(layout);
            let script=fs::read_to_string("pipeline.kts").unwrap();
            let stmt=engine.compile_stmt_blocks(script.clone());
            // println!("{:#?}",stmt);
            match stmt {
                Ok(stmt) => {
                    let background=PipelineEngine::background();
                    let pipeline=paths.get(0).unwrap().as_str();
                    let global=PipelineEngine::context_with_global_state(&background);
                    //确保global能够在engine执行eval前被释放

                    let mut global=global.write().unwrap();
                    global.set_value("path_pipeline",pipeline.into());
                    let task=paths.get(1).unwrap().as_str();
                    global.set_value("path_task",task.into());
                    global.set_value("source",script.as_str().into());

                    drop(global);
                    let r=engine.eval_stmt_blocks_from_ast_with_context(background,stmt);
                    match r {
                        Ok(_) => {}
                        Err(e) => {
                            handle_pipeline_err(e);
                        }
                    }
                }
                Err(e) => {
                    handle_pipeline_err(e);
                }
            }

        }
        Commands::Template(args)=>{
            if let Some(add)=&args.add{
                let home_dir = dirs::home_dir().expect("无法获取用户根目录");
                let mut file=File::open(Path::new("pipeline.kts")).expect("无法打开文件");
                let mut file_content=String::new();
                file.read_to_string(&mut file_content).expect("文件内容读取失败");
                let file_path=home_dir.join(".pipeline").join(format!("{}.kts",add));
                let mut file = match File::create(&file_path) {
                    Ok(file) => file,
                    Err(err) => {
                        println!("无法创建文件: {:?}", err);
                        return;
                    }
                };
                match file.write_all(file_content.as_bytes()) {
                    Ok(_) => {
                        println!("Successfully added to template {}.kts!",add);
                    }
                    Err(err) => {
                        println!("Failed:无法写入文件内容: {:?}", err);
                    }
                }
                return
            }
            if let Some(remove)=&args.remove{
                let home_dir = dirs::home_dir().expect("无法获取用户根目录");
                let file_path=home_dir.join(".pipeline").join(format!("{}.kts",remove));
                match fs::remove_file(file_path){
                    Ok(())=>println!("{}.kts has been successfully removed.",remove),
                    Err(_)=>println!("removed failed.")
                }
                return
            }
            let home_dir = dirs::home_dir().expect("无法获取用户根目录");
            let dir_path=home_dir.join(".pipeline");
            let entries = fs::read_dir(dir_path).expect("无法打开目录") ;
            for entry in entries {
                let file_name = entry.unwrap().file_name();
                if let Some(name) = file_name.to_str() {
                    println!("{}", name);
                }
            }
        }
    }
}

fn main() ->PipelineResult<()>{
    cli();
    Ok(())
}
