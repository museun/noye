use super::*;

use rand::prelude::*;
use std::collections::BTreeMap;
pub mod web;

pub(super) async fn initialize_module<R>(init: &mut ModuleInit<R>) -> anyhow::Result<()>
where
    R: Responder + Send + 'static,
{
    let mut mapping = web::Mapping::default();
    for (key, val) in &init.state.config().await?.modules.pictures.directories {
        let mut entry = web::Entry::new(key);
        for channel in &val.banned_channels {
            entry.blacklist(channel);
        }
        let count = entry.index(&val.directory);
        log::debug!("adding {} pictures from {} for {}", count, key, val.command);
        mapping.insert(entry);
    }

    init.state.expect_insert(web::Db::new(mapping))?;
    init.state.expect_insert(LastSent {
        instant: tokio::time::Instant::now(),
        lines: 0,
    })?;

    init.commands.add("pictures", pictures)?;
    init.passives.add(hear_passive);

    Ok(())
}

pub async fn pictures<R: Responder>(context: Context, mut responder: R) -> Result {
    async fn list<R: Responder>(context: Context, mut responder: R) -> Result {
        let commands = {
            let state = context.state.lock().await;
            let db = state.expect_get::<web::Db>()?.inner.read().await;

            let mut v = db.names().collect::<Vec<_>>();
            v.sort();

            v.into_iter().fold(String::new(), |mut a, c| {
                if !a.is_empty() {
                    a.push_str(", ")
                }
                a.push_str("!");
                a.push_str(c);
                a
            })
        };

        let resp = responses::Pictures::Listing { commands };
        responder.reply(context, resp).await
    }

    async fn refresh<R: Responder>(context: Context, mut responder: R) -> Result {
        let mut result = BTreeMap::new();
        {
            let dirs = context.config().await?.modules.pictures.directories;
            let state = context.state.lock().await;
            let mut mapping = state.expect_get::<web::Db>()?.inner.write().await;
            mapping.clear();

            let mut set: futures::stream::FuturesUnordered<_> = Default::default();
            for (k, v) in dirs {
                // we're only going to add /new/ entries
                if mapping.contains(&k) {
                    continue;
                }

                let mut entry = web::Entry::new(&k);
                for channel in &v.banned_channels {
                    entry.blacklist(channel);
                }

                set.push(tokio::task::spawn_blocking(move || {
                    let count = entry.index(&v.directory);
                    log::debug!("adding {} pictures from for {}/{}", count, k, v.command);
                    (k, count, entry)
                }));
            }

            while let Some(el) = set.next().await {
                let (key, count, entry) = el.unwrap(); // TODO not this
                result.insert(key, count);
                mapping.insert(entry);
            }
        }

        let new = result.into_iter().fold(String::new(), |mut s, (k, v)| {
            if !s.is_empty() {
                s.push_str(", ");
            }
            s.push_str(&format!("{} ({} pictures)", k, v.with_commas()));
            s
        });
        if !new.is_empty() {
            let resp = responses::Pictures::Refreshed { new };
            return responder.reply(context, resp).await;
        }
        Ok(())
    }

    let args = context.command_args();
    match args.as_slice() {
        ["refresh"] => refresh(context, responder).await,
        d if !d.is_empty() => {
            let command = args.join(" ");
            let resp = responses::Builtin::UnknownSubcommand { command };
            responder.reply(context, resp).await
        }
        _ => list(context, responder).await,
    }
}

pub async fn hear_passive<R: Responder>(context: Context, mut responder: R) -> Result {
    let crate::config::Pictures {
        cooldown,
        mention_chance,
        passive_chance,
        min_lines,
        quiet_time,
        ..
    } = context.config().await?.modules.pictures;

    let mut state = context.state.lock().await;

    let ExternalIp { address, port } = state.expect_get::<ExternalIp>()?.clone();
    let db = state.expect_get::<web::Db>()?.clone();

    let mut rng = rand::rngs::SmallRng::from_entropy();

    macro_rules! respond {
        (say => $path:expr, $id:expr) => {{
            let link = format!("http://{}:{}/p/{}/{}", address, port, $path, $id);
            respond!(@inner responses::Pictures::Say { link })
        }};
        (reply => $path:expr, $id:expr) => {{
            let link = format!("http://{}:{}/p/{}/{}", address, port, $path, $id);
            respond!(@inner responses::Pictures::Reply { link })
        }};
        (@inner $link:expr) => {{
            let _ = state.insert(LastSent {
                instant: tokio::time::Instant::now(),
                lines: 0,
            });
            return responder.say(context.clone(), $link).await;
        }};
    }

    if let Some(cmd) = context.command() {
        let mut mapping = db.inner.write().await;
        if let Some((path, id)) = mapping.choose(&mut rng, context.room(), cmd) {
            respond!(reply => path, id);
        }
    }

    let mut last = state.expect_get::<LastSent>()?.clone();
    last.lines += 1;

    if last.lines < min_lines {
        let _ = state.insert(last);
        return crate::util::dont_care();
    }

    let left = last.instant.elapsed();
    if left < tokio::time::Duration::from_secs(simple_duration_parse::parse_secs(&cooldown)?) {
        return crate::util::dont_care();
    }

    let bypass =
        left >= tokio::time::Duration::from_secs(simple_duration_parse::parse_secs(&quiet_time)?);

    if bypass || rng.gen_bool(mention_chance) {
        let mut mapping = db.inner.write().await;
        let mut parts = context.parts().collect::<Vec<_>>();
        parts.shuffle(&mut rng);
        for part in parts {
            if let Some((path, id)) = mapping.choose(&mut rng, context.room(), part) {
                respond!(say => path, id);
            }
        }
    }

    if bypass || rng.gen_bool(passive_chance) {
        let mut mapping = db.inner.write().await;
        if let Some((path, id)) = mapping.choose(&mut rng, context.room(), None) {
            respond!(say => path, id)
        }
    }

    Ok(())
}

#[derive(Clone, Debug)]
struct LastSent {
    instant: tokio::time::Instant,
    lines: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn pictures() {
        set_snapshot_path();
        let responses = TestEnv::new("!pictures")
            .insert(web::Db::new({
                let mut mapping = web::Mapping::default();
                mapping.insert(web::Entry::new("foo"));
                mapping.insert(web::Entry::new("bar"));
                mapping.insert(web::Entry::new("baz"));
                mapping
            }))
            .execute(super::pictures)
            .await;
        insta::assert_yaml_snapshot!(responses.get_reply::<responses::Pictures>());
        responses.expect_empty();

        let responses = TestEnv::new("!pictures foobar")
            .execute(super::pictures)
            .await;
        insta::assert_yaml_snapshot!(responses.get_reply::<responses::Builtin>());
        responses.expect_empty();

        let responses = TestEnv::new("!pictures foobar baz quux")
            .execute(super::pictures)
            .await;
        insta::assert_yaml_snapshot!(responses.get_reply::<responses::Builtin>());
        responses.expect_empty();

        let db = web::Db::default();

        let responses = TestEnv::new("!pictures refresh")
            .insert(db.clone())
            .config(|config| {
                config.modules.pictures.directories = {
                    let mut map = HashMap::new();
                    map.insert(
                        "youtube".into(),
                        crate::config::PicturesItem {
                            directory: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                                .join("src")
                                .join("modules")
                                .join("youtube")
                                .to_str()
                                .unwrap()
                                .to_string(),
                            command: "youtube".into(),
                            banned_channels: Default::default(),
                        },
                    );
                    map
                }
            })
            .execute(super::pictures)
            .await;

        insta::assert_yaml_snapshot!(responses.get_reply::<responses::Pictures>(), { ".new" => "[new]" });
        responses.expect_empty();

        let responses = TestEnv::new("!pictures refresh")
            .insert(db.clone())
            .config(|config| {
                config.modules.pictures.directories = {
                    let mut map = HashMap::new();
                    map.insert(
                        "pictures".into(),
                        crate::config::PicturesItem {
                            directory: std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                                .join("src")
                                .join("modules")
                                .join("pictures")
                                .to_str()
                                .unwrap()
                                .to_string(),
                            command: "pictures".into(),
                            banned_channels: Default::default(),
                        },
                    );
                    map
                }
            })
            .execute(super::pictures)
            .await;

        insta::assert_yaml_snapshot!(responses.get_reply::<responses::Pictures>(), { ".new" => "[new]" });
        responses.expect_empty();

        let responses = TestEnv::new("!pictures")
            .insert(db)
            .execute(super::pictures)
            .await;
        insta::assert_yaml_snapshot!(responses.get_reply::<responses::Pictures>());
        responses.expect_empty();
    }
}
