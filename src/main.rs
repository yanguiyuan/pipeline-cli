mod ast;
mod lexer;
mod parser;
mod token;
mod builtin;

use std::error::Error;
use std::fs;
use clap::{Args, Parser, Subcommand};
use crate::lexer::Lexer;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

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
    List,
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
fn main() {
    let cli=Cli::parse();
    match &cli.command {
        Commands::Init(t) => {
            let home_dir = dirs::home_dir();

            match home_dir {
                Some(path) => {
                    let file_path=path.join(".pipeline").join(format!("{}.kts",t.template.clone().unwrap()));
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
                None => {
                    println!("无法获取用户根目录");
                }
            }
        }
        Commands::Run(path)=>{
            match path.path.clone() {
                Some(p)=>{println!("Run {}.",p);}
                None=>{println!("Run All.");}
            }
            let token_stream=Lexer::from_path("pipeline.kts").unwrap().tokenize().expect("Token解析失败");
            let ast=parser::Parser::from_token_stream(token_stream).generate_ast();
            match path.path.clone() {
                Some(p)=>{
                    let paths=p.split(".").map(|e|String::from(e)).collect();
                    ast.execute_script(paths);
                }
                None=>{
                    ast.execute_script(vec![]);
                }
            }
        }
        Commands::List=>{
            let token_stream=Lexer::from_path("pipeline.kts").unwrap().tokenize().expect("Token解析失败");
            let ast=parser::Parser::from_token_stream(token_stream).generate_ast();
            ast.walk();
        }
        Commands::Template(args)=>{
            if let Some(add)=&args.add{
                let home_dir = dirs::home_dir();
                let mut file=File::open(Path::new("pipeline.kts")).expect("无法打开文件");
                let mut file_content=String::new();
                file.read_to_string(&mut file_content).expect("文件内容读取失败");
                match home_dir {
                    Some(path) => {
                        let file_path=path.join(".pipeline").join(format!("{}.kts",add));
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
                    }
                    None => {
                        println!("无法获取用户根目录");
                    }
                }

                return
            }
            if let Some(remove)=&args.remove{
                let home_dir = dirs::home_dir().expect("无法获取用户根目录");
                let file_path=home_dir.join(".pipeline").join(format!("{}.kts",remove));
                match fs::remove_file(file_path){
                    Ok(())=>println!("{}.kts has been successfully removed.",remove),
                    Err(e)=>println!("removed failed.")
                }
                return
            }
            let home_dir = dirs::home_dir();
            match home_dir {
                Some(path) => {
                    let dir_path=path.join(".pipeline");
                    if let Ok(entries) = fs::read_dir(dir_path) {
                        for entry in entries {
                            if let Ok(entry) = entry {
                                let file_name = entry.file_name();
                                if let Some(name) = file_name.to_str() {
                                    println!("{}", name);
                                }
                            }
                        }
                    } else {
                        println!("无法打开目录");
                    }
                }
                None => {
                    println!("无法获取用户根目录");
                }
            }
        }
    }
}