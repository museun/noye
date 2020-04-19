use anyhow::Context as _;

#[derive(Clone, Debug)]
pub struct Message {
    pub sender: String,
    pub channel: String,
    pub data: String,
}

pub enum Command {
    Privmsg,
    Ping,
    Ready,
    Quit,
    Join,
    Part,
    Invite,
    Nick,
    NickCollision,
    Numeric(u16),
    Unknown(Box<str>),
}

pub struct Irc {
    pub prefix: Option<Prefix>,
    pub command: Command,
    pub args: Vec<String>,
    pub data: Option<String>,
}

#[derive(Clone, Debug)]
pub enum Prefix {
    User {
        nick: String,
        user: String,
        host: String,
    },
    Server {
        host: String,
    },
}

impl Prefix {
    fn parse(input: &str) -> Option<Self> {
        if !input.starts_with(':') {
            return None;
        }

        let input = &input[1..];
        input
            .find('!')
            .map(|pos| {
                input.find('@').map(|at| Self::User {
                    nick: input[..pos].to_string(),
                    user: input[pos + 1..at].to_string(),
                    host: input[at + 1..].to_string(),
                })
            })
            .flatten()
            .or_else(|| {
                Some(Self::Server {
                    host: input.to_string(),
                })
            })
    }
}

impl Irc {
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

struct Parser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    const fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn prefix(&mut self) -> Option<Prefix> {
        let input = &self.input[self.pos..];
        if input.starts_with(':') {
            let pos = input.find(' ')?;
            self.pos += pos + 1;
            return Prefix::parse(&input[..pos]);
        }
        None
    }

    fn command(&mut self) -> anyhow::Result<Command> {
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

    fn args(&mut self) -> Vec<String> {
        let input = &self.input[self.pos..];
        let pos = input.find(':').unwrap_or_else(|| input.len());
        self.pos += pos + 1;
        input[..pos]
            .split_whitespace()
            .map(|s| s.trim())
            .map(ToString::to_string)
            .collect()
    }

    fn data(&mut self) -> Option<String> {
        self.input
            .get(self.pos..)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
    }
}
