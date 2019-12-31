use super::*;
use anyhow::Result;

#[derive(Clone, Debug)]
pub enum Target<'a> {
    Channel(&'a str),
    Private(&'a str),
}

#[derive(Clone, Debug)]
pub struct Message {
    pub prefix: Option<Prefix>,
    pub command: Command,
    pub args: Vec<String>,
    pub data: Option<String>,
}

#[cfg(test)]
impl Default for Message {
    fn default() -> Self {
        Self {
            prefix: Some(Default::default()),
            command: Command::Numeric(0),
            args: vec![],
            data: None,
        }
    }
}

impl Message {
    pub fn data(&self) -> Option<&str> {
        self.data.as_ref().map(|s| s.as_str())
    }

    pub fn nick(&self) -> Option<&str> {
        self.prefix.as_ref().and_then(|s| match s {
            Prefix::User { nick, .. } => Some(nick.as_str()),
            _ => None,
        })
    }

    pub fn target(&self) -> Option<Target<'_>> {
        // TODO this is really, really not the spec but it'll work for this limited use
        match self.command {
            Command::Privmsg => self.args.first().and_then(|s| {
                if s.starts_with('#') {
                    Target::Channel(s.as_str()).into()
                } else {
                    self.nick().map(Target::Private)
                }
            }),
            _ => None,
        }
    }

    pub(super) fn parse(input: &str) -> Result<Self> {
        if !input.ends_with("\r\n") {
            anyhow::bail!("message did not contain a \\r\\n");
        }
        let input = &input.trim_start_matches(' ')[..input.len() - 2];
        if input.is_empty() {
            anyhow::bail!("message was empty after trimming");
        }

        let mut parser = Parser::new(input);
        Ok(Self {
            prefix: parser.prefix(),
            command: parser.command()?,
            args: parser.args(),
            data: parser.data(),
        })
    }
}
