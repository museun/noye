use super::{handler::HandlerKind, Event};
use crate::irc::{Message, Target};
use crate::{config::Config, matches::Matches};

use std::sync::Arc;

pub struct Command<'a> {
    pub command: &'a str,
    pub args: Option<&'a str>,
}

/// Handler context passed to each running handler
#[derive(Clone)]
pub struct Context {
    config: Arc<Config>,
    event: Event,
    message: Arc<Message>,
    matches: Arc<Matches>,
    kind: Arc<HandlerKind>,
}

impl Context {
    pub(super) fn new(message: Message, config: Arc<Config>) -> Self {
        Self {
            config,
            event: message.command.clone(),
            message: Arc::new(message),
            matches: Default::default(),
            kind: Default::default(),
        }
    }

    pub(super) fn with_kind(self, kind: Arc<HandlerKind>) -> Self {
        Self { kind, ..self }
    }

    pub(super) fn with_matches(self, matches: Matches) -> Self {
        Self {
            matches: Arc::new(matches),
            ..self
        }
    }
}

impl Context {
    pub fn config(&self) -> &Config {
        &*self.config
    }

    pub fn matches(&self) -> &Matches {
        &*self.matches
    }

    pub fn data(&self) -> anyhow::Result<&str> {
        self.message
            .data()
            .ok_or_else(|| anyhow::anyhow!("expected data attached"))
    }

    pub fn event(&self) -> &Event {
        &self.event
    }

    pub fn target(&self) -> Option<Target<'_>> {
        self.message.target()
    }

    pub fn target_channel(&self) -> Option<&str> {
        match self.message.target()? {
            Target::Channel(s) => Some(s),
            _ => None,
        }
    }

    pub fn target_private(&self) -> Option<&str> {
        match self.message.target()? {
            Target::Private(s) => Some(s),
            _ => None,
        }
    }

    pub fn nick(&self) -> Option<&str> {
        self.message.nick()
    }

    pub fn arg(&self, nth: usize) -> Option<&str> {
        self.message.args.get(nth).as_ref().map(|s| s.as_str())
    }

    pub fn command(&self) -> Option<Command<'_>> {
        use super::handler::match_command;
        if let HandlerKind::Command { command } = &*self.kind {
            return match_command(command, self.data().ok()?)
                .map(|(command, args)| Command { command, args });
        }
        None
    }

    pub fn auth_reply(&self) -> super::Response {
        use super::IntoResponse as _;
        Output::NotOwner.into_response(self.clone())
    }

    pub fn check_auth(&self) -> bool {
        if let Some(nick) = self.nick() {
            return self.config.irc_config.owners.iter().any(|d| d == nick);
        }
        false
    }
}

#[derive(crate::bot::prelude::Template, Debug)]
#[parent("user_error")]
enum Output {
    NotOwner,
}

#[cfg(test)]
impl Context {
    fn make_context(data: impl ToString, matches: Matches) -> Self {
        Self {
            config: Arc::new(Config::default()),
            event: Event::Privmsg,
            message: Arc::new(Message {
                prefix: None,
                command: Event::Privmsg,
                args: vec!["#museun".into()],
                data: Some(data.to_string()),
            }),
            matches: Arc::new(matches),
            kind: Default::default(),
        }
    }

    pub fn config_mut(&mut self) -> &mut Config {
        Arc::make_mut(&mut self.config)
    }

    pub fn mock_context_regex(data: impl ToString, re: impl AsRef<str>) -> Self {
        let re = regex::Regex::new(re.as_ref()).expect("valid regex");
        let data = data.to_string();
        let matches = Matches::from_regex(&re, &data);
        Self::make_context(data, matches)
    }

    pub fn mock_context(data: impl ToString) -> Self {
        Self::make_context(data, Default::default())
    }
}
