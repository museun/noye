use crate::bot::prelude::*;
use futures::prelude::*;

registry!("vimeo" => {
    passive => VIMEO_REGEX, hear_vimeo
});

static VIMEO_REGEX: &str = r"(?x)
    vimeo\.com/(?P<vid>\d+)
";

#[derive(Template, Debug)]
#[parent("vimeo")]
enum Output {
    Video {
        id: i64,
        width: i64,
        height: i64,
        duration: String,
        fps: String,
        title: String,
        owner: String,
    },
}

async fn hear_vimeo(context: Context) -> impl IntoResponse {
    let vids = context.matches().get_many("vid")?;

    let client = std::sync::Arc::new(reqwest::Client::new());
    let vec = concurrent_for_each("vimeo", None, vids, |vid| {
        let client = client.clone();
        async move { lookup(&client, vid).await }
    })
    .await
    .collect::<Vec<_>>()
    .await;

    Ok(vec)
}

async fn lookup(client: &reqwest::Client, vid: &str) -> anyhow::Result<Output> {
    #[derive(Debug, serde::Deserialize)]
    struct Response {
        video: Video,
    }

    #[derive(Debug, serde::Deserialize)]
    struct Video {
        id: i64,
        width: i64,
        height: i64,
        duration: i64,
        fps: f64,
        title: String,
        owner: Owner,
    }

    #[derive(Debug, serde::Deserialize)]
    struct Owner {
        name: String,
    }

    let url = format!("https://player.vimeo.com/video/{}/config", vid);
    let video = crate::http::get_json(
        &client,
        &url,
        &crate::http::NoQuery,
        &[("Accept-Encoding", "identity")],
    )
    .map_ok(|resp: Response| resp.video)
    .await?;

    let output = Output::Video {
        id: video.id,
        width: video.width,
        height: video.height,
        duration: video.duration.as_timestamp(),
        fps: video.fps.to_string(),
        title: video.title,
        owner: video.owner.name,
    };
    Ok(output)
}
