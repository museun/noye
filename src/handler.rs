use crate::{Context, Responder};

use std::{future::Future, pin::Pin};

pub type AnyhowFut<'a> = Pin<Box<dyn Future<Output = anyhow::Result<()>> + 'a + Send>>;

pub trait Handler<R: Send + 'static>: Send + 'static {
    type Fut: Future<Output = anyhow::Result<()>> + Send + 'static;
    fn call(&self, state: Context, responder: R) -> Self::Fut;
}

impl<F, Fut, R> Handler<R> for F
where
    F: Fn(Context, R) -> Fut + Send + 'static,
    Fut: Future<Output = anyhow::Result<()>> + Send + 'static,
    R: Responder + Send + 'static,
{
    type Fut = AnyhowFut<'static>;
    fn call(&self, state: Context, responder: R) -> Self::Fut {
        Box::pin((self)(state, responder))
    }
}
