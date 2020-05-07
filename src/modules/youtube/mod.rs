use super::*;
use std::collections::HashMap;

mod client;
mod data;

pub(super) async fn initialize_module<R>(init: &mut ModuleInit<R>) -> anyhow::Result<()>
where
    R: Responder + Send + 'static,
{
    init.passives.add(hear_video);
    init.passives.add(hear_channel);

    let client = client::YoutubeClient::new(&init.state.config().await?.modules.youtube.api_key);
    init.state.expect_insert(client)
}

fn filter(url: &url::Url) -> bool {
    const ACCEPTED: [&str; 6] = [
        "youtube.com",
        "youtu.be",
        "youtube.jp",
        "www.youtube.com",
        "www.youtu.be",
        "www.youtube.jp",
    ];

    url.domain().filter(|s| ACCEPTED.contains(s)).is_some()
}

pub async fn hear_video<R: Responder>(context: Context, responder: R) -> Result {
    fn get_vid_ts(url: url::Url) -> Option<(String, Option<String>)> {
        let map = url.query_pairs().collect::<HashMap<_, _>>();
        match url.path() {
            "/watch" => map.get("v").map(ToString::to_string),
            d if d.len() == 12 => Some(d[1..].to_string()),
            _ => return None,
        }
        .map(|vid| (vid, map.get("t").map(ToString::to_string)))
    }

    let state = context.state.lock().await;
    // TODO why is this in the state? (see how the vimeo modules does it so it can be lazy)
    let client = state.expect_get::<client::YoutubeClient>()?;

    let set = context
        .get_links_filter(filter)?
        .into_iter()
        .filter_map(get_vid_ts)
        .map(|(vid, ts)| {
            let (context, responder, client) = (context.clone(), responder.clone(), client.clone());
            async move {
                let video = client.lookup_video(&vid).await?;
                let template = make_resp_for_video(video, ts);
                { responder }.say(context, template).await
            }
        })
        .collect::<futures::stream::FuturesUnordered<_>>();

    set.for_each(|_| async move {}).await;
    Ok(())
}

pub async fn hear_channel<R: Responder>(context: Context, responder: R) -> Result {
    let state = context.state.lock().await;
    // TODO why is this in the state? (see how the vimeo modules does it so it can be lazy)
    let client = state.expect_get::<client::YoutubeClient>()?;

    let set = context
        .get_links_filter(filter)?
        .into_iter()
        .filter_map(|url| {
            let mut path = url.path_segments().into_iter().flatten();
            let channel = match (path.next()?, path.next()?) {
                ("channel", id) => client::Channel::Channel(id.to_string()),
                ("user", id) => client::Channel::User(id.to_string()),
                _ => return None,
            };
            channel.into()
        })
        .map(|channel| {
            let (context, responder, client) = (context.clone(), responder.clone(), client.clone());
            async move {
                let item = client.lookup_channel(channel).await?;
                let template = make_resp_for_channel(item);
                { responder }.say(context, template).await
            }
        })
        .collect::<futures::stream::FuturesUnordered<_>>();

    set.for_each(|_| async move {}).await;
    Ok(())
}

pub fn make_resp_for_channel(channel: data::Item) -> responses::Youtube {
    responses::Youtube::Channel {
        id: channel.id,
        title: channel.snippet.title,
        videos: channel.statistics.video_count.with_commas(),
        views: channel.statistics.view_count.with_commas(),
    }
}

pub fn make_resp_for_video(video: data::Item, ts: Option<String>) -> responses::Youtube {
    let ts = ts.map(|s| format!("?t={}", s));

    return match video.snippet.live_broadcast_content {
        data::LiveBroadcastContent::Live => for_live(video, ts.unwrap_or_default()),
        data::LiveBroadcastContent::Upcoming => for_upcoming(video),
        data::LiveBroadcastContent::None => for_none(video, ts.unwrap_or_default()),
    };

    fn for_live(video: data::Item, ts: String) -> responses::Youtube {
        responses::Youtube::Live {
            viewers: video
                .live_streaming_details
                .unwrap()
                .concurrent_viewers
                .with_commas(),
            channel: video.snippet.channel_title,
            id: video.id,
            title: video.snippet.title,
            ts,
        }
    }

    fn for_upcoming(video: data::Item) -> responses::Youtube {
        responses::Youtube::Upcoming {
            start: video
                .live_streaming_details
                .unwrap()
                .scheduled_start_time
                .and_then(|start| {
                    (start - time::OffsetDateTime::now_utc())
                        .as_readable_time()
                        .into()
                })
                .unwrap_or_else(|| "unknown start time".into()),
            channel: video.snippet.channel_title,
            id: video.id,
            title: video.snippet.title,
        }
    }

    fn for_none(video: data::Item, ts: String) -> responses::Youtube {
        responses::Youtube::Video {
            duration: video
                .content_details
                .unwrap()
                .duration
                .from_iso8601()
                .as_timestamp(),
            channel: video.snippet.channel_title,
            id: video.id,
            title: video.snippet.title,
            views: video.statistics.view_count.with_commas(),
            ts,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    #[tokio::test]
    async fn video() {
        use httptest::{matchers::*, responders::*, Expectation, Server};
        set_snapshot_path();

        let server = Server::run();

        server.expect(
            Expectation::matching(request::query(url_decoded(contains(("id", "JzDQj4X17gI")))))
                .times(..)
                .respond_with(status_code(200).body(
                    std::fs::read_to_string("./snapshots/inputs/youtube/JzDQj4X17gI.json").unwrap(),
                )),
        );

        let videos = &[
            "https://www.youtube.com/watch?v=JzDQj4X17gI",
            "https://youtu.be/JzDQj4X17gI",
            "https://www.youtube.com/watch?v=JzDQj4X17gI&list=TLPQMTMwNDIwMjDXaIbNqEN7hQ&index=1",
        ];

        let client = super::client::YoutubeClient::new_with_ep(server.url_str(""));
        for link in videos {
            let responses = TestEnv::new(link)
                .insert(client.clone())
                .execute(super::hear_video)
                .await;

            insta::assert_yaml_snapshot!(responses.get_say::<responses::Youtube>(), {
                ".views" => "[views]",
                ".title" => "[title]",
                ".duration" => "[duration]",
                ".channel" => "[channel]"
            });
            responses.expect_empty();
        }

        let responses = TestEnv::new(videos.join(" "))
            .insert(client.clone())
            .execute(super::hear_video)
            .await;
        insta::assert_yaml_snapshot!(responses.get_say::<responses::Youtube>(), {
            ".views" => "[views]",
            ".title" => "[title]",
            ".duration" => "[duration]",
            ".channel" => "[channel]"
        });
        insta::assert_yaml_snapshot!(responses.get_say::<responses::Youtube>(), {
            ".views" => "[views]",
            ".title" => "[title]",
            ".duration" => "[duration]",
            ".channel" => "[channel]"
        });
        insta::assert_yaml_snapshot!(responses.get_say::<responses::Youtube>(), {
            ".views" => "[views]",
            ".title" => "[title]",
            ".duration" => "[duration]",
            ".channel" => "[channel]"
        });
        responses.expect_empty();

        let other = &[
            "https://www.youtube.com/channel/UC6FadPgGviUcq6VQ0CEJqdQ",
            "https://www.youtube.com/user/sugoooi9/videos",
            "http://google.com",
            "youtube.com/asdf",
            "https://example.com",
            "ftp://foo@bar:localhost:1234",
        ];

        for other in other {
            let responses = TestEnv::new(other)
                .insert(client.clone())
                .execute(super::hear_video)
                .await;
            responses.expect_empty();
        }

        let responses = TestEnv::new(other.join(" "))
            .insert(client)
            .execute(super::hear_video)
            .await;
        responses.expect_empty();
    }

    #[tokio::test]
    async fn upcoming() {
        use httptest::{matchers::*, responders::*, Expectation, Server};
        set_snapshot_path();

        let server = Server::run();

        for id in &["LFpF4jPfnpo", "WsRUC73L-MA"] {
            server.expect(
                Expectation::matching(request::query(url_decoded(contains(("id", *id)))))
                    .respond_with(
                        status_code(200).body(
                            std::fs::read_to_string(format!(
                                "./snapshots/inputs/youtube/{}.json",
                                id
                            ))
                            .unwrap(),
                        ),
                    ),
            );
        }

        let client = super::client::YoutubeClient::new_with_ep(server.url_str(""));

        let responses = TestEnv::new("https://www.youtube.com/watch?v=LFpF4jPfnpo")
            .insert(client.clone())
            .execute(super::hear_video)
            .await;
        insta::assert_yaml_snapshot!(responses.get_say::<responses::Youtube>(), {
            ".start" => "[start]",
            ".title" => "[title]",
            ".channel" => "[channel]"
        });
        responses.expect_empty();

        let responses = TestEnv::new("https://www.youtube.com/watch?v=WsRUC73L-MA")
            .insert(client.clone())
            .execute(super::hear_video)
            .await;
        insta::assert_yaml_snapshot!(responses.get_say::<responses::Youtube>(), {
            ".title" => "[title]",
            ".channel" => "[channel]",
            ".viewers" => "[viewers]"
        });
        responses.expect_empty();
    }

    #[tokio::test]
    async fn channel() {
        use httptest::{matchers::*, responders::*, Expectation, Server};
        set_snapshot_path();

        let server = Server::run();

        let inputs = &[
            ("id", "UC6FadPgGviUcq6VQ0CEJqdQ"),
            ("forUsername", "sugoooi9"),
        ];

        for (kind, ep) in inputs {
            server.expect(
                Expectation::matching(request::query(url_decoded(contains((*kind, *ep)))))
                    .respond_with(
                        status_code(200).body(
                            std::fs::read_to_string(format!(
                                "./snapshots/inputs/youtube/{}.json",
                                ep
                            ))
                            .unwrap(),
                        ),
                    ),
            );
        }

        let links = &[
            "https://www.youtube.com/channel/UC6FadPgGviUcq6VQ0CEJqdQ",
            "https://www.youtube.com/user/sugoooi9/videos",
        ];

        let client = super::client::YoutubeClient::new_with_ep(server.url_str(""));
        for link in links {
            let responses = TestEnv::new(link)
                .insert(client.clone())
                .execute(super::hear_channel)
                .await;

            insta::assert_yaml_snapshot!(responses.get_say::<responses::Youtube>(), {
                ".views" => "[views]",
                ".videos" => "[videos]",
                ".title" => "[title]",
            });
            responses.expect_empty();
        }
    }
}
