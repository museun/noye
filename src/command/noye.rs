use super::{Context};
use crate::bot::{UserError};
use template::{Template, TemplateResolver};
use tokio::sync::mpsc::Sender;

#[derive(Clone)]
pub struct Noye {
    sender: Sender<String>,
}

impl Noye {
    pub(crate) fn new(sender: Sender<String>) -> Self {
        Self { sender }
    }

    pub fn nothing(&self) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn raw(&mut self, data: impl ToString) -> anyhow::Result<()> {
        let _ = self.sender.try_send(data.to_string());
        Ok(())
    }

    pub fn reply(&mut self, ctx: Context, data: impl std::fmt::Display) -> anyhow::Result<()> {
        use crate::irc::Target;
        match ctx.target() {
            Some(Target::Channel(target)) => self.raw(format!(
                "PRIVMSG {} :{}: {}",
                target,
                ctx.nick().expect("nick must be attached to message"),
                data
            )),
            Some(Target::Private(target)) => self.raw(format!("PRIVMSG {} :{}", target, data)),
            None => {
                log::warn!("cannot reply to a message without a target");
                self.nothing()
            }
        }
    }

    pub fn say(&mut self, ctx: Context, data: impl std::fmt::Display) -> anyhow::Result<()> {
        use crate::irc::Target;
        match ctx.target() {
            Some(Target::Channel(target)) | Some(Target::Private(target)) => {
                self.raw(format!("PRIVMSG {} :{}", target, data))
            }
            None => {
                log::warn!("cannot reply to a message without a target");
                self.nothing()
            }
        }
    }

    pub fn say_template<T: Template>(&mut self, ctx: Context, template: T) -> anyhow::Result<()> {
        match resolve_template(template) {
            Some(data) => self.say(ctx, data),
            None => self.nothing(),
        }
    }

    pub fn reply_template<T: Template>(&mut self, ctx: Context, template: T) -> anyhow::Result<()> {
        match resolve_template(template) {
            Some(data) => self.reply(ctx, data),
            None => self.nothing(),
        }
    }

    pub fn join(&mut self, data: impl std::fmt::Display) -> anyhow::Result<()> {
        self.raw(format!("JOIN {}", data))
    }

    pub fn part(&mut self, data: impl std::fmt::Display) -> anyhow::Result<()> {
        self.raw(format!("PART {} :bye", data))
    }

    pub fn nick(&mut self, data: impl std::fmt::Display) -> anyhow::Result<()> {
        self.raw(format!("NICK {}", data))
    }

    pub fn requires_auth(&mut self, ctx: Context) -> anyhow::Result<()> {
        self.reply_template(ctx, UserError::NotOwner)
    }
}

pub fn resolve_template<T: Template>(template: T) -> Option<String> {
    let (parent, name) = (T::parent(), template.variant());
    TemplateResolver::load(parent, name, "templates.toml")
        .and_then(|data| template.apply(&data))
        .or_else(|| {
            log::warn!("template resolver failed to find {}::{}", parent, name);
            None
        })
}
