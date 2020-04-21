#![allow(dead_code)]
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
    pub(super) fn parse(input: &str) -> Option<Self> {
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
