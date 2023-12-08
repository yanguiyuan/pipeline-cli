use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Instant;
use crate::builtin::cmd;

#[derive(Debug,Clone)]
pub struct AST{
    ast:Vec<FunctionCall>,
    ctx:Context
}
#[derive(Debug,Clone)]
struct Context{
    values:RefCell<HashMap<&'static  str,&'static  str>>
}
impl Context{
    pub fn new()->Context{
        return Context{ values: Default::default() }
    }
    pub fn set(&self,key:&'static str,value:&'static str){
        self.values.borrow_mut().insert(key,value);
    }
    pub fn get(&self,key:&'static  str)->&'static  str{
        return self.values.borrow().get(key).unwrap()
    }
}
impl AST{
    pub fn new()->Self{
        return Self{ ast: vec![], ctx: Context::new()}
    }
    pub fn add(&mut self,fc:FunctionCall){
        self.ast.push(fc)
    }
    pub fn to_rhai_script(&self)->String{
        return String::new()
    }
    pub fn execute_script(&self,path:Vec<String>){
        for fc in &self.ast{
            self.ctx.set("workspace",".");
            self.ctx.set("blank","");
            self.ctx.set("prev","");
            fc.execute(&self.ctx,&path)
        }
    }
    pub fn walk(&self){
        for fc in &self.ast{
            fc.walk(0);
        }
    }
}
#[derive(Debug,Clone)]
pub struct FunctionCall{
    name:String,
    args:Vec<Box<Argument>>
}
impl FunctionCall{
    pub fn new(name:String)->Self{
        return Self{ name, args: vec![] }
    }
    pub fn add_argument(&mut self,argument: Argument){
        self.args.push(Box::new(argument))
    }
    pub fn walk(&self,deep:i32){
        if deep==0&&self.name=="pipeline"{
            let mut pipeline_name =String::new();
            if let Argument::StringArgument(name)=self.args.get(0).unwrap().as_ref(){
                pipeline_name=name.clone();
            }
            println!("{}",pipeline_name);
        }else if deep==1&&self.name=="step"{
            let mut step_name =String::new();
            if let Argument::StringArgument(name)=self.args.get(0).unwrap().as_ref(){
                step_name=name.clone();
            }
            println!("  ╰─▶{}",step_name)
        }else{
            return;
        }
        if let Argument::ClosureArgument(closure)=self.args.get(1).unwrap().as_ref(){
            for fc in closure.clone().expressions{
                fc.walk(1);
            }
        }
    }
    pub fn execute(&self, x: &Context,path:&Vec<String>){
        match self.name.as_str() {
            "echo"=>{
                match self.args.get(0).unwrap().as_ref(){
                    Argument::StringArgument(s)=>{
                        println!("{}╰─▶[Debug]:{}",x.get("blank"),s)
                    }
                    _=>{}
                }
            }
            "pipeline"=>{

                let mut pipeline_name =String::new();
                if let Argument::StringArgument(name)=self.args.get(0).unwrap().as_ref(){
                    pipeline_name=name.clone();
                }
                if path.len()>0&&path.get(0).unwrap()!=&pipeline_name{
                    return;
                }
                println!("╰─▶Running Pipeline {pipeline_name}");
                if let Argument::ClosureArgument(closure)=self.args.get(1).unwrap().as_ref(){
                    let start_time = Instant::now();
                    x.set("blank",format!("   {}",x.get("blank")).leak());
                    for fc in closure.clone().expressions{
                        let mut path=path.clone();
                        if path.len()>0{
                            path.remove(0);
                        }
                        fc.execute(x,&path)
                    }
                    let duration = start_time.elapsed();
                    let red = "\x1b[31m";   // 红色
                    let green = "\x1b[32m"; // 绿色
                    let reset = "\x1b[0m";  // 重置颜色
                    println!("   ╰─▶Pipeline {red}『{pipeline_name}』{reset} finished in {duration:?}")
                }
            }
            "step"=>{
                let mut step_name =String::new();
                if let Argument::StringArgument(name)=self.args.get(0).unwrap().as_ref(){
                    step_name=name.clone();
                }
                if path.len()>0&&path.get(0).unwrap()!=&step_name{
                    return;
                }
                println!("{}╰─▶Running Step {step_name}",x.get("blank"));
                if let Argument::ClosureArgument(closure)=self.args.get(1).unwrap().as_ref(){
                    x.set("blank",format!("   {}",x.get("blank")).leak());
                    let start_time = Instant::now();
                    for fc in closure.clone().expressions{
                        fc.execute(x,path);

                    }
                    let duration = start_time.elapsed();
                    let red = "\x1b[31m";   // 红色
                    let green = "\x1b[32m"; // 绿色
                    let reset = "\x1b[0m";  // 重置颜色
                    println!("{}╰─▶Step {green}『{step_name}』{reset} finished in {duration:?}",x.get("blank"));
                    let old=x.get("blank");
                    x.set("blank", &old[3..]);
                }
            }
            "cmd"=>{
                if let Argument::StringArgument(cmds)=self.args.get(0).unwrap().as_ref(){
                   cmd(x.get("workspace"),cmds,x.get("blank"));
                }
            }
            "workspace"=>{
                if let Argument::StringArgument(path)=self.args.get(0).unwrap().as_ref(){
                    x.set("workspace",path.clone().leak())
                }
            }
            "parallel"=>{

            }
            _=>{}
        }
    }
}
#[derive(Debug,Clone)]
pub enum  Argument{
    StringArgument(String),
    ClosureArgument(Box<Closure>),
    FunctionCallArgument(Box<FunctionCall>)
}


#[derive(Debug,Clone)]
pub struct Closure{
    captures:Vec<Box<Argument>>,
    expressions:Vec<Box<FunctionCall>>,
}
impl Closure{
    pub fn new()->Self{
        Self{ captures: vec![], expressions: vec![] }
    }
    pub fn add_expression(&mut self,fc:FunctionCall){
        self.expressions.push(Box::new(fc))
    }
}
