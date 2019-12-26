/// An IRC command
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// When a nick collision happens
    NickCollision,
    /// When the ready event is received
    Ready,
    /// When the server is checking the connection
    Ping,
    /// When someone invites you to a channel
    Invite,
    /// When someone joins a channel
    Join,
    /// When someone leaves a channel
    Part,
    /// When someone quits
    Quit,
    /// When someone changes their name
    Nick,
    /// When a message is received either on a channel or from a user
    Privmsg,
    /// A numeric event
    Numeric(u16),
    /// An unknown event
    Unknown(Box<str>),
}

impl Command {
    pub fn as_box_str(&self) -> Box<str> {
        match self {
            Command::NickCollision => "NickCollision".into(),
            Command::Ready => "Ready".into(),
            Command::Ping => "Ping".into(),
            Command::Invite => "Invite".into(),
            Command::Join => "Join".into(),
            Command::Part => "Part".into(),
            Command::Quit => "Quit".into(),
            Command::Nick => "Nick".into(),
            Command::Privmsg => "Privmsg".into(),
            Command::Numeric(num) => format!("{}", num).into(),
            Command::Unknown(s) => s.clone(),
        }
    }
}
