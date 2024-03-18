use std::collections::HashMap;
use std::sync::{Arc};
use async_recursion::async_recursion;
use async_trait::async_trait;
use tokio::sync::RwLock;
use crate::context;
use crate::engine::PipelineResult;
use crate::logger::PipelineLogger;
use crate::v1::position::Position;
use crate::v1::types::Dynamic;

#[async_trait]
pub trait Context<T>:Send + Sync{
    async fn value(& self,key:&str)->Option<T>;
}

pub struct ValueContext<T> { parent:Arc<RwLock<dyn Context<T>>>, key:&'static str, value:T }

impl<T:Clone + std::marker::Sync> ValueContext<T> {
    pub fn with_value(ctx: Arc<RwLock<dyn Context<T>>>, key:&'static str, value:T) ->Self{
        return Self{
            parent: ctx,
            key,
            value,
        }
    }
}
#[async_trait]
impl<T:Clone + std::marker::Sync + std::marker::Send> Context<T> for ValueContext<T> {

    async fn value(&self, key:&str) ->Option<T>{
        if key==self.key{
            return Some(self.value.clone());
        }
        return self.parent.read().await.value(key).await;
    }
}
#[derive(Debug)]
pub struct EmptyContext;
impl EmptyContext{
    pub fn new()->Self{
        return Self;
    }
}
#[async_trait]
impl<T> Context<T> for EmptyContext{

    async fn value(&self, _: &str) -> Option<T> {
        None
    }
}
#[derive(Debug)]
pub struct AppContext<T>{
    map:HashMap<String,T>,
}

impl<T> AppContext<T>{
    pub fn new()->AppContext<T>{
        Self{ map: HashMap::new() }
    }
    pub fn value(&self,key:& str)->Option<&T>{
        self.map.get(key)
    }
    pub fn set_value(&mut self, key:&str, value:T){
        self.map.insert(key.into(),value);
    }
}
#[derive(Debug,Clone)]
pub enum PipelineContextValue{
    GlobalState(Arc<RwLock<AppContext<String>>>),
    JoinSet(Arc<RwLock<tokio::task::JoinSet<PipelineResult<()>>>>),
    Scope(Arc<RwLock<Scope>>),
    Env(Arc<RwLock<HashMap<String,String>>>),
    Position(Position),
    Local(String),
    Logger(Arc<RwLock<PipelineLogger>>)
}
#[derive(Debug,Clone)]
pub struct Scope{
    parent:Option<Arc<RwLock<Scope>>>,
    data:HashMap<String,Dynamic>
}

impl Scope {
    pub fn new()->Self{
        Self{data:HashMap::new(),parent:None}
    }
    pub fn set_parent(&mut self,p:Arc<RwLock<Scope>>){self.parent=Some(p)}
    #[async_recursion]
    pub async fn get(&self, key:&str) ->Option<Dynamic>{
        let r=self.data.get(key);
        match r {
            None => {
                if self.parent.is_some(){
                   let rr=self.parent.clone().unwrap();
                    let rr=rr.read().await;
                    return rr.get(key).await
                }
                return None
            }
            Some(s) => {Some(s.clone())}
        }
    }
    pub fn set(&mut self,key:&str,value:Dynamic){
        self.data.insert(key.into(),value);
    }
}
impl From<Position> for PipelineContextValue{
    fn from(value: Position) -> Self {
        PipelineContextValue::Position(value)
    }
}
impl PipelineContextValue{
    pub fn as_env(&self)->Option<Arc<RwLock<HashMap<String,String>>>>{
        match self {
            PipelineContextValue::Env(e)=>Some(e.clone()),
            _=>None
        }
    }

    pub fn as_scope(&self) ->Option<Arc<RwLock<Scope>>>{
        match self {
            PipelineContextValue::Scope(s)=>Some(s.clone()),
            _=>None
        }
    }
    pub fn as_join_set(&self)->Option<Arc<RwLock<tokio::task::JoinSet<PipelineResult<()>>>>>{
        match self {
            PipelineContextValue::JoinSet(j)=>Some(j.clone()),
            _=>None
        }
    }
    pub fn as_logger(&self)->Option< Arc<RwLock< PipelineLogger>>>{
        match self {
            PipelineContextValue::Logger(s)=>Some( s.clone()),
            _=>None
        }
    }
    pub fn as_global_state(&self)->Option<Arc<RwLock<AppContext<String>>>>{
        match self {
            PipelineContextValue::GlobalState(s)=>Some( s.clone()),
            _=>None
        }
    }
    pub fn as_position(&self)->Option<Position>{
        match self {
            PipelineContextValue::Position(s)=>Some( s.clone()),
            _=>None
        }
    }
    pub fn as_local(&self)->Option<String>{
        match self {
            PipelineContextValue::Local(s)=>Some( s.clone()),
            _=>None
        }
    }
}