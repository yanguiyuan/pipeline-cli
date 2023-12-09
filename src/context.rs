use std::collections::HashMap;
use std::sync::{Arc};
use async_trait::async_trait;
use tokio::sync::RwLock;
#[async_trait]
pub trait Context<T>:Send + Sync{
    async fn value(&self,key:&str)->Option<T>;
}

pub struct ValueContext<T>{ parent:Arc<RwLock<dyn Context<T>>>, key:&'static str, value:T }

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
    map:HashMap<&'static str,T>,
}
impl<T> AppContext<T>{
    pub fn new()->AppContext<T>{
        Self{ map: HashMap::new() }
    }
    pub fn value(&self,key:&'static str)->Option<&T>{
        self.map.get(key)
    }
    pub fn set_value(&mut self, key:&'static str, value:T){
        self.map.insert(key,value);
    }
}
#[test]
fn test_context(){
    // let ctx=EmptyContext::new();
    // let ctx=ValueContext::with_value(ctx,"k1","Hello");
    // let ctx=ValueContext::with_value(ctx,"k2","Hello2");
    // let v1=ctx.value("k2").unwrap();
    // println!("{}",v1)
}