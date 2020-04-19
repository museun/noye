use super::*;

table!(RepostTable => "./sql/schema.sql");

pub(super) async fn initialize_module<R>(init: &mut ModuleInit<R>) -> anyhow::Result<()>
where
    R: Responder + Send + 'static,
{
    init.commands.add("ignore", ignore_link)?;
    init.passives.add(repost_shame);
    Ok(())
}

pub async fn ignore_link<R: Responder>(context: Context, mut responder: R) -> Result {
    context.expect_owner(&mut responder).await?;

    let room = context.room();
    let channel = persist::Channel::new(room);

    let link = match context.without_command() {
        Some(args) => args,
        None => {
            let resp = responses::Repost::NoLinkProvided;
            return responder.reply(context, resp).await;
        }
    };

    if !channel.ignore_link(link)? {
        let resp = responses::Repost::AlreadyIgnored;
        return responder.reply(context, resp).await;
    }

    responder.reply(context, responses::Repost::Ignored).await
}

pub async fn repost_shame<R: Responder>(context: Context, mut responder: R) -> Result {
    let staleness = context.config().await?.modules.repost.staleness;

    let secs = simple_duration_parse::parse_secs(&staleness)?;
    let grace = time::Duration::seconds(secs as _);

    let (nick, room) = (context.nick(), context.room());
    let channel = persist::Channel::new(room);

    for updated in context
        .get_links()?
        .into_iter()
        .filter(|url| !channel.is_ignored(url.as_str()))
        .flat_map(|url| channel.insert_link(nick, url.as_str()))
    {
        let time = time::OffsetDateTime::now() - updated.time;
        if time > grace {
            continue;
        }

        let ago = if time.as_seconds_f32() < 1.0 {
            "briefly".into()
        } else {
            time.as_readable_time()
        };

        let other = &updated.nick;
        let res = if !nick.eq_ignore_ascii_case(other) {
            responses::Repost::AlreadyPosted {
                nick: other.clone(),
                count: updated.posts.with_commas(),
                ago,
            }
        } else {
            responses::Repost::SelfPosted {
                count: updated.posts.with_commas(),
                ago,
            }
        };

        responder.say(context.clone(), res).await?;
    }

    Ok(())
}

mod persist;

#[cfg(test)]
mod tests;
