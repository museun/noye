use super::{Context, Handler, HandlerKind, Noye, WrappedHandler};
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

#[derive(Default, Debug)]
pub struct Dispatcher {
    commands: Vec<WrappedHandler>,
    passives: Vec<WrappedHandler>,
    listeners: Vec<WrappedHandler>,
}

impl Dispatcher {
    pub fn command(&mut self, command: impl ToString, handler: impl Handler) {
        let command = command.to_string();
        self.commands.push(WrappedHandler::new(
            handler,
            HandlerKind::Command { command },
        ));
    }

    pub fn passive(&mut self, pattern: impl AsRef<str>, handler: impl Handler) {
        Regex::new(pattern.as_ref())
            .map(|regex| {
                let handler = WrappedHandler::new(handler, HandlerKind::Passive { regex });
                self.passives.push(handler);
            })
            .unwrap()
    }

    pub fn listener(&mut self, event: Event, handler: impl Handler) {
        self.listeners.push(WrappedHandler::new(
            handler,
            HandlerKind::Listener { event },
        ));
    }

    pub(crate) fn dispatch(
        &self,
        message: Message,
        config: Arc<Config>,
    ) -> impl Stream<Item = String> {
        let Self {
            listeners,
            commands,
            passives,
            ..
        } = self;

        let (tx, rx) = tokio::sync::mpsc::channel(32);
        // this needs to drop the channel once it drops
        let noye = Noye::new(tx);

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
            let fut = (handler.handler)(context, noye.clone()).boxed();
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

            let fut = (handler.handler)(context, noye.clone()).boxed();
            set.push(fut);
        }

        let drain = async move { while let Some(..) = set.next().await {} };
        // detach the task, they'll produce values for the cahnnel
        tokio::task::spawn(drain);
        rx
    }
}
