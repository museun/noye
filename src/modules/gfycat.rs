use super::*;

pub(super) async fn initialize_module<R>(init: &mut ModuleInit<R>) -> anyhow::Result<()>
where
    R: Responder + Send + 'static,
{
    init.passives.add(gfycat);

    Ok(())
}

fn filter(url: &url::Url) -> bool {
    const ACCEPTED: [&str; 1] = ["gfycat.com"];
    url.domain().filter(|s| ACCEPTED.contains(s)).is_some()
}

pub async fn gfycat<R: Responder>(context: Context, mut responder: R) -> Result {
    use crate::http::client::*;

    let mut client = None;

    let api_key = context.config().await?.modules.gfycat.api_key;

    #[derive(serde::Deserialize)]
    struct Wrapper {
        #[serde(rename = "gfyItem")]
        item: Item,
    }

    for link in context.get_links_filter(filter)? {
        let client = client.get_or_insert_with(new_client).clone();

        let id = match link.path().splitn(2, '-').next().and_then(|s| {
            let s = &s[1..];
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        }) {
            Some(id) => id,
            None => continue,
        };

        let item: Wrapper = get_json(
            client,
            &format!("https://api.gfycat.com/v1/gfycats/{}", id),
            &NoQuery,
            &[("Authorization", &format!("Bearer {}", api_key))],
        )
        .await?;

        let Item {
            tags,
            title,
            size,
            width,
            height,
            framerate,
            name,
            nsfw,
            has_audio,
            ..
        } = item.item;

        let size = size.as_file_size();
        let framerate = format!("{:.02}", framerate);
        let link = format!("https://gfycat.com/{}", name);

        let meta = if has_audio { " (has audio) " } else { " " };
        let meta = if tags.is_empty() {
            meta.to_string()
        } else {
            format!("[{}]", tags.join(","))
        };

        let resp = if nsfw.filter(|s| s == "1").is_some() {
            crate::responses::Gfycat::LinkNsfw {
                title,
                size,
                width: width as _,
                height: height as _,
                framerate,
                link,
                meta,
            }
        } else {
            crate::responses::Gfycat::Link {
                title,
                size,
                width: width as _,
                height: height as _,
                framerate,
                link,
                meta,
            }
        };

        responder.say(context.clone(), resp).await?;
    }

    Ok(())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Item {
    tags: Vec<String>,
    title: String,
    nsfw: Option<String>,
    views: i64,
    #[serde(rename = "hasAudio")]
    has_audio: bool,
    #[serde(rename = "gfyName")]
    name: String,
    #[serde(rename = "gfySlug")]
    slug: Option<String>,
    width: i64,
    height: i64,
    #[serde(rename = "frameRate")]
    framerate: f32,
    #[serde(rename = "numFrames")]
    frame_count: f32,
    #[serde(rename = "webmSize")]
    size: i64,
    #[serde(rename = "createDate")]
    create_date: i64,
}
