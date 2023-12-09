use std::process::{Command,Stdio};
use std::io::{BufRead, Read};
use encoding_rs::*;
use tokio::sync::oneshot;
use tokio::sync::oneshot::Sender;

pub fn cmd(work_path:&'static str,command:&String,blank:&'static str){
    let mut child = Command::new("powershell")
        .current_dir(work_path)
        .args(&["/C", command])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn() // 执行命令，并获取输出结果
        .expect("执行命令失败");
    let flag=is_system_command(command);
    let mut stdout =child.stdout.take().expect("Can not get stderr.");
    let stdout_handle = std::thread::spawn(move || {
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
                        eprintln!("{}\x1b[31m╰─▶[Sys-Err]:Decoding error.\x1b[0m", blank);
                        println!("{}\x1b[31m╰─▶[Err]: {}\x1b[0m", blank,String::from_utf8(bytes.clone()).unwrap());
                    }
                    for line in cow.lines() {
                        println!("{}\x1b[32m╰─▶[Output]: {}\x1b[0m", blank, line);
                    }
                }else{
                    println!("{}\x1b[32m╰─▶[Output]: {}\x1b[0m", blank,String::from_utf8(bytes.clone()).unwrap());
                }

                bytes.clear();
            }else{
                bytes.append(&mut buffer.to_vec());
            }

        }
        if bytes.len()!=0&&String::from_utf8(bytes.clone()).unwrap()!="\n"{
            println!("{}\x1b[32m╰─▶[Output]: {}\x1b[0m", blank, String::from_utf8(bytes.clone()).unwrap());
        }
    });

    let mut stderr =child.stderr.take().expect("Can not get stderr.");
    let stderr_handle = std::thread::spawn(move || {
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
                        // 如果出现解码错误，可以按照需要进行处理
                        // eprintln!("{}\x1b[31m╰─▶[Sys-Err]:Decoding error.\x1b[0m", blank);
                        println!("{}\x1b[31m╰─▶[Err]: {}\x1b[0m", blank, String::from_utf8(bytes.clone()).unwrap());
                    }

                    for line in cow.lines() {
                        println!("{}\x1b[31m╰─▶[Err]: {}\x1b[0m", blank, line);
                    }
                }else {
                    println!("{}\x1b[31m╰─▶[Err]: {}\x1b[0m", blank,String::from_utf8(bytes.clone()).unwrap());
                }
                bytes.clear();
            }else{
                bytes.append(&mut buffer.to_vec());
            }

        }
        if bytes.len()!=0{
            println!("{}\x1b[32m╰─▶[Err]: {}\x1b[0m", blank, String::from_utf8(bytes.clone()).unwrap());
        }
    });
    let stderr_result = stderr_handle.join().expect("The stderr thread panicked");
    let stdout_result = stdout_handle.join().expect("The stdout thread panicked");

    let status = child.wait().expect("Failed to wait for command execution");

}
fn is_system_command(c:&String)->bool{
    if c.starts_with("ls"){return true}
    false
}