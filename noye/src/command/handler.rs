use super::{Context, Event, IntoResponse, Response};

use futures::future::BoxFuture;
use regex::Regex;

use std::future::Future;
use std::sync::Arc;

#[derive(Clone)]
pub struct Noye {
    sender: tokio::sync::mpsc::Sender<String>,
}

impl Noye {
    pub(crate) fn new(sender: tokio::sync::mpsc::Sender<String>) -> Self {
        Self { sender }
    }

    pub fn raw(&mut self, data: impl ToString) {
        let _ = self.sender.try_send(data.to_string());
    }

    pub fn reply(&mut self, ctx: Context, data: impl std::fmt::Display) {
        use crate::irc::Target;
        match ctx.target() {
            Some(Target::Channel(target)) => self.raw(format!(
                "PRIVMSG {} :{}: {}",
                target,
                ctx.nick().expect("nick must be attached to message"),
                data
            )),
            Some(Target::Private(target)) => self.raw(format!("PRIVMSG {} :{}", target, data)),
            None => {
                log::warn!("cannot reply to a message without a target");
            }
        }
    }

    pub fn say(&mut self, ctx: Context, data: impl std::fmt::Display) {
        use crate::irc::Target;
        match ctx.target() {
            Some(Target::Channel(target)) | Some(Target::Private(target)) => {
                self.raw(format!("PRIVMSG {} :{}", target, data))
            }
            None => {
                log::warn!("cannot reply to a message without a target");
            }
        }
    }

    pub fn join(&mut self, data: impl std::fmt::Display) {
        self.raw(format!("JOIN {}", data))
    }

    pub fn part(&mut self, data: impl std::fmt::Display) {
        self.raw(format!("PART {} :bye", data))
    }

    pub fn nick(&mut self, data: impl std::fmt::Display) {
        self.raw(format!("NICK {}", data))
    }
}

pub type DynHandler = dyn (Fn(Noye) -> BoxFuture<'static, Response>) + 'static + Send;
pub trait Handler: Send + 'static {
    type Fut: Future<Output = Response> + Send + 'static;
    fn call(&self, noye: Noye) -> Self::Fut;
}

impl<F, Fut> Handler for F
where
    F: Send + 'static,
    F: Fn(Noye) -> Fut,
    Fut: Future + Send + 'static,
    Fut::Output: IntoResponse,
{
    type Fut = BoxFuture<'static, Response>;
    fn call(&self, noye: Noye) -> Self::Fut {
        let fut = (self)(noye.clone());
        Box::pin(async move { fut.await.into_response(noye) })
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
