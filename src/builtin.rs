use std::process::{Command,Stdio};
use std::io::{BufRead, Read};
use encoding_rs::*;
pub fn cmd(work_path:&'static str,command:&String,blank:&'static str){
    let mut child = Command::new("powershell")
        .current_dir(work_path)
        .args(&["/C", command])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn() // 执行命令，并获取输出结果
        .expect("执行命令失败");
    if let Some(ref mut stdout) = child.stdout {
        // let reader = BufReader::new(stderr);
        let mut buffer = Vec::new();
        // 读取整个 stderr 流到 buffer
        stdout.read_to_end(&mut buffer).unwrap();
        // 使用 GBK 解码器将字节解码为 UTF-8
        let (cow, _encoding_used, had_errors) = GBK.decode(&buffer);

        if had_errors {
            // 如果出现解码错误，可以按照需要进行处理
            eprintln!("{}\x1b[31m╰─▶[Sys-Err]:Decoding error.\x1b[0m",blank);
        }
        for line in cow.lines() {
            println!("{}\x1b[32m╰─▶[Output]: {}\x1b[0m",blank ,line);
        }
    }
    if let Some(ref mut stderr) = child.stderr {
        // let reader = BufReader::new(stderr);
        let mut buffer = Vec::new();
        // 读取整个 stderr 流到 buffer
        stderr.read_to_end(&mut buffer).unwrap();

        // 使用 GBK 解码器将字节解码为 UTF-8
        let (cow, _encoding_used, had_errors) = GBK.decode(&buffer);

        if had_errors {
            // 如果出现解码错误，可以按照需要进行处理
            eprintln!("{}\x1b[31m╰─▶[Sys-Err]:Decoding error.\x1b[0m",blank);
        }

        for line in cow.lines() {
            println!("{}\x1b[31m╰─▶[Err]: {}\x1b[0m",blank ,line);
        }
    }

    // 等待子进程结束
    let status = child.wait().expect("Failed to wait on child");

}
