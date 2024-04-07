
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::{fs, io, thread};
use std::fs::File;
use std::io::{Stdin, Write};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::sync::{Arc, RwLock};
use rand::{random, Rng};
use regex::Regex;
use scanner_rust::Scanner;
use crate::builtin::{cmd, copy, move_file, replace};
use crate::context::{Context, PipelineContextValue};
use crate::engine::{PipelineEngine};
use crate::error::{PipelineError, PipelineResult};
use crate::v1;
use crate::v1::interpreter::Interpreter;

use crate::v1::parser::FnDef;
use crate::v1::types::{Dynamic, Value};

trait NativeFunction<Marker>{
    fn into_pipe_function(self) ->Arc<PipeFn>;
}
trait NativeType{}
impl NativeType for String{}
impl NativeType for i64{}
impl NativeType for f64{}

type PipeFn= dyn Send+Sync+ Fn(Arc<RwLock<dyn Context<PipelineContextValue>>>, Vec<Value>) -> PipelineResult<Value>;
#[derive(Clone)]
pub enum Function{
    Native(Arc<PipeFn>),
    Script(Box<FnDef>)
}
impl Debug for Function{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Function::Native(_) => {
                write!(f,"Native Function")
            }
            Function::Script(s) => {
                write!(f,"{s:?}")
            }
        }

    }
}
#[derive(Clone,Debug)]
pub struct Module{
    name:String,
    functions:HashMap<String,Function>
}

impl Function {
    pub fn call(&self, ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>,  args:Vec<Value>) ->PipelineResult<Value>{
        match self {
            Function::Native(n) => {
                return (*n)(ctx,args);
            }
            Function::Script(s) => {
                let mut e=PipelineEngine::default_with_pipeline();
                let share_module=PipelineEngine::context_with_shared_module(&ctx);
                let modules=PipelineEngine::context_with_modules(&ctx);
                let modules=modules.write().unwrap();
                let mut i=Interpreter::with_shared_module(share_module);
                for m in modules.iter(){
                    i.register_module(m.0.clone(),m.1.clone());
                }
                e.set_interpreter(&i);
                let scope=PipelineEngine::context_with_scope(&ctx);
                let mut scope=scope.write().unwrap();
                let mut i=0;
                for a in &s.args{
                    scope.set(a.name.as_str(),args.get(i).unwrap().clone());
                    i+=1;
                }
                drop(scope);
                e.eval_stmt_blocks_from_ast_with_context(ctx,s.body.clone())
            }
        }
    }
}
impl Module{
    pub fn new(name:impl Into<String>)->Self{
        Self{
            name:name.into(),
            functions:HashMap::new(),
        }
    }
    pub fn get_name(&self)->String{
        return self.name.clone()
    }
    pub fn merge(&mut self,module: &Module){
        for (k,v) in &module.functions{
            if !self.functions.contains_key(k){
                self.functions.insert(k.clone(),v.clone());
            }
        }
    }
    pub fn with_std_module()->Self{
        let mut std=Module::new("std");
        std.register_pipe_function("print",|ctx,args|{
            for v in args{
                let v=v.as_dynamic();
                if v.is_variable(){
                    let variable=v.as_variable().unwrap();
                    let v=PipelineEngine::context_with_dynamic(&ctx,variable.as_str());
                    match v {
                        None => {
                            return Err(PipelineError::VariableUndefined(variable))
                        }
                        Some(v) => {
                            print!("{}",v.as_dynamic());
                            continue
                        }
                    }
                }
                print!("{v}");
            }
            Ok(().into())
        });
        std.register_pipe_function("call",|ctx,args|{
            let blocks=args.get(0).unwrap().as_dynamic().as_fn_ptr().unwrap().fn_def.unwrap().body;
            let mut e=PipelineEngine::default_with_pipeline();
            let share_module=PipelineEngine::context_with_shared_module(&ctx);
            let i=Interpreter::with_shared_module(share_module);
            e.set_interpreter(&i);
            if args.len()>1{
                let scope=PipelineEngine::context_with_scope(&ctx);
                let  mut scope=scope.write().unwrap();
                if args.len()==2{
                    let v=args.get(1).unwrap().clone();
                    scope.set("it",v);
                }else{
                    let args_list=args[1..args.len()].to_vec();
                    scope.set("it",Value::Mutable(Arc::new(RwLock::new(Dynamic::Array(args_list)))));
                }
            }
            e.eval_stmt_blocks_from_ast_with_context(ctx,blocks).unwrap();
            Ok(().into())
        });
        std.register_pipe_function("println",|ctx,args|{
            for v in args{
                let v=v.as_dynamic();
                if v.is_variable(){
                    let variable=v.as_variable().unwrap();
                    let v=PipelineEngine::context_with_dynamic(&ctx,variable.as_str());
                    match v {
                        None => {
                            return Err(PipelineError::VariableUndefined(variable))
                        }
                        Some(v) => {
                            print!("{}",v.as_dynamic());
                            continue
                        }
                    }
                }
                print!("{v}");
            }
            println!();
            Ok(().into())
        });
        std.register_pipe_function("remove",|ctx,args|{
            let target=args.get(0).unwrap().as_dynamic();
            match target {
                Dynamic::Array(a)=>{
                    let a=args.get(0).unwrap().as_arc();
                    let mut a=a.write().unwrap();
                    let a=a.as_mut_array().unwrap();
                    let index=args.get(1).unwrap().as_dynamic();
                    let index=index.as_integer().unwrap();
                    a.remove(index as usize);
                }
                Dynamic::Map(_)=>{
                    let key=args.get(1).unwrap().as_dynamic();
                    let m=args.get(0).unwrap().as_arc();
                    let mut m=m.write().unwrap();
                    let m=m.as_mut_map().unwrap();
                    m.remove(&key);
                }
                t=>{
                    panic!("{} not support remove",t.type_name())
                }
            }
            Ok(().into())
        });
        std.register_pipe_function("append",|ctx,args|{
            let target=args.get(0).unwrap().as_arc();
            let mut target=target.write().unwrap();
            let target_array=target.as_mut_array().unwrap();
            for it in args.iter().skip(1){
                target_array.push(it.clone());
            }
            Ok(().into())
        });
        std.register_pipe_function("readLine",|_,args|{
            if args.len()>0{
                let c=args.get(0).unwrap().as_dynamic().as_string().unwrap();
                print!("{c}");
                io::stdout().flush().unwrap();
            }
            let mut input = String::new();
            io::stdin().read_line(&mut input).expect("无法读取输入");
            Ok(input.into())
        });
        std.register_pipe_function("len",|_,args|{
            let c=args.get(0).unwrap().as_dynamic();
            match c {
                Dynamic::String(s) => {
                    Ok((s.len() as i64).into())
                }
                Dynamic::Array(a) => {
                    Ok((a.len() as i64).into())
                }
                Dynamic::Map(m) => {
                    Ok((m.len() as i64).into())
                }
                t=>return Err(PipelineError::UnexpectedType(t.type_name()))
            }

        });
        std.register_pipe_function("type",|_,args|{
            let c=args.get(0).unwrap();
            Ok(c.as_dynamic().type_name().into())
        });
        std.register_pipe_function("clone",|_,args|{
            let c=args.get(0).unwrap();
            Ok(match c {
                Value::Immutable(i) => {
                    i.clone().into()
                }
                Value::Mutable(m) => {
                    let a=m.read().unwrap().clone();
                    Value::Mutable(Arc::new(RwLock::new(a)))
                }
                Value::Refer(r) => {
                    let a=r.upgrade().unwrap().read().unwrap().clone();
                    Value::Mutable(Arc::new(RwLock::new(a)))
                }
                _=>panic!("signal can not be cloned")
            })
        });
        std.register_pipe_function("readInt",|ctx,args|{
            if args.len()>0{
                let c=args.get(0).unwrap().as_dynamic().as_string().unwrap();
                print!("{c}");
                io::stdout().flush().unwrap();
            }
            let sc=PipelineEngine::context_with_native(&ctx,"$sc");

            let mut sc=sc.write().unwrap();
            let mut sc=sc.downcast_mut::<Scanner<Stdin>>().unwrap();
            let i =sc.next_i64().unwrap().unwrap();
            Ok(i.into())
        });
        std.register_pipe_function("readFloat",|ctx,args|{
            if args.len()>0{
                let c=args.get(0).unwrap().as_dynamic().as_string().unwrap();
                print!("{c}");
                io::stdout().flush().unwrap();
            }
            let sc=PipelineEngine::context_with_native(&ctx,"$sc");

            let mut sc=sc.write().unwrap();
            let mut sc=sc.downcast_mut::<Scanner<Stdin>>().unwrap();
            let i =sc.next_f64().unwrap().unwrap();
            Ok(i.into())
        });
        std.register_pipe_function("readString",|ctx,args|{
            if args.len()>0{
                let c=args.get(0).unwrap().as_dynamic().as_string().unwrap();
                print!("{c}");
                io::stdout().flush().unwrap();
            }
            let sc=PipelineEngine::context_with_native(&ctx,"$sc");

            let mut sc=sc.write().unwrap();
            let mut sc=sc.downcast_mut::<Scanner<Stdin>>().unwrap();
            let i =sc.next().unwrap().unwrap();
            Ok(i.into())
        });
        std.register_pipe_function("cmd",|ctx,args| {
            let c=args.get(0).unwrap().as_dynamic().as_string().unwrap();
            return cmd(c.as_str(),ctx);

        });
        std.register_pipe_function("env",|ctx,args| {
            let k=args.get(0).unwrap().as_dynamic().as_string().unwrap();
            let v=args.get(1).unwrap().as_dynamic().as_string().unwrap();
            let env=PipelineEngine::context_with_env(&ctx);;
            let mut env=env.write().unwrap();
            env.insert(k,v);
            Ok(().into())

        });
        std.register_pipe_function("workspace",|ctx,args| {
            let global=PipelineEngine::context_with_global_state(&ctx);
            let arg=args.get(0).unwrap().as_dynamic().as_string().unwrap();
            if !Path::new(arg.as_str()).exists(){
                let source=PipelineEngine::context_with_global_value(&ctx,"source");
                let pos=PipelineEngine::context_with_position(&ctx);
                let c=source.chars().collect();
                let (row,col)=pos.get_row_col(&c);
                println!("\x1b[31m  {}|{col}   {:}\x1b[0m",row+1,pos.get_raw_string(&c));
                println!("\x1b[31m[Error]:路径\"{arg}\"不存在\x1b[0m");
                exit(0);
            }
            global.write().unwrap().set_value("workspace",arg);
            return Ok(().into())
        });
        std.register_pipe_function("copy",|ctx,args| {
            let source=args.get(0).unwrap().as_dynamic().as_string().unwrap();
            let target=args.get(1).unwrap().as_dynamic().as_string().unwrap();
            copy(ctx,source.as_str(),target.as_str());
            return Ok(().into())
        });
        std.register_pipe_function("replace",|ctx,args| {
            let path=args.get(0).unwrap().as_string().unwrap();
            let regex=args.get(1).unwrap().as_string().unwrap();
            let replace_content=args.get(2).unwrap().as_string().unwrap();
            replace(ctx,path.as_str(),regex.as_str(),replace_content.as_str());
            return Ok(().into())
        });
        std.register_pipe_function("move",|ctx,args| {
            let source=args.get(0).unwrap().as_string().unwrap();
            let target=args.get(1).unwrap().as_string().unwrap();
            move_file(ctx,source.as_str(),target.as_str());
            return Ok(().into())
        });
        return std
    }
    pub fn with_math_module()->Self{
        let mut math=Module::new("math");
        math.register_pipe_function("max",|ctx,args| {
            let first=args.get(0).unwrap().as_dynamic();
            let mut max=first.convert_float().unwrap();
            for a in &args{
                let i=a.as_dynamic().convert_float().unwrap();
                if i>max{
                    max=i
                }
            }
            return Ok(max.into())
        });
        math.register_pipe_function("randomInt",|ctx,args| {
            if args.len()>0{
                let a=args[0].as_integer().unwrap();
                if args.len()>1{
                    let b=args[1].as_integer().unwrap();
                    let random_number = rand::thread_rng().gen_range(a..=b);
                    return Ok(random_number.into())
                }
                let random_number = rand::thread_rng().gen_range(0..=a);
                return Ok(random_number.into())
            }
            let i=random::<i64>();
            return Ok(i.into())
        });
        return math
    }
    pub fn with_pipe_module()->Self{
        let  mut pipe=Module::new("pipe");
        pipe.register_pipe_function("pipeline",|ctx,args| {
            let pipeline_name=args.get(0).unwrap().as_string().unwrap();
            let blocks=args.get(1).unwrap().as_dynamic().as_fn_ptr().unwrap().fn_def.unwrap().body;
            let mut e=PipelineEngine::default_with_pipeline();
            let share_module=PipelineEngine::context_with_shared_module(&ctx);
            let i=Interpreter::with_shared_module(share_module);
            e.set_interpreter(&i);
            let pipeline=PipelineEngine::context_with_global_value(&ctx,"path_pipeline");
            let ctx=PipelineEngine::with_value(ctx,"join_set",PipelineContextValue::JoinSet(Arc::new(std::sync::RwLock::new(vec![]))));
            if pipeline==pipeline_name||pipeline=="all"{
                e.eval_stmt_blocks_from_ast_with_context(ctx.clone(),blocks).unwrap();
            }
            let join_set=PipelineEngine::context_with_join_set(&ctx,"join_set");
            let mut join_set=join_set.write().unwrap();
            while !join_set.is_empty(){
               let e= join_set.pop().unwrap();
                e.join().unwrap().unwrap();
            }
            Ok(().into())
        });
        pipe.register_pipe_function("parallel",|ctx,args| {
            let pipeline_name=args.get(0).unwrap().as_string().unwrap();
            let blocks=args.get(1).unwrap().as_dynamic().as_fn_ptr().unwrap().fn_def.unwrap().body;
            let mut e=PipelineEngine::default();
            let ctx=PipelineEngine::with_value(ctx,"$env",PipelineContextValue::Env(Arc::new(std::sync::RwLock::new(HashMap::new()))));
            let pipeline=PipelineEngine::context_with_global_value(&ctx,"path_task");
            let logger=PipelineEngine::context_with_logger(&ctx,"logger");
            let logger=logger.as_logger().unwrap();
            logger.write().unwrap().set_parallel(true);
            if pipeline==pipeline_name||pipeline.as_str()=="all"{
                let join=PipelineEngine::context_with_join_set(&ctx,"join_set");
                let mut join=join.write().unwrap();
                let handle=thread::spawn(move||{
                    let ctx=PipelineEngine::with_value(ctx,"op_join_set",PipelineContextValue::JoinSet(Arc::new(RwLock::new(vec![]))));
                    let ctx=PipelineEngine::with_value(ctx,"$task_name",PipelineContextValue::Local(pipeline_name.into()));
                    e.eval_stmt_blocks_from_ast_with_context(ctx.clone(),blocks).unwrap();
                    let join_set=PipelineEngine::context_with_join_set(&ctx,"op_join_set");
                    let mut join_set=join_set.write().unwrap();
                    while !join_set.is_empty(){
                        let e= join_set.pop().unwrap();
                        e.join().unwrap().unwrap();
                    }
                    return Ok(())
                });
                join.push(handle);
            }
            Ok(().into())
        });
        pipe.register_pipe_function("step",|ctx,args| {
            let pipeline_name=args.get(0).unwrap().as_string().unwrap();
            let mut ptr=args.get(1).unwrap().as_dynamic().as_fn_ptr().unwrap();
            let mut e=PipelineEngine::default();
            let ctx=PipelineEngine::with_value(ctx,"$env",PipelineContextValue::Env(Arc::new(std::sync::RwLock::new(HashMap::new()))));
            let pipeline=PipelineEngine::context_with_global_value(&ctx,"path_task");
            if pipeline==pipeline_name||pipeline.as_str()=="all"{
                let ctx=PipelineEngine::with_value(ctx,"op_join_set",PipelineContextValue::JoinSet(Arc::new(std::sync::RwLock::new(vec![]))));
                let ctx=PipelineEngine::with_value(ctx,"$task_name",PipelineContextValue::Local(pipeline_name.into()));
                ptr.call(&mut e,ctx.clone()).unwrap();
                let join_set=PipelineEngine::context_with_join_set(&ctx,"op_join_set");
                let mut join_set=join_set.write().unwrap();
                while !join_set.is_empty(){
                    let e= join_set.pop().unwrap();
                    e.join().unwrap().unwrap();
                }
            }
            Ok(().into())
        });
        return pipe
    }
    pub fn with_layout_module()->Self{
        let  mut layout=Module::new("layout");
        layout.register_pipe_function("layout",|ctx,args|{
            let name=args.get(0).unwrap().as_string().unwrap();
            println!("\x1b[32musing layout {}",name);
            let mut ptr=args.get(1).unwrap().as_dynamic().as_fn_ptr().unwrap();
            let mut e=PipelineEngine::default();
            let share_module=PipelineEngine::context_with_shared_module(&ctx);
            let i=Interpreter::with_shared_module(share_module);
            e.set_interpreter(&i);
            let scope=PipelineEngine::context_with_scope(&ctx);
            let mut scope=scope.write().unwrap();
            let v:Arc<RwLock<HashMap<String,String>>>=Arc::new(RwLock::new(HashMap::new()));
            scope.set("layoutName",Value::Mutable(Arc::new(RwLock::new(Dynamic::String(name)))));
            drop(scope);
            ptr.call(&mut e,ctx.clone()).unwrap();
            println!("╰─▶successfully finished.\x1b[0m");
            Ok(().into())
        });
        layout.register_pipe_function("template",|ctx,args|{
            let target=args.get(0).unwrap().as_string().unwrap();
            let template=args.get(1).unwrap().as_string().unwrap();
            println!("╰─▶using template {} to generate {}.",template,target);

            let mut e=PipelineEngine::default();
            let scope=PipelineEngine::context_with_scope(&ctx);
            let mut scope=scope.write().unwrap();
            let v:Arc<RwLock<HashMap<String,String>>>=Arc::new(RwLock::new(HashMap::new()));
            scope.set("ctx",Value::Mutable(Arc::new(RwLock::new(Dynamic::Native(v.clone())))));
            drop(scope);
            let share_module=PipelineEngine::context_with_shared_module(&ctx);
            let i=Interpreter::with_shared_module(share_module);
            e.set_interpreter(&i);
            let mut ptr=args.get(2);
            // let mut ptr=args.get(2).unwrap().as_dynamic().as_fn_ptr().unwrap();
            if let Some(p)=ptr{
                let mut ptr=p.as_dynamic().as_fn_ptr().unwrap();
                ptr.call(&mut e,ctx.clone()).unwrap();
            }
            let m=v.read().unwrap();
            let layout_name=PipelineEngine::context_with_dynamic(&ctx,"layoutName").unwrap().as_string().unwrap();
            let home_dir = dirs::home_dir().expect("无法获取用户根目录");
            let template_path=home_dir.join(".pipeline").join(format!("layout/{}/{}",layout_name,template));
            let template_content=fs::read_to_string(template_path).unwrap();
            let re = Regex::new(r"\$\{([a-zA-Z_][a-zA-Z0-9_]*)\}").unwrap();
            let replaced = re.replace_all(template_content.as_str(), |caps: &regex::Captures| {
                let key=&caps[1];
                let r=m.get(key).unwrap();
                r.as_str()
            });
            let target_path=PathBuf::from(target.as_str());
            if !target_path.exists(){
                fs::create_dir_all(target_path.parent().unwrap()).unwrap();
            }
            let mut file=File::create(target).unwrap();
            file.write_all(replaced.as_bytes()).unwrap();
            Ok(().into())
        });
        layout.register_pipe_function("set",|ctx,args|{
            let hashmap=args.get(0).unwrap().as_dynamic().as_native().unwrap();
            let mut hashmap=hashmap.write().unwrap();
            let hashmap=hashmap.downcast_mut::<HashMap<String,String>>().unwrap();
            let key=args.get(1).unwrap().as_string().unwrap();
            let value=args.get(2).unwrap().as_string().unwrap();
            hashmap.insert(key,value);
            Ok(().into())
        });
        layout.register_pipe_function("folder",|ctx,args|{
            let folder_name=args.get(0).unwrap().as_string().unwrap();
            println!("╰─▶creating folder {}.",folder_name);
            let target_path=PathBuf::from(folder_name.as_str());
            if !target_path.exists(){
                fs::create_dir_all(target_path).unwrap();
            }
            Ok(().into())
        });
        return layout
    }
    pub fn register_native_function<A:'static,F>(&mut self, name:impl Into<String>, f:F )
    where F:NativeFunction<A>
    {
        self.functions.insert(name.into(),Function::Native(f.into_pipe_function()));
    }
    pub fn register_pipe_function(&mut self,name:impl Into<String>,f:impl Send+Sync+Fn(Arc<RwLock<dyn Context<PipelineContextValue>>>,Vec<Value>)->PipelineResult<Value> + 'static){
        let a: Arc<PipeFn> = Arc::new(f);
        self.functions.insert(name.into(),Function::Native(a));
    }
    pub fn register_script_function(&mut self,name:impl Into<String>,f:FnDef){
        self.functions.insert(name.into(),Function::Script(Box::new(f)));
    }
    pub fn call(&mut self,ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>,function_name:impl Into<String>,args: Vec<Value>)->PipelineResult<Value>{
        let name=function_name.into();
        let f=self.functions.get(name.clone().as_str());
        match f {
            None => {Err(PipelineError::FunctionUndefined(name))}
            Some(f) => {
                let r=f.call(ctx,args);
                return r
            }
        }
    }
    pub fn get_function(&self,name:impl Into<String>)->Option<Function>{
        let r=self.functions.get(name.into().as_str());
        match r {
            None => {None}
            Some(s) => {Some(s.clone())}
        }
    }
}
impl<
    T:Fn(A,B)->Ret + 'static + Send + Sync,A:NativeType + From<Value>,
    B:NativeType + From<Value>,
    Ret:NativeType + From<Value>>
NativeFunction<(A,B)> for T
    where Value: From<Ret>
{
    fn into_pipe_function(self) -> Arc<PipeFn> {
        Arc::new(move |ctx:Arc<RwLock<dyn Context< PipelineContextValue >>>, args:Vec< Value >|->PipelineResult<Value>{
            let mut it=args.iter();
            let a=it.next().unwrap().clone().into();
            let b=it.next().unwrap().clone().into();
            let r=self(a,b);
            return Ok(r.into())
        })
    }
}
impl <T:Fn(A) + 'static + Send + Sync,A:NativeType + From<Value>>NativeFunction<(A)> for T {
    fn into_pipe_function(self) -> Arc<PipeFn> {
        Arc::new(move|_:Arc<RwLock<dyn Context< PipelineContextValue >>>, args:Vec< Value >|->PipelineResult<Value>{
            self(args[0].clone().into());
            Ok(().into())
        })
    }
}