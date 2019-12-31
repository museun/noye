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

async fn uptime(context: Context, mut noye: Noye) -> anyhow::Result<()> {
    #[derive(Template, Debug)]
    #[parent("uptime")]
    enum Output {
        Uptime { uptime: String },
    }
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
    write_command(Supervisor::Restart, &addr).await
}

async fn respawn(context: Context, mut noye: Noye) -> anyhow::Result<()> {
    if !context.check_auth() {
        return noye.requires_auth(context);
    }

    let delay = context
        .command()?
        .args()
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(15);

    let addr = &context.config().restart_config.address;
    write_command(Supervisor::Delay(delay), &addr).await
}

#[derive(Debug, Copy, Clone)]
enum Supervisor {
    Restart,
    Delay(u16),
}

async fn write_command(cmd: Supervisor, addr: &str) -> anyhow::Result<()> {
    let mut cmd = match cmd {
        Supervisor::Restart => "RESTART".into(),
        Supervisor::Delay(d) => format!("DELAY {}", d),
    };
    cmd.push('\0');

    use tokio::io::*;
    let mut stream = tokio::net::TcpStream::connect(addr).await?;
    stream.write_all(cmd.as_bytes()).await?;
    stream.flush().await.map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::irc::*;

    use futures::prelude::*;
    use tokio::sync::mpsc;

    fn check_error<F, Fut>(func: F, input: Context)
    where
        F: Copy + FnOnce(Context, Noye) -> Fut,
        Fut: Future<Output = anyhow::Result<()>>,
    {
        let (tx, mut rx) = mpsc::channel(32);
        let noye = Noye::new(tx);
        tokio_test::block_on(async move {
            func(input, noye).await.unwrap_err();
            assert!(rx.next().await.is_none())
        });
    }

    fn check<F, Fut>(func: F, input: Context, mut output: Vec<&str>)
    where
        F: Copy + FnOnce(Context, Noye) -> Fut,
        Fut: Future<Output = anyhow::Result<()>>,
    {
        let (tx, mut rx) = mpsc::channel(32);
        let noye = Noye::new(tx);

        tokio_test::block_on(async move {
            if let Err(err) = func(input.clone(), noye).await {
                panic!("failed to run '{}' on '{:#?}'", err, input.message());
            }
            let mut index = 0_usize;
            while let Some(msg) = rx.next().await {
                if output.is_empty() {
                    panic!("got input: ({}) '{}' but output was empty", index, msg);
                }
                let left = output.remove(0);
                assert_eq!(
                    left,
                    msg,
                    "expected at output pos: {}. '{}' != '{}'",
                    index,
                    left.escape_debug(),
                    msg.escape_debug()
                );
                index += 1;
            }
        });
    }

    #[test]
    fn test_ready() {
        let mut ctx = Context::mock_context_msg(Message {
            command: Command::Ready,
            args: vec!["test_user".into()],
            ..Default::default()
        });

        // try without q info
        ctx.config_mut().irc_config.channels = vec!["#test".into(), "#test2".into()];
        check(ready, ctx.clone(), vec!["JOIN #test", "JOIN #test2"]);

        // try with q info
        ctx.config_mut().irc_config.q_name = "test_user".into();
        ctx.config_mut().irc_config.q_pass = "hunter2".into();
        check(
            ready,
            ctx,
            vec![
                "PRIVMSG Q@CServe.quakenet.org :AUTH test_user hunter2",
                "MODE test_user +x",
            ],
        );
    }

    #[test]
    fn test_autojoin() {
        let mut ctx = Context::mock_context_msg(Message {
            command: Command::Ready,
            args: vec!["test_user".into()],
            ..Default::default()
        });

        // without q info
        ctx.config_mut().irc_config.channels = vec!["#test".into(), "#test2".into()];
        check(ready, ctx, vec!["JOIN #test", "JOIN #test2"]);

        // assume our host has been hidden
        let mut ctx = Context::mock_context_msg(Message {
            command: Command::Numeric(396),
            args: vec!["test_user".into()],
            ..Default::default()
        });

        // without q info
        ctx.config_mut().irc_config.channels = vec!["#test".into(), "#test2".into()];
        check(ready, ctx, vec!["JOIN #test", "JOIN #test2"]);
    }

    #[test]
    fn test_ping() {
        let ctx = Context::mock_context_msg(Message {
            command: Command::Ping,
            data: Some("123456789".into()),
            ..Default::default()
        });

        check(ping, ctx, vec!["PONG 123456789"])
    }

    #[test]
    fn test_invite() {
        let ctx = Context::mock_context_msg(Message {
            command: Command::Invite,
            data: Some("#test".into()),
            ..Default::default()
        });

        check(invite, ctx, vec!["JOIN #test"])
    }

    #[test]
    fn test_join() {
        let ctx = Context::mock_context("!join", "#test");
        check(join, ctx, vec!["JOIN #test"]);

        let ctx = Context::mock_context("!join", "");
        check_error(join, ctx);
    }

    #[test]
    fn test_part() {
        let ctx = Context::mock_context("!part", "");
        check(part, ctx, vec!["PART #museun :bye"]);

        let mut ctx = Context::mock_context("!part", "");
        ctx.message_mut().args = Default::default();
        check_error(part, ctx);
    }

    #[test]
    fn test_alt_nick() {
        let ctx = Context::mock_context_msg(Message {
            command: Command::NickCollision,
            args: vec!["".into(), "noye".into()],
            ..Default::default()
        });

        check(alt_nick, ctx, vec!["NICK noye_"]);
    }

    #[test]
    fn test_reclaim() {
        let mut ctx = Context::mock_context_msg(Message {
            command: Command::Nick,
            ..Default::default()
        });
        ctx.config_mut().irc_config.nick = "noye".into();
        check(reclaim, ctx, vec!["NICK noye"]);

        let mut ctx = Context::mock_context_msg(Message {
            command: Command::Quit,
            ..Default::default()
        });
        ctx.config_mut().irc_config.nick = "noye".into();
        check(reclaim, ctx, vec!["NICK noye"]);
    }

    #[test]
    fn test_uptime() {
        init_start(Instant::now() - std::time::Duration::from_secs(90062));

        let ctx = Context::mock_context("!uptime", "");
        check(
            uptime,
            ctx,
            vec!["PRIVMSG #museun :I've been running for 1 day, 1 hour, 1 minute and 2 seconds"],
        );
    }

    #[test]
    fn test_restart() {
        let mut ctx = Context::mock_context("!restart", "");
        check(
            restart,
            ctx.clone(),
            vec!["PRIVMSG #museun :noye: you cannot do that"],
        );

        let mut rt = tokio::runtime::Runtime::new().unwrap();
        let (addr, mut rx) = supervisor(&mut rt);

        ctx.config_mut().restart_config.address = addr;
        ctx.config_mut().irc_config.owners.push("noye".into());
        check(restart, ctx, vec![]);

        let command = rt.block_on(async move { rx.next().await.unwrap() });
        assert_eq!(command, "RESTART");
    }

    #[test]
    fn test_respawn() {
        let mut ctx = Context::mock_context("!respawn", "");
        check(
            respawn,
            ctx.clone(),
            vec!["PRIVMSG #museun :noye: you cannot do that"],
        );

        let mut rt = tokio::runtime::Runtime::new().unwrap();
        let (addr, mut rx) = supervisor(&mut rt);

        ctx.config_mut().restart_config.address = addr;
        ctx.config_mut().irc_config.owners.push("noye".into());
        check(respawn, ctx, vec![]);

        let command = rt.block_on(async move { rx.next().await.unwrap() });
        assert_eq!(command, "DELAY 15");

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
