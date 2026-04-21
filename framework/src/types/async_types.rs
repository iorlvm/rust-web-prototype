use std::pin::Pin;

pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
pub type AsyncResult<T, E> = BoxFuture<Result<T, E>>;