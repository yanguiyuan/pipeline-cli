// use crate::v1::interpreter::Function;
// use std::any::Any;
// use crate::v1::types::Dynamic;
// use std::sync::Arc;
// use tokio::sync::RwLock;
// use crate::Context;
// use crate::context::PipelineContextValue;
// use std::pin::Pin;
// pub trait NativeFunction<A:'static>{
//     #[must_use]
//     fn into_function(self) -> Function;
//
// }
// macro_rules! def_register {
//     () => {
//         def_register!(imp Native ;);
//     };
//     (imp $abi:ident ; $($par:ident => $arg:expr =>$mark:ty=>$param:ty ),*) => {
//         impl<
//             FN: Fn($($param),*) -> RET + Send+Sync+Clone + 'static,
//             $($par: Any + Send + Sync + Clone+From<Dynamic>,)*
//             RET: Any + Send + Sync + Clone,
//         >  NativeFunction<($($mark,)*) > for FN where Dynamic: From<RET> {
//             fn into_function(self) -> Function {
//                 let f=self.clone();
//                 let f=Arc::new(move|ctx:Arc<RwLock<dyn Context<PipelineContextValue>>>, args:  Vec<Dynamic>|Box::pin(async move {
//                     // The arguments are assumed to be of the correct number and types!
//                     let mut drain = args.iter_mut();
//                     $(let mut $par = drain.next().unwrap(); )*
//                     // Call the function with each argument value
//                     let r = f($($arg.clone().into()),*);
//                     // Map the result
//                     Ok(Dynamic::from(r))
//                 }));
//                 Function::$abi( f)
//             }
//         }
//     };
//     ($p0:ident $(, $p:ident)*) => {
//         def_register!(imp Native ; $p0 => $p0 => $p0=>$p0  $(, $p => $p => $p=> $p )*);
//         def_register!($($p),*);
//     };
// }
//
// def_register!(A, B, C);