use crate::bot::prelude::*;
use crate::http;
use futures::prelude::*;

mod data;
use data::*;

mod output;
use output::*;

load_api_key!({ crate::config::Youtube::get_key() });

registry!("youtube" => {
    passive => VIDEO_REGEX, hear_video;
    passive => CHANNEL_REGEX, hear_channel;
});

const VIDEO_REGEX: &str = r"(?x)
    (youtu\.be/|youtube.com/(\S*(v|video_id)=|v/|e/|embed))([\w\-]{11})
";

const CHANNEL_REGEX: &str = r"(?x)
    (youtu\.be/|youtube.com/(\S*(channel/(?P<channel>[\w\-]+))|(user/(?P<user>[\w\-]+))))
";

async fn hear_video(context: Context) -> impl IntoResponse {
    static BASE: &str = "https://www.googleapis.com/youtube/v3/videos";
    static PART: &str = "statistics,snippet,liveStreamingDetails,contentDetails";
    static FIELDS: &str = "items(id,statistics,liveStreamingDetails,snippet \
                           (title,channelTitle,channelId,liveBroadcastContent,publishedAt), \
                           contentDetails(duration,regionRestriction))";

    let pairs = context.data()?.split(' ').flat_map(find_vid_ts);

    let client = std::sync::Arc::new(reqwest::Client::new());
    let output = concurrent_for_each("youtube", None, pairs, |(id, ts)| {
        let client = client.clone();
        async move {
            let item = lookup(&client, &id, BASE, PART, FIELDS).await?;
            anyhow::Result::<_>::Ok(Output::from_video(ts.as_deref(), item))
        }
    })
    .await;

    Ok(output.collect::<Vec<_>>().await)
}

async fn hear_channel(context: Context) -> impl IntoResponse {
    static BASE: &str = "https://www.googleapis.com/youtube/v3/channels";
    static PART: &str = "snippet,statistics";
    static FIELDS: &str = "items(id,snippet(title,description,publishedAt),statistics,status)";

    let iter = context.matches().gather(&["channel", "user"])?;

    let client = std::sync::Arc::new(reqwest::Client::new());
    let output = concurrent_for_each("youtube", None, iter, |cid| {
        let client = client.clone();
        async move {
            let item = lookup(&client, &cid, BASE, PART, FIELDS).await?;
            anyhow::Result::<_>::Ok(Output::from_channel(item))
        }
    })
    .await;

    Ok(output.collect::<Vec<_>>().await)
}

async fn lookup(
    client: &reqwest::Client,
    id: &str,
    base: &str,
    part: &str,
    fields: &str,
) -> anyhow::Result<Item> {
    #[derive(serde::Serialize)]
    struct Query<'a> {
        id: &'a str,
        part: &'a str,
        fields: &'a str,
        key: &'a str,
    }

    let query = Query {
        id,
        part,
        fields,
        key: &get_api_key("youtube")?,
    };

    #[derive(serde::Deserialize)]
    struct Items {
        items: Vec<Item>,
    }

    let mut items: Items = http::get_json(&client, base, &query, &[])
        .map_err(|err| anyhow::anyhow!("cannot get id: {}", id).context(err))
        .await?;

    items
        .items
        .pop()
        .ok_or_else(|| anyhow::anyhow!("cannot get response for: {}", id))
}

// TODO rewrite this so it'll just borrow from the input
fn find_vid_ts(input: &str) -> Option<(String, Option<String>)> {
    use std::convert::TryFrom as _;
    let url = crate::http::Url::try_from(input).ok()?;
    let short = match url.domain() {
        "www.youtube.com" | "youtube.com" => false,
        "youtu.be" => true,
        _ => return None,
    };
    let path = url.path();
    let pair = if short && path.starts_with('/') && path.len() == 12 {
        let ts = url.query("t");
        (path[1..].to_string(), ts.map(|s| s.to_string()))
    } else {
        let id = url.query("v");
        let ts = url.query("t");
        ((*id?).to_string(), ts.map(|s| s.to_string()))
    };
    pair.into()
}

// TODO unit tests
