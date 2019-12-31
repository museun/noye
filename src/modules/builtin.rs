use crate::bot::prelude::*;

use once_cell::sync::Lazy;
use std::time::Instant;

pub(crate) static START: Lazy<Instant> = Lazy::new(Instant::now);

registry!("builtin" => {
    listener => Command::Ping,          ping;
    listener => Command::Ready,         ready;
    listener => Command::Invite,        invite;
    listener => Command::NickCollision, alt_nick;
    listener => Command::Quit,          reclaim;
    listener => Command::Nick,          reclaim;
    command  => "!join",                join;
    command  => "!part",                part;
    command  => "!uptime",              uptime;
    // command  => "!restart",             restart;
    // command  => "!respawn",             respawn;
});

async fn ready(context: Context, mut noye: Noye) -> anyhow::Result<()> {
    let config::Irc {
        channels,
        q_name,
        q_pass,
        ..
    } = &context.config().irc_config;

    if !q_name.is_empty() && !q_pass.is_empty() {
        noye.raw(format!(
            "PRIVMSG Q@CServe.quakenet.org :AUTH {} {}",
            q_name, q_pass
        ))?;
        noye.raw(format!("MODE {} +x", context.arg(0)?))?;
    }
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
return        noye.nick(nick)
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
            uptime: START.elapsed().as_readable_time(),
        },
    )
}

// async fn restart(ctx: Context) -> impl IntoResponse {
//     if !ctx.check_auth() {
//         return Ok(ctx.auth_reply());
//     }

//     let addr = &ctx.config().restart_config.address;
//     write_command(Supervisor::Restart, &addr).await
// }

// async fn respawn(ctx: Context) -> impl IntoResponse {
//     if !ctx.check_auth() {
//         return Ok(ctx.auth_reply());
//     }

//     let delay = ctx.data()?.parse::<u16>()?;
//     let addr = &ctx.config().restart_config.address;
//     write_command(Supervisor::Delay(delay), &addr).await
// }

// #[derive(Debug, Copy, Clone)]
// enum Supervisor {
//     Restart,
//     Delay(u16),
// }

// async fn write_command(cmd: Supervisor, addr: &str) -> anyhow::Result<Response> {
//     let mut cmd = match cmd {
//         Supervisor::Restart => "RESTART".into(),
//         Supervisor::Delay(d) => format!("DELAY {}", d),
//     };
//     cmd.push('\0');

//     use tokio::io::*;
//     let mut stream = tokio::net::TcpStream::connect(addr).await?;
//     stream.write_all(cmd.as_bytes()).await?;
//     stream.flush().await?;
//     Ok(Response::empty())
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::irc::{Command, Message, Prefix};

    use futures::prelude::*;
    use tokio::sync::mpsc;

    #[test]
    fn test_ready() {
        let mut ctx = Context::mock_context_msg(Message {
            prefix: None,
            command: Command::Ready,
            args: vec!["test_user".into()],
            data: None,
        });

        ctx.config_mut().irc_config.channels = vec!["#test".into(), "#test2".into()];
        ctx.config_mut().irc_config.q_name = "test_user".into();
        ctx.config_mut().irc_config.q_pass = "hunter2".into();

        let (tx, mut rx) = mpsc::channel(32);
        let noye = Noye::new(tx);

        tokio_test::block_on(async move { ready(ctx, noye).await.unwrap() });

        tokio_test::block_on(async move {
            while let Some(msg) = rx.next().await {
                eprintln!("{}", msg);
            }
        });
    }

    #[test]
    #[ignore]
    fn test_ping() {
        unimplemented!()
    }

    #[test]
    #[ignore]
    fn test_invite() {
        unimplemented!()
    }

    #[test]
    #[ignore]
    fn test_join() {
        unimplemented!()
    }

    #[test]
    #[ignore]
    fn test_part() {
        unimplemented!()
    }

    #[test]
    #[ignore]
    fn test_alt_nick() {
        unimplemented!()
    }

    #[test]
    #[ignore]
    fn test_reclaim() {
        unimplemented!()
    }

    #[test]
    #[ignore]
    fn test_uptime() {
        unimplemented!()
    }
}
