use crate::modules;
use prelude::*;
use std::sync::Arc;

/// Bot is the main item for creating a bot
pub struct Bot<T> {
    config: Config,
    client: crate::irc::Client<T>,
    dispatcher: crate::command::Dispatcher,
}

impl<T> Bot<T> {
    /// Create a bot from a provided [`Config`](../config/struct.Config.html) and a [`Client`](../irc/struct.Client.html)
    pub fn create(config: Config, client: crate::irc::Client<T>) -> Self {
        // this is when the bot started, so force the initialization here
        once_cell::sync::Lazy::force(&modules::builtin::START);

        let mut dispatcher = Dispatcher::default();
        modules::load_modules(&config, &mut dispatcher);
        Self {
            config,
            client,
            dispatcher,
        }
    }

    /// Get a mutable reference to the [`Dispatcher`](./prelude/struct.Dispatcher.html)
    ///
    /// This allows you to add:
    /// * [`Command handlers`](./prelude/struct.Dispatcher.html#method.command)
    ///     - for reacting to user-triggered commands
    ///     - ex: `!command`
    /// * [`Listening handlers`](./prelude/struct.Dispatcher.html#method.listener)
    ///     - for reacting to IRC events
    ///     - ex: `PING`
    /// * [`Passive handlers`](./prelude/struct.Dispatcher.html#method.passive)
    ///     - for reacting to regular expressions on user messages
    ///     - ex: `(^hello)\s(?P<what>.*?$)`
    pub fn dispatcher(&mut self) -> &mut crate::command::Dispatcher {
        &mut self.dispatcher
    }

    /// Run the bot until it disconnects
    pub async fn run_loop(mut self) -> anyhow::Result<()>
    where
        T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        let crate::config::Irc {
            nick, user, real, ..
        } = &self.config.irc_config;

        self.client
            .register(crate::irc::Registration {
                nick: &nick,
                user: &user,
                real: &real,
            })
            .await?;

        let config = Arc::new(self.config);

        loop {
            let msg = self.client.read().await?;
            log::trace!("{:?}", msg);
            for resp in self.dispatcher.dispatch(msg, Arc::clone(&config)).await {
                for line in resp.into_iter().filter(|s| !s.is_empty()) {
                    self.client.write(line).await?;
                }
            }
        }
    }
}

/// Module to allow for globbing all of the common imports used when writing modules/handlers
pub mod prelude;

mod template;
pub use template::Template;
