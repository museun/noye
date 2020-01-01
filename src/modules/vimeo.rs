use crate::bot::prelude::*;
use crate::http;
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

async fn hear_vimeo(context: Context, mut noye: Noye) -> anyhow::Result<()> {
    let vids = context.matches().get_many("vid")?;

    let client = http::new_client();
    let mut stream = concurrent_map("vimeo", None, vids, |vid| {
        let client = client.clone();
        async move { lookup(&client, vid).await }
    })
    .await;

    while let Some(resp) = stream.next().await {
        noye.say_template(&context, resp)?;
    }

    noye.nothing()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bot::test::*;

    // TODO mock this

    #[test]
    #[ignore]
    fn hear_vimeo() {
        let tests = vec![
            (
                "https://vimeo.com/23960970",
                Output::Video {
                    id: 23_960_970,
                    width: 1280,
                    height: 720,
                    duration: "02:32".into(),
                    fps: "25".into(),
                    title: "NuFormer - 3D video mapping interactivity test. April 2011".into(),
                    owner: "NuFormer".into(),
                },
            ),
            (
                "https://vimeo.com/220883711",
                Output::Video {
                    id: 220_883_711,
                    width: 1920,
                    height: 1080,
                    duration: "03:11".into(),
                    fps: "25".into(),
                    title: "Mixed Reality - THEORIZ - RnD test 002".into(),
                    owner: "THÉORIZ".into(),
                },
            ),
            (
                "https://vimeo.com/16750764",
                Output::Video {
                    id: 16_750_764,
                    width: 1280,
                    height: 720,
                    duration: "02:40".into(),
                    fps: "24".into(),
                    title: "D7000 Video Test - Film Look".into(),
                    owner: "Melo".into(),
                },
            ),
        ];

        for (input, expected) in tests {
            let ctx = Context::mock_context_regex(input, VIMEO_REGEX);
            let resp = say_template(&ctx, expected);
            check(super::hear_vimeo, ctx, vec![&resp])
        }
    }
}
