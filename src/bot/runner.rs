use super::*;
use crate::irc::{Command, Prefix, RawMessage};

use std::sync::Arc;
use tokio::sync::{Mutex, Notify};

pub struct Runner<R> {
    pub quit: Arc<Notify>,
    pub writer: Writer,
    pub commands: CommandsMap<R>,
    pub passives: PassivesList<R>,
    pub state: Arc<Mutex<State>>,
    _phantom: std::marker::PhantomData<R>,
}

impl<R: Responder + Send + 'static> Runner<R> {
    pub fn new(
        state: State,
        writer: Writer,
        commands: CommandsMap<R>,
        passives: PassivesList<R>,
    ) -> Self {
        let (quit, _phantom) = Default::default();
        Self {
            quit,
            commands,
            passives,
            writer,
            state: Arc::new(Mutex::new(state)),
            _phantom,
        }
    }

    pub async fn handle(&mut self, data: &str, responder: R) -> anyhow::Result<()> {
        let msg = RawMessage::parse(data)?;

        match msg.command {
            Command::Privmsg => {
                let context = Context::new(
                    msg.into_message(),
                    context::ContextArgs {
                        quit: self.quit.clone(),
                        writer: self.writer.clone(),
                        state: self.state.clone(),
                    },
                );
                self.dispatch(context, responder.clone())
            }

            Command::Ping => {
                // TODO add in netsplit detection (incremental ping backoff)
                self.writer
                    .raw(format!("PONG {}", msg.data.unwrap()))
                    .await?;
            }

            Command::Ready => {
                let mut state = self.state.lock().await;
                let crate::config::Irc {
                    q_name,
                    q_pass,
                    channels,
                    ..
                } = &state.config().await?.irc_config;

                // TODO if we don't get a response from Q within N seconds, just join the channels
                match (q_name, q_pass) {
                    (Some(q_name), Some(q_pass)) if !q_name.is_empty() && !q_pass.is_empty() => {
                        log::info!("authing with Q");
                        self.writer
                            .raw(format!(
                                "PRIVMSG Q@CServe.quakenet.org :AUTH {} {}",
                                q_name, q_pass
                            ))
                            .await?;
                        self.writer.raw(format!("MODE {} +x", &msg.args[0])).await?;
                    }
                    _ => {
                        log::warn!("not authing with Q, joining channels");
                        for channel in channels {
                            self.writer.join(channel).await?;
                        }
                    }
                }
            }

            Command::Quit | Command::Nick => {
                let mut state = self.state.lock().await;
                let name = &state.config().await?.irc_config.name;
                if let Some(Prefix::User { nick, .. }) = &msg.prefix {
                    if nick == name {
                        log::info!("attempting to regain our nick: {}", name);
                        self.writer.nick(name).await?;
                    }
                }
            }

            Command::NickCollision => {
                let new_nick = format!("{}_", msg.args[1]);
                log::info!("our nickname is taken, changing to: {}", new_nick);
                self.writer.nick(new_nick).await?;
            }

            Command::Invite => {
                let channel = msg.data.unwrap();
                log::info!("we were invited to {}, joining", channel);
                self.writer.join(channel).await?;
            }

            Command::Numeric(396) => {
                // TODO this is where the timeout should be handled for Q
                log::info!("successfully authenticated with Q");
                let mut state = self.state.lock().await;
                for channel in &state.config().await?.irc_config.channels {
                    self.writer.join(channel).await?;
                }
            }

            _ => {}
        }
        Ok(())
    }

    fn dispatch(&self, context: Context, responder: R) {
        use crate::util::inspect_err;
        use futures::prelude::*;

        if let Some((cmd, head)) = context.command().and_then(|head| {
            self.commands
                .map
                .get(head)
                .map(|cmd| (cmd, head.to_string()))
        }) {
            let fut = cmd
                .call(context.clone(), responder.clone())
                .inspect_err(move |err| inspect_err(err, || format!("command '{}'", head)));
            tokio::task::spawn(fut);
        }

        for passive in &self.passives.list {
            let fut = passive
                .call(context.clone(), responder.clone())
                .inspect_err(move |err| inspect_err(err, || "passive"));
            tokio::task::spawn(fut);
        }
    }
}
