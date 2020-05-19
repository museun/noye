use super::{parser::Parser, Command, Message, Prefix};

#[derive(Debug)]
pub struct RawMessage {
    pub prefix: Option<Prefix>,
    pub command: Command,
    pub args: Vec<String>,
    pub data: Option<String>,
}

impl RawMessage {
    pub fn parse(input: &str) -> anyhow::Result<Self> {
        if !input.ends_with("\r\n") {
            anyhow::bail!("message did not contain a \\r\\n");
        }

        let input = &input.trim_start_matches(' ')[..input.len() - 2];
        if input.is_empty() {
            anyhow::bail!("message was empty after trimming")
        }

        let mut parser = Parser::new(input);
        Ok(Self {
            prefix: parser.prefix(),
            command: parser.command()?,
            args: parser.args(),
            data: parser.data(),
        })
    }

    pub fn into_message(self) -> Message {
        Message {
            sender: match self.prefix.unwrap() {
                Prefix::User { nick, .. } => nick,
                _ => panic!("invalid message kind"),
            },
            channel: { self.args }.remove(0),
            data: self.data.unwrap(),
        }
    }
}
