#[derive(Debug)]
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
