use super::{Context, Handler, HandlerKind, Response, WrappedHandler};
use crate::config::Config;
use crate::irc::Message;
use crate::matches::Matches;

use std::sync::Arc;

use futures::prelude::*;
use futures::stream::FuturesOrdered;
use regex::Regex;

/// An IRC event
#[doc(inline)]
pub type Event = crate::irc::Command;

/// The ***Handler*** dispatcher
///
/// # Handlers
/// Handlers are functions that takes in a [`Context`](../prelude/struct.Context.html) and returns a [`Future`](https://doc.rust-lang.org/std/future/trait.Future.html) to a [`Response`](../prelude/struct.Response.html) or [`impl IntoResponse`](../prelude/trait.IntoResponse.html)
///
/// # Example
/// ```no_run
/// async fn foo(context: Context) -> impl IntoResponse {
///     "this is a response"
/// }
///
/// async fn bar(context: Context) -> impl IntoResponse {
///     vec![
///         Response::reply(context, "hello"),
///         Response::say(context, "world"),
///     ]
/// }
/// ```
#[derive(Default, Debug)]
pub struct Dispatcher {
    commands: Vec<WrappedHandler>,
    passives: Vec<WrappedHandler>,
    listeners: Vec<WrappedHandler>,
}

impl Dispatcher {
    /// Add a command to the dispatcher
    ///
    /// # Description
    /// This takes in the command trigger string, e.g. `"!hello world"` and the ***Handler***
    ///
    /// When this command is seen, the handler will be called and its response will be processed
    pub fn command(&mut self, command: impl ToString, handler: impl Handler) {
        let command = command.to_string();
        self.commands.push(WrappedHandler::new(
            handler,
            HandlerKind::Command { command },
        ));
    }

    /// Add a passive listener to the dispatcher
    ///
    /// # Description
    /// This takes in a &str and produces a _regex_ from it. When it matches, the ***Handler*** will be called
    ///
    /// This matches the provided **regex pattern** to the user input on ***PRIVMSG***s
    ///
    /// See [`Matches`](../prelude/struct.Matches.html) on how to extract matches out of the **regex**
    pub fn passive(&mut self, pattern: impl AsRef<str>, handler: impl Handler) {
        Regex::new(pattern.as_ref())
            .map(|regex| {
                let handler = WrappedHandler::new(handler, HandlerKind::Passive { regex });
                self.passives.push(handler);
            })
            .unwrap()
    }

    /// Add an event listener to the dispatcher
    ///
    /// # Description
    /// This takes in an [`Event`](../prelude/type.Event.html) and a ***Handler***.
    ///
    /// When the Event is seen, the Handler is ran
    pub fn listener(&mut self, event: Event, handler: impl Handler) {
        self.listeners.push(WrappedHandler::new(
            handler,
            HandlerKind::Listener { event },
        ));
    }

    pub(crate) async fn dispatch(&self, message: Message, config: Arc<Config>) -> Vec<Response> {
        let Self {
            listeners,
            commands,
            passives,
            ..
        } = self;
        
        let (rx,tx) = tokio::sync::mpsc::channel::<String>(32);

        let context = Context::new(message, config);
        let mut set = FuturesOrdered::new();

        for handler in listeners
            .iter()
            .chain(commands.iter())
            .filter(|s| s.want(&context))
        {
            let kind = &handler.kind;
            let (kind, name, func) = (kind.as_str(), kind.name().to_owned(), handler.name);
            log::debug!("++ {} ({}: '{}')", func, kind, name);

            let context = context.clone().with_kind(Arc::clone(&handler.kind));

            let fut = (handler.handler)(context)
                .then(move |resp| {
                    async move {
                        log::debug!("-- {} ({}: '{}') -> {:?}", func, kind, name, resp);
                        resp
                    }
                })
                .boxed_local();

            set.push(fut);
        }

        for (handler, regex) in passives
            .iter()
            .filter(|s| s.want(&context))
            .filter_map(|s| match &*s.kind {
                HandlerKind::Passive { regex } => (s, regex).into(),
                _ => None,
            })
        {
            let kind = &handler.kind;
            let (kind, name, func) = (kind.as_str(), kind.name().to_owned(), handler.name);
            log::debug!("++ {} ({}: '{}')", func, kind, name);

            let data = context.data().unwrap();
            let matches = Matches::from_regex(regex, &data);
            let context = context
                .clone()
                .with_kind(Arc::clone(&handler.kind))
                .with_matches(matches);

            let fut = (handler.handler)(context)
                .then(move |resp| {
                    async move {
                        log::debug!("-- {} ({}: '{}') -> {:?}", func, kind, name, resp);
                        resp
                    }
                })
                .boxed_local();

            set.push(fut);
        }

        set.collect().await
    }
}
