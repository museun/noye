use crate::modules;
use futures::prelude::*;
use prelude::*;
use std::sync::Arc;

#[derive(Template, Debug)]
#[parent("user_error")]
pub enum UserError {
    NotOwner,
}

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
        modules::builtin::init_start(std::time::Instant::now());

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

            // this detaches a task that waits for all of the futures to be done
            // when they finish, the channel will drop
            let mut responses = self.dispatcher.dispatch(msg, Arc::clone(&config));
            // which causes this stream to end
            // so this stays serial, as intended
            // TODO if the handler blocks it'll block here
            // so we should probably spawn this in a task and put it in a set
            // which it can poll or timeout
            while let Some(resp) = responses.next().await {
                self.client.write(resp).await?;
            }
        }
    }
}

/// Module to allow for globbing all of the common imports used when writing modules/handlers
pub mod prelude;

#[doc(inline)]
pub use template::Template;

#[cfg(test)]
pub mod test;
