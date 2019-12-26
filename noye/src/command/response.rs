use crate::command::Context;
use crate::irc::Target;

/// A Response from a handler
#[derive(Debug, Default)]
pub struct Response {
    data: Vec<String>,
}

impl std::iter::IntoIterator for Response {
    type Item = String;
    type IntoIter = <Vec<Self::Item> as std::iter::IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.data.into_iter()
    }
}

impl Response {
    /// Create an empty response
    pub fn empty() -> Self {
        Self { data: vec![] }
    }

    /// Sends this data as a raw string to the connection
    pub fn raw(data: impl ToString) -> Self {
        Self {
            data: vec![data.to_string()],
        }
    }

    /// Reply to this context with this message
    pub fn reply(context: Context, msg: impl std::fmt::Display) -> Self {
        match context.target() {
            Some(Target::Channel(target)) => Self::raw(format!(
                "PRIVMSG {} :{}: {}",
                context.nick().expect("nick to be attached to message"),
                target,
                msg
            )),
            Some(Target::Private(target)) => Self::raw(format!("PRIVMSG {} :{}", target, msg)),
            None => {
                log::warn!("cannot reply to a message without a target");
                Response::empty()
            }
        }
    }

    pub fn say(context: Context, msg: impl std::fmt::Display) -> Self {
        match context.target() {
            Some(Target::Channel(target)) => Self::raw(format!("PRIVMSG {} :{}", target, msg)),
            Some(Target::Private(target)) => Self::raw(format!("PRIVMSG {} :{}", target, msg)),
            None => {
                log::warn!("cannot reply to a message without a target");
                Response::empty()
            }
        }
    }

    /// Joins the channel
    pub fn join(channel: impl std::fmt::Display) -> Self {
        Self::raw(format!("JOIN {}", channel))
    }

    /// Parts the channel
    pub fn part(channel: impl std::fmt::Display) -> Self {
        Self::raw(format!("PART {}", channel))
    }

    pub fn nick(nick: impl std::fmt::Display) -> Self {
        Self::raw(format!("NICK {}", nick))
    }
}

/// Convert this type into a response
pub trait IntoResponse {
    /// Consume self, returning a response
    fn into_response(self, context: Context) -> Response;
}

/// An empty response
impl IntoResponse for () {
    fn into_response(self, _: Context) -> Response {
        Response::empty()
    }
}

/// Identity response
impl IntoResponse for Response {
    fn into_response(self, _: Context) -> Response {
        self
    }
}

/// Say this string
impl IntoResponse for String {
    fn into_response(self, context: Context) -> Response {
        self.as_str().into_response(context)
    }
}

/// Say this string
impl IntoResponse for &str {
    fn into_response(self, context: Context) -> Response {
        Response::say(context, self)
    }
}

impl<T> IntoResponse for Option<T>
where
    T: IntoResponse,
{
    fn into_response(self, context: Context) -> Response {
        self.map(|data| IntoResponse::into_response(data, context))
            .unwrap_or_else(Response::empty)
    }
}

impl<T> IntoResponse for Result<T, anyhow::Error>
where
    T: IntoResponse,
{
    fn into_response(self, context: Context) -> Response {
        match self {
            Ok(resp) => resp.into_response(context),
            Err(err) => {
                log::error!("got a module error: {}", err);
                Response::empty()
            }
        }
    }
}

/// A vec of responses
impl<T> IntoResponse for Vec<T>
where
    T: IntoResponse,
{
    fn into_response(self, context: Context) -> Response {
        let data = self
            .into_iter()
            .map(|s| s.into_response(context.clone()))
            .flat_map(|s| s.data.into_iter())
            .collect();
        Response { data }
    }
}
