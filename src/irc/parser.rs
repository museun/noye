use super::{Command, Prefix};
use anyhow::Context as _;

pub struct Parser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    pub const fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    pub fn prefix(&mut self) -> Option<Prefix> {
        let input = &self.input[self.pos..];
        if input.starts_with(':') {
            let pos = input.find(' ')?;
            self.pos += pos + 1;
            return Prefix::parse(&input[..pos]);
        }
        None
    }

    pub fn command(&mut self) -> anyhow::Result<Command> {
        let input = &self.input[self.pos..];
        let pos = input
            .find(' ')
            .ok_or_else(|| anyhow::anyhow!("command not found"))
            .with_context(|| format!("input: {}", input.escape_debug()))?;

        self.pos += pos + 1;
        let cmd = match &input[..pos] {
            "433" => Command::NickCollision,
            "001" => Command::Ready,
            "PING" => Command::Ping,
            "INVITE" => Command::Invite,
            "JOIN" => Command::Join,
            "PART" => Command::Part,
            "QUIT" => Command::Quit,
            "NICK" => Command::Nick,
            "PRIVMSG" => Command::Privmsg,
            s => s
                .parse::<u16>()
                .map(Command::Numeric)
                .unwrap_or_else(|_| Command::Unknown(s.into())),
        };
        Ok(cmd)
    }

    pub fn args(&mut self) -> Vec<String> {
        let input = &self.input[self.pos..];
        let pos = input.find(':').unwrap_or_else(|| input.len());
        self.pos += pos + 1;
        input[..pos]
            .split_whitespace()
            .map(|s| s.trim())
            .map(ToString::to_string)
            .collect()
    }

    pub fn data(&mut self) -> Option<String> {
        self.input
            .get(self.pos..)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
    }
}
