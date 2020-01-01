use crate::bot::prelude::*;
use crate::http;
use futures::prelude::*;
use once_cell::sync::Lazy;

registry!("link_size" => {
    passive => LINK_REGEX, hear_instagram;    
});

static LINK_REGEX: &str = r"(?x)
    (?:www|https?)?instagram\.com/p/(?P<id>[^\s/]+)/?
";

#[derive(Template, Debug)]
#[parent("instagram")]
enum Output {
    Post { name: String, display: String },
}

static DISPLAY_REGEX: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r##"full_name":\s?"(?P<name>.*?)"##).unwrap());

static NAME_REGEX: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r##"<meta content=".*?\s\((?P<name>@.*?)\)\s"##).unwrap());

async fn hear_instagram(context: Context, mut noye: Noye) -> anyhow::Result<()> {
    let iter = context.matches().get_many("id")?;
    let client = http::new_client();
    let mut stream = concurrent_map("instagram", None, iter, |id| {
        let client = client.clone();
        async move { fetch_info(&client, id).await }
    })
    .await;

    while let Some(resp) = stream.next().await {
        noye.say_template(&context, resp)?;
    }

    noye.nothing()
}

async fn fetch_info(client: &reqwest::Client, id: &str) -> anyhow::Result<Output> {
    let url = format!("https://www.instagram.com/p/{}/", id);
    let ua = &[(
        "User-Agent",
        "Mozilla/5.0 (Windows NT 6.1; WOW64; rv:54.0) Gecko/20100101 Firefox/71.0", // TODO don't spoof firefox
    )];
    let body = crate::http::get_body(&client, &url, ua).await?;
    get_title_and_name(&id, &body)
        .map(|(display, name)| Output::Post { name, display })
        .ok_or_else(|| anyhow::anyhow!("cannot get display info"))
}

fn get_title_and_name(id: &str, body: &str) -> Option<(String, String)> {
    let title = match body
        .find("<title>")
        .and_then(|start| {
            let end = body.find("</title>")?;
            let s = &body[start + 7..end];
            let mid = s.find(" on Instagram: ")?;
            s[..mid].trim().into()
        })
        .and_then(|s| escaper::decode_html(s).ok())
    {
        Some(title) => title,
        None => {
            log::warn!("no title found for instagram id: {}", id);
            return None;
        }
    };

    let name = match NAME_REGEX
        .captures(&body)
        .and_then(|re| re.name("name")?.as_str().into())
        .or_else(|| {
            DISPLAY_REGEX
                .captures(&body)
                .and_then(|re| re.name("name")?.as_str().into())
        }) {
        Some(name) => name.into(),
        None => {
            log::warn!("no name found for instagram id: {}", id);
            return None;
        }
    };

    (title, name).into()
}

// TODO add unit tests for this
