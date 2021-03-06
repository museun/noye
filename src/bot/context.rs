use super::{Message, Responder, State, Writer};

use std::sync::Arc;
use tokio::sync::{Mutex, Notify};

#[derive(Clone)]
pub struct ContextArgs {
    pub quit: Arc<Notify>,
    pub writer: Writer,
    pub state: Arc<Mutex<State>>,
}

#[derive(Clone)]
pub struct Context<A: std::fmt::Debug = Message> {
    pub args: Arc<A>,
    pub writer: Writer,
    pub state: Arc<Mutex<State>>,
    pub quit: Arc<Notify>,
}

impl<A: std::fmt::Debug> std::fmt::Debug for Context<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context").field("args", &self.args).finish()
    }
}

impl<A: std::fmt::Debug> Context<A> {
    pub fn new(args: A, ctx_args: ContextArgs) -> Self {
        Self {
            args: Arc::new(args),
            writer: ctx_args.writer,
            state: ctx_args.state,
            quit: ctx_args.quit,
        }
    }
}

impl Context<Message> {
    pub fn parts(&self) -> impl Iterator<Item = &'_ str> + '_ {
        self.args.data.split(' ')
    }

    pub fn command(&self) -> Option<&str> {
        if self.args.data.starts_with('!') {
            return self.args.data.split_terminator(' ').next().map(|s| &s[1..]);
        }
        None
    }

    pub fn command_args(&self) -> Vec<&str> {
        if self.args.data.starts_with('!') {
            return self.args.data[1..].split(' ').skip(1).collect();
        }
        vec![]
    }

    pub fn without_command(&self) -> Option<&str> {
        if self.args.data.starts_with('!') {
            let pos = self.args.data.find(' ')?;
            return self.args.data.get(pos + 1..);
        }
        None
    }

    pub fn nick(&self) -> &str {
        &self.args.sender
    }

    pub fn room(&self) -> &str {
        &self.args.channel
    }

    pub async fn config(&self) -> anyhow::Result<crate::Config> {
        self.state.lock().await.config().await.map(Clone::clone)
    }

    pub async fn expect_owner<R: ?Sized + Responder>(
        &self,
        responder: &mut R,
    ) -> anyhow::Result<()> {
        if self
            .state
            .lock()
            .await
            .config()
            .await?
            .irc_config
            .owners
            .contains(&self.args.sender)
        {
            return Ok(());
        }

        responder
            .reply(self.clone(), crate::responses::Builtin::NotOwner)
            .await?;

        // This is needed to signal that this is an error
        crate::util::dont_care()
    }

    pub fn get_links(&self) -> anyhow::Result<Vec<url::Url>> {
        self.get_links_filter(|_| true)
    }

    pub fn get_links_filter(&self, f: impl Fn(&url::Url) -> bool) -> anyhow::Result<Vec<url::Url>> {
        let urls: Vec<_> = self
            .args
            .data
            .split(' ')
            .flat_map(crate::http::client::parse_http_url)
            .filter(f)
            .collect();

        if !urls.is_empty() {
            return Ok(urls);
        }

        crate::util::dont_care()
    }

    pub async fn is_banned_channel(&self) -> anyhow::Result<()> {
        if self
            .state
            .lock()
            .await
            .config()
            .await?
            .irc_config
            .channels
            .contains(&self.args.sender)
        {
            return Ok(());
        }

        crate::util::dont_care()
    }
}
