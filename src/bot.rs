use crate::{AnyhowFut, Command, Config, Context, Handler, Irc, Prefix, Responder, State, Writer};

use futures::prelude::*;
use std::{collections::HashMap, future::Future, sync::Arc};
use tokio::sync::{Mutex, Notify};

pub struct CommandsMap<R> {
    map: HashMap<String, Arc<dyn Handler<R, Fut = AnyhowFut<'static>> + Send + 'static>>,
    _marker: std::marker::PhantomData<R>,
}

impl<R> Default for CommandsMap<R> {
    fn default() -> Self {
        Self {
            map: Default::default(),
            _marker: std::marker::PhantomData::default(),
        }
    }
}

impl<R: Responder + Send + 'static> CommandsMap<R> {
    pub fn add<H, F>(&mut self, cmd: impl ToString, handler: H) -> anyhow::Result<()>
    where
        H: Handler<R, Fut = F>,
        F: Future<Output = anyhow::Result<()>> + Send + 'static,
        F::Output: Send + 'static,
    {
        let cmd = cmd.to_string();
        if self.map.contains_key(&cmd) {
            anyhow::bail!("{} already exists as a command", cmd)
        }

        self.map
            .insert(cmd, Arc::new(move |state, resp| handler.call(state, resp)));

        Ok(())
    }
}

pub struct PassivesList<R> {
    list: Vec<Arc<dyn Handler<R, Fut = AnyhowFut<'static>> + Send + 'static>>,
    _marker: std::marker::PhantomData<R>,
}

impl<R> Default for PassivesList<R> {
    fn default() -> Self {
        Self {
            list: Default::default(),
            _marker: std::marker::PhantomData::default(),
        }
    }
}

impl<R: Responder + Send + 'static> PassivesList<R> {
    pub fn add<H, F>(&mut self, handler: H)
    where
        H: Handler<R, Fut = F>,
        F: Future<Output = anyhow::Result<()>> + Send + 'static,
        F::Output: Send + 'static,
    {
        self.list
            .push(Arc::new(move |state, resp| handler.call(state, resp)))
    }
}

pub struct Bot<R> {
    pub quit: Arc<Notify>,
    pub writer: Writer,
    pub commands: CommandsMap<R>,
    pub passives: PassivesList<R>,
    pub state: Arc<Mutex<State>>,
    _phantom: std::marker::PhantomData<R>,
}

impl<R: Responder + Send + 'static> Bot<R> {
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
        let msg = Irc::parse(data)?;
        match msg.command {
            Command::Privmsg => {
                let context = Context::new(msg.into_message(), self);
                self.dispatch(context, responder.clone())
            }

            Command::Ping => {
                self.writer
                    .raw(format!("PONG {}", msg.data.unwrap()))
                    .await?;
            }

            Command::Ready => {
                let mut state = self.state.lock().await;
                let Config {
                    irc_config:
                        crate::config::Irc {
                            q_name,
                            q_pass,
                            channels,
                            ..
                        },
                    ..
                } = state.config().await?;

                match (q_name, q_pass) {
                    (Some(q_name), Some(q_pass)) if !q_name.is_empty() && !q_pass.is_empty() => {
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
                let Config {
                    irc_config: crate::config::Irc { name, .. },
                    ..
                } = state.config().await?;

                let sender = match &msg.prefix {
                    Some(Prefix::User { nick, .. }) => nick,
                    _ => unreachable!(),
                };
                if name == sender {
                    self.writer.nick(name).await?;
                }
            }

            Command::NickCollision => {
                self.writer.nick(format!("{}_", msg.args[1])).await?;
            }

            Command::Invite => {
                self.writer.join(msg.data.unwrap()).await?;
            }

            Command::Numeric(396) => {
                let mut state = self.state.lock().await;
                let Config {
                    irc_config: crate::config::Irc { channels, .. },
                    ..
                } = state.config().await?;

                for channel in channels {
                    self.writer.join(channel).await?;
                }
            }

            _ => {}
        }
        Ok(())
    }

    fn dispatch(&self, context: Context, responder: R) {
        if context.args.data.starts_with('!') {
            let input = &context.args.data[1..];
            if let Some(head) = input.splitn(2, ' ').next() {
                if let Some(cmd) = self.commands.map.get(head) {
                    let head = head.to_string();
                    let fut =
                        cmd.call(context.clone(), responder.clone())
                            .inspect_err(move |err| {
                                crate::util::inspect_err(err, || format!("command '{}'", head))
                            });
                    tokio::task::spawn(fut);
                }
            }
        }

        for passive in &self.passives.list {
            let fut = passive
                .call(context.clone(), responder.clone())
                .inspect_err(move |err| crate::util::inspect_err(err, || "passive"));
            tokio::task::spawn(fut);
        }
    }
}
