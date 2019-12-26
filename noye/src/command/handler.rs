use super::{Context, Event, IntoResponse, Response};

use futures::future::BoxFuture;
use regex::Regex;

use std::future::Future;
use std::sync::Arc;

pub type DynHandler = dyn (Fn(Context) -> BoxFuture<'static, Response>) + 'static + Send;
pub trait Handler: Send + 'static {
    type Fut: Future<Output = Response> + Send + 'static;
    fn call(&self, context: Context) -> Self::Fut;
}

impl<F, Fut> Handler for F
where
    F: Send + 'static,
    F: Fn(Context) -> Fut,
    Fut: Future + Send + 'static,
    Fut::Output: IntoResponse,
{
    type Fut = BoxFuture<'static, Response>;
    fn call(&self, context: Context) -> Self::Fut {
        let fut = (self)(context.clone());
        Box::pin(async move { fut.await.into_response(context) })
    }
}

fn type_name<T>(_: &T) -> &'static str {
    std::any::type_name::<T>()
}

fn trunc<'a>(s: &'a str, sep: &str) -> &'a str {
    let start = s
        .rfind(sep)
        .and_then(|pos| s[..pos].rfind(sep))
        .map(|start| start + sep.len())
        .unwrap_or_default();
    &s[start..]
}

pub struct WrappedHandler {
    pub handler: Box<DynHandler>,
    pub kind: Arc<HandlerKind>,
    pub name: &'static str,
}

impl std::fmt::Debug for WrappedHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handler")
            .field("func", &self.name)
            .field("name", &self.kind.name())
            .field("kind", &self.kind.as_str())
            .finish()
    }
}

impl WrappedHandler {
    pub fn new(handler: impl Handler, kind: HandlerKind) -> Self {
        Self {
            name: trunc(type_name(&handler), "::"),
            handler: Box::new(move |ctx| Box::pin(handler.call(ctx))),
            kind: Arc::new(kind),
        }
    }

    pub fn want(&self, context: &Context) -> bool {
        macro_rules! ensure {
            ($expr:expr) => {
                match $expr {
                    Ok(d) => d,
                    Err(..) => return false,
                }
            };
        }

        match &*self.kind {
            HandlerKind::Command { command } if *context.event() == Event::Privmsg => {
                match_command(&command, ensure!(context.data())).is_some()
            }
            HandlerKind::Passive { regex } => regex.is_match(ensure!(context.data())),
            HandlerKind::Listener { event } => event == context.event(),
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
pub enum HandlerKind {
    Command { command: String },
    Passive { regex: Regex },
    Listener { event: Event },
    Default,
}

impl Default for HandlerKind {
    fn default() -> Self {
        Self::Default
    }
}

impl HandlerKind {
    pub fn name(&self) -> Box<str> {
        use HandlerKind::*;
        match self {
            Command { command } => command.clone().into_boxed_str(),
            Passive { regex } => regex.as_str().into(),
            Listener { event } => event.as_box_str(),
            _ => "unknown".into(),
        }
    }

    pub fn as_str(&self) -> &'static str {
        use HandlerKind::*;
        match self {
            Command { .. } => "Command",
            Passive { .. } => "Passive",
            Listener { .. } => "Listener",
            _ => "Unknown",
        }
    }
}

type HeadTail<'a> = (&'a str, Option<&'a str>);

pub(super) fn match_command<'a>(command: &str, input: &'a str) -> Option<HeadTail<'a>> {
    if command.is_empty() || input.is_empty() {
        return None;
    }
    let (head, tail) = {
        let mut iter = input.splitn(2, ' ');
        (iter.next()?, iter.next())
    };
    if head != command {
        return None;
    }
    (head, tail).into()
}