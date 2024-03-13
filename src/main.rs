
mod v1;
mod builtin;
mod context;
mod engine;
mod logger;

use std::any::{Any, TypeId};
use std::fs;
use clap::{Args, Parser, Subcommand};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use crate::context::{Context};
use crate::engine::{PipelineEngine, PipelineResult};
use crate::v1::lexer::Lexer;
use crate::v1::parser::PipelineParser;


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
}
#[derive(Args)]
struct RunArgs{
    path:Option<String>
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

async fn cli(){
    let cli=Cli::parse();
    match &cli.command {
        Commands::Init(t) => {
            handle_init(t.template.clone().unwrap().as_str());
        }
        Commands::Run(path)=>{
            let mut paths=vec![];
            let f=match path.path.clone() {
                Some(p)=>{
                    paths=path.path.clone().unwrap().split(".").map(|s|s.to_string()).collect();
                }
                None=>{}
            };
            if paths.len()<2{
                paths.push("all".into());
            }
            if paths.len()<2{
                paths.push("all".into());
            }
            let mut engine=PipelineEngine::default_with_pipeline();
            let script=fs::read_to_string("pipeline.kts").unwrap();
            let stmt=engine.compile_stmt_blocks(script.clone()).unwrap();
            let background=PipelineEngine::background();
            let pipeline=paths.get(0).unwrap().as_str();
            let global=PipelineEngine::context_with_global_state(&background).await;
            //确保global能够在engine执行eval前被释放
            {
                let mut global=global.write().await;
                global.set_value("path_pipeline",pipeline.into());
                let task=paths.get(1).unwrap().as_str();
                global.set_value("path_task",task.into());
                global.set_value("source",script.as_str().into());
            }
            // println!("{:#?}",stmt);
            engine.eval_stmt_blocks_from_ast_with_context(background,stmt).await.unwrap();
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
#[tokio::main]
async fn main() ->PipelineResult<()>{
    cli().await;
    // let script=fs::read_to_string("pipeline.kts").unwrap();
    // let lexer=Lexer::from_script(script);
    // let mut parser=PipelineParser::from_token_stream(lexer.into_iter());
    // let (fn_def,pos)=parser.par
    // println!("{:#?}",fn_def);
    // let mut e=PipelineEngine::default_with_pipeline();
    // let r=e.eval_expr("3*2+2").await?;
    // println!("{r}");
    Ok(())
}