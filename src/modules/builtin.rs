use crate::bot::prelude::*;

use once_cell::sync::OnceCell;
use std::time::Instant;

pub(crate) static START: OnceCell<Instant> = OnceCell::new();

pub(crate) fn init_start(when: Instant) {
    START.get_or_init(|| when);
}

registry!("builtin" => {
    listener => Command::Ping,          ping;
    listener => Command::Ready,         ready;
    listener => Command::Numeric(396),  autojoin;
    listener => Command::Invite,        invite;
    listener => Command::NickCollision, alt_nick;
    listener => Command::Quit,          reclaim;
    listener => Command::Nick,          reclaim;    
    command  => "!join",                join;
    command  => "!part",                part;
    command  => "!uptime",              uptime;
    command  => "!restart",             restart;
    command  => "!respawn",             respawn;
});

async fn ready(context: Context, mut noye: Noye) -> anyhow::Result<()> {
    let config::Irc { q_name, q_pass, .. } = &context.config().irc_config;
    if q_name.is_empty() || q_pass.is_empty() {
        log::warn!("not authing with Q, joining channels");
        return autojoin(context, noye).await;
    }

    noye.raw(format!(
        "PRIVMSG Q@CServe.quakenet.org :AUTH {} {}",
        q_name, q_pass
    ))?;
    noye.raw(format!("MODE {} +x", context.arg(0)?))
}

async fn autojoin(context: Context, mut noye: Noye) -> anyhow::Result<()> {
    let config::Irc { channels, .. } = &context.config().irc_config;
    for channel in channels {
        log::debug!("autojoin: {}", channel);
        noye.join(channel)?;
    }
    noye.nothing()
}

async fn ping(context: Context, mut noye: Noye) -> anyhow::Result<()> {
    noye.raw(format!("PONG {}", context.data()?))
}

async fn invite(context: Context, mut noye: Noye) -> anyhow::Result<()> {
    noye.join(context.data()?)
}

async fn join(context: Context, mut noye: Noye) -> anyhow::Result<()> {
    noye.join(context.command()?.args()?)
}

async fn part(context: Context, mut noye: Noye) -> anyhow::Result<()> {
    noye.part(context.target_channel()?)
}

async fn alt_nick(context: Context, mut noye: Noye) -> anyhow::Result<()> {
    noye.nick(format!("{}_", context.arg(1)?))
}

async fn reclaim(context: Context, mut noye: Noye) -> anyhow::Result<()> {
    let config::Irc { nick, .. } = &context.config().irc_config;
    let sender = context.nick()?;
    if nick == sender {
        return noye.nick(nick);
    }
    noye.nothing()
}

#[derive(Template, Debug)]
#[parent("uptime")]
enum Output {
    Uptime { uptime: String },
}

async fn uptime(context: Context, mut noye: Noye) -> anyhow::Result<()> {
    noye.say_template(
        context,
        Output::Uptime {
            uptime: START.get().unwrap().elapsed().as_readable_time(),
        },
    )
}

async fn restart(context: Context, mut noye: Noye) -> anyhow::Result<()> {
    if !context.check_auth() {
        return noye.requires_auth(context);
    }

    let addr = &context.config().restart_config.address;
    Supervisor::Restart.write(&addr).await
}

async fn respawn(context: Context, mut noye: Noye) -> anyhow::Result<()> {
    if !context.check_auth() {
        return noye.requires_auth(context);
    }

    let delay = context
        .command()?
        .args()
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(15);

    let addr = &context.config().restart_config.address;
    Supervisor::Delay(delay).write(&addr).await
}

#[derive(Debug, Copy, Clone)]
enum Supervisor {
    Restart,
    Delay(u16),
}

impl Supervisor {
    async fn write(self, addr: &str) -> anyhow::Result<()> {
        let cmd = match self {
            Supervisor::Restart => "RESTART\0".to_string(),
            Supervisor::Delay(d) => format!("DELAY {}\0", d),
        };

        use tokio::io::*;
        let mut stream = tokio::net::TcpStream::connect(addr).await?;
        stream.write_all(cmd.as_bytes()).await?;
        stream.flush().await.map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bot::test::*;

    #[test]
    fn ready_without_q() {
        let mut ctx = Context::mock_context_msg(Message {
            command: Command::Ready,
            args: vec!["test_user".into()],
            ..Default::default()
        });

        // try without q info
        ctx.config_mut().irc_config.channels = vec!["#test".into(), "#test2".into()];
        check(super::ready, ctx, vec!["JOIN #test", "JOIN #test2"]);
    }

    #[test]
    fn ready_with_q() {
        let mut ctx = Context::mock_context_msg(Message {
            command: Command::Ready,
            args: vec!["test_user".into()],
            ..Default::default()
        });

        ctx.config_mut().irc_config.channels = vec!["#test".into(), "#test2".into()];
        ctx.config_mut().irc_config.q_name = "test_user".into();
        ctx.config_mut().irc_config.q_pass = "hunter2".into();

        check(
            super::ready,
            ctx,
            vec![
                "PRIVMSG Q@CServe.quakenet.org :AUTH test_user hunter2",
                "MODE test_user +x",
            ],
        );
    }

    #[test]
    fn autojoin_without_q() {
        let mut ctx = Context::mock_context_msg(Message {
            command: Command::Ready,
            args: vec!["test_user".into()],
            ..Default::default()
        });

        // without q info
        ctx.config_mut().irc_config.channels = vec!["#test".into(), "#test2".into()];
        check(super::ready, ctx, vec!["JOIN #test", "JOIN #test2"]);
    }

    #[test]
    fn autojoin_after_hidden_host() {
        // assume our host has been hidden
        let mut ctx = Context::mock_context_msg(Message {
            command: Command::Numeric(396),
            args: vec!["test_user".into()],
            ..Default::default()
        });

        // without q info
        ctx.config_mut().irc_config.channels = vec!["#test".into(), "#test2".into()];
        check(super::ready, ctx, vec!["JOIN #test", "JOIN #test2"]);
    }

    #[test]
    fn ping() {
        let ctx = Context::mock_context_msg(Message {
            command: Command::Ping,
            data: Some("123456789".into()),
            ..Default::default()
        });

        check(super::ping, ctx, vec!["PONG 123456789"])
    }

    #[test]
    fn invite() {
        let ctx = Context::mock_context_msg(Message {
            command: Command::Invite,
            data: Some("#test".into()),
            ..Default::default()
        });

        check(super::invite, ctx, vec!["JOIN #test"])
    }

    #[test]
    fn join() {
        let ctx = Context::mock_context("!join", "#test");
        check(super::join, ctx, vec!["JOIN #test"]);
    }

    #[test]
    fn join_invalid() {
        let ctx = Context::mock_context("!join", "");
        check_error(super::join, ctx);
    }

    #[test]
    fn part() {
        let ctx = Context::mock_context("!part", "");
        check(super::part, ctx, vec!["PART #museun :bye"]);
    }

    #[test]
    fn part_invalid() {
        let mut ctx = Context::mock_context("!part", "");
        ctx.message_mut().args = vec!["test_user".into()];
        check_error(super::part, ctx);
    }

    #[test]
    fn alt_nick() {
        let ctx = Context::mock_context_msg(Message {
            command: Command::NickCollision,
            args: vec!["".into(), "noye".into()],
            ..Default::default()
        });

        check(super::alt_nick, ctx, vec!["NICK noye_"]);
    }

    #[test]
    fn reclaim_nick() {
        let mut ctx = Context::mock_context_msg(Message {
            command: Command::Nick,
            ..Default::default()
        });
        ctx.config_mut().irc_config.nick = "noye".into();
        check(super::reclaim, ctx, vec!["NICK noye"]);
    }

    #[test]
    fn reclaim_quit() {
        let mut ctx = Context::mock_context_msg(Message {
            command: Command::Quit,
            ..Default::default()
        });
        ctx.config_mut().irc_config.nick = "noye".into();
        check(super::reclaim, ctx, vec!["NICK noye"]);
    }

    #[test]
    fn reclaim_nick_nothing() {
        let mut ctx = Context::mock_context_msg(Message {
            command: Command::Nick,
            ..Default::default()
        });
        ctx.config_mut().irc_config.nick = "not_noye".into();
        check(super::reclaim, ctx, vec![]);
    }

    #[test]
    fn reclaim_quit_nothing() {
        let mut ctx = Context::mock_context_msg(Message {
            command: Command::Quit,
            ..Default::default()
        });
        ctx.config_mut().irc_config.nick = "not_noye".into();
        check(super::reclaim, ctx, vec![]);
    }

    #[test]
    fn uptime() {
        let ts = Instant::now() - std::time::Duration::from_secs(90062);
        init_start(ts);

        let ctx = Context::mock_context("!uptime", "");
        let resp = say_template(
            ctx.clone(),
            Output::Uptime {
                uptime: ts.elapsed().as_readable_time(),
            },
        );

        check(super::uptime, ctx, vec![&resp]);
    }

    #[test]
    fn restart_no_auth() {
        let ctx = Context::mock_context("!restart", "");
        let resp = reply_template(ctx.clone(), UserError::NotOwner);
        check(super::restart, ctx, vec![&resp]);
    }

    #[test]
    fn restart() {
        let mut ctx = Context::mock_context("!restart", "");

        let mut rt = tokio::runtime::Runtime::new().unwrap();
        let (addr, mut rx) = supervisor(&mut rt);

        ctx.config_mut().restart_config.address = addr;
        ctx.config_mut().irc_config.owners.push("noye".into());
        check(super::restart, ctx, vec![]);

        let command = rt.block_on(async move { rx.next().await.unwrap() });
        assert_eq!(command, "RESTART");
    }

    #[test]
    fn respawn_no_auth() {
        let ctx = Context::mock_context("!respawn", "");
        let resp = reply_template(ctx.clone(), UserError::NotOwner);
        check(super::respawn, ctx, vec![&resp]);
    }

    #[test]
    fn respawn_default() {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        let (addr, mut rx) = supervisor(&mut rt);

        let mut ctx = Context::mock_context("!respawn", "");
        ctx.config_mut().restart_config.address = addr;
        ctx.config_mut().irc_config.owners.push("noye".into());
        check(respawn, ctx, vec![]);

        let command = rt.block_on(async move { rx.next().await.unwrap() });
        assert_eq!(command, "DELAY 15");
    }

    #[test]
    fn respawn_arg() {
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        let (addr, mut rx) = supervisor(&mut rt);

        let mut ctx = Context::mock_context("!respawn", "30");
        ctx.config_mut().restart_config.address = addr;
        ctx.config_mut().irc_config.owners.push("noye".into());
        check(respawn, ctx, vec![]);

        let command = rt.block_on(async move { rx.next().await.unwrap() });
        assert_eq!(command, "DELAY 30");
    }

    fn supervisor(rt: &mut tokio::runtime::Runtime) -> (String, impl Stream<Item = String>) {
        let (mut tx, rx) = tokio::sync::mpsc::channel(1);

        let addr = rt.block_on(async move {
            let mut listener = tokio::net::TcpListener::bind("localhost:0").await.unwrap();
            let addr = listener.local_addr().unwrap().to_string();

            tokio::task::spawn(async move {
                use tokio::io::*;
                let mut buf = vec![];
                BufReader::new(listener.accept().await.unwrap().0)
                    .read_until(b'\0', &mut buf)
                    .await
                    .unwrap();
                assert_eq!(buf.pop().unwrap(), b'\0');
                let cmd = String::from_utf8(buf).unwrap();
                tx.send(cmd).await.unwrap();
            });

            addr
        });

        (addr, rx)
    }
}
