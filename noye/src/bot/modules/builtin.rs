use super::prelude::*;

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
    command  => "!restart",             restart;
    command  => "!respawn",             respawn;
});

async fn ready(context: Context) -> impl IntoResponse {
    let config::Irc {
        channels,
        q_name,
        q_pass,
        ..
    } = &context.config().irc_config;

    let mut responses = vec![];
    if !q_name.is_empty() && !q_pass.is_empty() {
        let auth = format!("PRIVMSG Q@CServe.quakenet.org :AUTH {} {}", q_name, q_pass,);
        let mode = format!("MODE {} +x", context.arg(0).unwrap(),);
        responses.push(Response::raw(auth));
        responses.push(Response::raw(mode));
    }
    let autojoin = channels
        .iter()
        .inspect(|channel| log::debug!("autojoin: {}", channel))
        .map(Response::join);
    responses.extend(autojoin);
    responses
}

async fn ping(context: Context) -> impl IntoResponse {
    Ok(Response::raw(format!("PONG {}", context.data()?)))
}

async fn invite(context: Context) -> impl IntoResponse {
    context.data().map(Response::join)
}

async fn join(context: Context) -> impl IntoResponse {
    Some(Response::join(context.command()?.args?))
}

async fn part(context: Context) -> impl IntoResponse {
    context.target_channel().map(Response::part)
}

async fn alt_nick(context: Context) -> impl IntoResponse {
    Some(Response::nick(format!("{}_", context.arg(1)?)))
}

async fn reclaim(context: Context) -> impl IntoResponse {
    let config::Irc { nick, .. } = &context.config().irc_config;
    let nick = context.nick().filter(|sender| sender == nick)?;
    Some(Response::nick(nick))
}

async fn uptime(_: Context) -> impl IntoResponse {
    START.elapsed().as_readable_time()
}

async fn restart(ctx: Context) -> impl IntoResponse {
    if !ctx.check_auth() {
        return Ok(ctx.auth_reply());
    }

    let addr = &ctx.config().restart_config.address;
    write_command(Supervisor::Restart, &addr).await
}

async fn respawn(ctx: Context) -> impl IntoResponse {
    if !ctx.check_auth() {
        return Ok(ctx.auth_reply());
    }

    let delay = ctx.data()?.parse::<u16>()?;
    let addr = &ctx.config().restart_config.address;
    write_command(Supervisor::Delay(delay), &addr).await
}

#[derive(Debug, Copy, Clone)]
enum Supervisor {
    Restart,
    Delay(u16),
}

async fn write_command(cmd: Supervisor, addr: &str) -> anyhow::Result<Response> {
    let mut cmd = match cmd {
        Supervisor::Restart => "RESTART".into(),
        Supervisor::Delay(d) => format!("DELAY {}", d),
    };
    cmd.push('\0');

    use tokio::io::*;
    let mut stream = tokio::net::TcpStream::connect(addr).await?;
    stream.write_all(cmd.as_bytes()).await?;
    stream.flush().await?;
    Ok(Response::empty())
}
