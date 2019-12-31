use super::{handler::HandlerKind, Event};
use crate::irc::{Message, Target};
use crate::{config::Config, matches::Matches};

use std::sync::Arc;

pub struct Command<'a> {
    command: &'a str,
    args: Option<&'a str>,
}
impl<'a> Command<'a> {
    pub fn command(&self) -> &'a str {
        self.command
    }
    pub fn args(&self) -> anyhow::Result<&'a str> {
        self.args
            .ok_or_else(|| anyhow::anyhow!("no args found for: {}", self.command))
    }
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

    pub(crate) fn kind(&self) -> Box<str> {
        self.kind.name()
    }

    pub fn target(&self) -> Option<Target<'_>> {
        self.message.target()
    }

    pub fn target_channel(&self) -> anyhow::Result<&str> {
        match self.message.target() {
            Some(Target::Channel(s)) => Ok(s),
            _ => anyhow::bail!("no target channel"),
        }
    }

    pub fn target_private(&self) -> anyhow::Result<&str> {
        match self.message.target() {
            Some(Target::Private(s)) => Ok(s),
            _ => anyhow::bail!("no target user"),
        }
    }

    pub fn nick(&self) -> anyhow::Result<&str> {
        self.message
            .nick()
            .ok_or_else(|| anyhow::anyhow!("no nick found on message"))
    }

    pub fn arg(&self, nth: usize) -> anyhow::Result<&str> {
        self.message
            .args
            .get(nth)
            .as_ref()
            .map(|s| s.as_str())
            .ok_or_else(|| anyhow::anyhow!("no arg at pos:{}", nth))
    }

    pub fn command(&self) -> anyhow::Result<Command<'_>> {
        use super::handler::match_command;
        if let HandlerKind::Command { command } = &*self.kind {
            if let Some((command, args)) = match_command(command, self.data()?) {
                return Ok(Command { command, args });
            }
        }
        anyhow::bail!("command not found")
    }

    pub fn auth_reply(&self) {
        unimplemented!();
        // #[derive(crate::bot::prelude::Template, Debug)]
        // #[parent("user_error")]
        // enum Output {
        //     NotOwner,
        // }

        // use super::IntoResponse as _;
        // Output::NotOwner.into_response(self.clone())
    }

    pub fn check_auth(&self) -> bool {
        if let Ok(nick) = self.nick() {
            return self.config.irc_config.owners.iter().any(|d| d == nick);
        }
        false
    }
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

    pub fn mock_context_msg(msg: Message) -> Self {
        Self {
            config: Default::default(),
            event: msg.command.clone(),
            message: Arc::new(msg),
            matches: Default::default(),
            kind: Default::default(),
        }
    }

    pub fn mock_context(data: impl ToString) -> Self {
        Self::make_context(data, Default::default())
    }
}
