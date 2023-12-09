use std::cell::Cell;
use std::process::Command;
use crate::core::operation::{Operation};
use crate::core::pipeline::{Pipeline, PipelineRoot};
use crate::core::task::{Task, TaskType};

pub struct AST{
    ast:Vec<FunctionCall>,
    pos:Cell<usize>
}
fn clear_screen() {
    #[cfg(target_os = "windows")]
    Command::new("cmd")
        .args(&["/C", "cls"])
        .status()
        .unwrap();

    #[cfg(not(target_os = "windows"))]
    Command::new("clear")
        .status()
        .unwrap();
}
fn try_parse_task(f: &Box<FunctionCall>) ->Option<Task>{
    if f.name!="step"&&f.name!="parallel"{
        return None
    }
    let mut name=String::new();
    if let Argument::StringArgument(s)=f.args.get(0).unwrap().as_ref(){
        name=s.clone()
    }
    let mut task=Task::new(&name,TaskType::from_str(f.name.as_str()));
    let closure:Box<Closure>=f.args.get(1).unwrap().into();
    let mut f0=closure.expressions.get(0).unwrap();
    let mut i=0;
    while let Some(op)=try_parse_operation(f0){
        task.add_operation(op);
        i+=1;
        if i>=closure.expressions.len(){
            break
        }
        f0=closure.expressions.get(i).unwrap();

    }
    return Some(task)
}
fn try_parse_operation(f: &Box<FunctionCall>)->Option<Operation>{
    let mut name=String::new();
    if let Argument::StringArgument(s)=f.args.get(0).unwrap().as_ref(){
        name=s.clone()
    }
    return Some(Operation::from_str(&f.name,&name))
}

impl AST{
    pub fn new()->Self{
        return Self{ ast: vec![],  pos: Cell::new(0) }
    }
    pub fn add(&mut self,fc:FunctionCall){
        self.ast.push(fc)
    }
    #[allow(unused)]
    pub fn to_rhai_script(&self)->String{
        return String::new()
    }
    pub fn to_pipeline(&self)->PipelineRoot{
        let mut root=PipelineRoot::new();
        while let Some(pipeline)=self.try_parse_pipeline(){
            root.add_pipeline(pipeline);
        }
        root
    }
    fn try_parse_pipeline(&self)->Option<Pipeline>{
        if self.pos.get()>=self.ast.len(){
            return None
        }
        if let Some(fc)=self.ast.get(self.pos.get()){
            self.pos.set(self.pos.get()+1);
            if fc.name!="pipeline"{
                return None
            }
            let mut name=String::new();
            if let Argument::StringArgument(s)=fc.args.get(0).unwrap().as_ref(){
                name=s.clone()
            }
            let mut p =Pipeline::new(&name);
            if let Argument::ClosureArgument(closure)=fc.args.get(1).unwrap().as_ref(){
                let mut f=closure.expressions.get(0).unwrap();
                let mut i=0;
                while let Some(task)=try_parse_task(f){
                    p.add_task(task);
                    i+=1;
                    if i>=closure.expressions.len(){
                        break
                    }
                    f=closure.expressions.get(i).unwrap();

                }
                return Some(p)
            }


        }
        return None

    }
}
#[derive(Clone)]
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
}
#[derive(Clone)]
pub enum  Argument{
    StringArgument(String),
    ClosureArgument(Box<Closure>),
    FunctionCallArgument(Box<FunctionCall>)
}


#[derive(Clone)]
pub struct Closure{
    #[allow(unused)]
    captures:Vec<Box<Argument>>,
    expressions:Vec<Box<FunctionCall>>,
}
impl From<&Box<Argument>> for Box<Closure>{
    fn from(value: &Box<Argument>) -> Self {
        if let Argument::ClosureArgument(c)=value.as_ref(){
            return c.clone()
        }
        panic!("can not transfer")
    }
}
impl Closure{
    pub fn new()->Self{
        Self{ captures: vec![], expressions: vec![] }
    }
    pub fn add_expression(&mut self,fc:FunctionCall){
        self.expressions.push(Box::new(fc))
    }
}
