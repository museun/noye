use super::{Context, Responder};
use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

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

pub struct CommandsMap<R> {
    pub(super) map: HashMap<String, Arc<dyn Handler<R, Fut = AnyhowFut<'static>> + Send + 'static>>,
    _marker: std::marker::PhantomData<R>,
}

impl<R> Default for CommandsMap<R> {
    fn default() -> Self {
        Self {
            map: Default::default(),
            _marker: std::marker::PhantomData::default(),
        }
    }
}

impl<R: Responder + Send + 'static> CommandsMap<R> {
    pub fn add<H, F>(&mut self, cmd: impl ToString, handler: H) -> anyhow::Result<()>
    where
        H: Handler<R, Fut = F>,
        F: Future<Output = anyhow::Result<()>> + Send + 'static,
        F::Output: Send + 'static,
    {
        let cmd = cmd.to_string();
        if self.map.contains_key(&cmd) {
            anyhow::bail!("{} already exists as a command", cmd)
        }

        self.map
            .insert(cmd, Arc::new(move |state, resp| handler.call(state, resp)));

        Ok(())
    }
}

pub struct PassivesList<R> {
    pub(super) list: Vec<Arc<dyn Handler<R, Fut = AnyhowFut<'static>> + Send + 'static>>,
    _marker: std::marker::PhantomData<R>,
}

impl<R> Default for PassivesList<R> {
    fn default() -> Self {
        Self {
            list: Default::default(),
            _marker: std::marker::PhantomData::default(),
        }
    }
}

impl<R: Responder + Send + 'static> PassivesList<R> {
    pub fn add<H, F>(&mut self, handler: H)
    where
        H: Handler<R, Fut = F>,
        F: Future<Output = anyhow::Result<()>> + Send + 'static,
        F::Output: Send + 'static,
    {
        self.list
            .push(Arc::new(move |state, resp| handler.call(state, resp)))
    }
}
