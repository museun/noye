use crate::bot::prelude::*;
use crate::http;
use futures::prelude::*;

registry!("link_size" => {
    passive => LINK_REGEX, link_size;    
});

static LINK_REGEX: &str = r"(?x)
    (?P<link>(?:www|https?)[^\s]+)
";

#[derive(Template, Debug)]
#[parent("link_size")]
enum Output {
    Single { size: String },
    Many { files: String },
}

async fn link_size(context: Context) -> impl IntoResponse {
    let crate::config::LinkSize { size_limit } = context.config().modules_config.link_size;
    let links = context.matches().get_many("link")?;

    let sizes = get_many_sizes(links, size_limit).await;
    let output = match sizes.as_slice() {
        [(_, size)] => Output::Single {
            size: size.as_file_size(),
        },
        [] => return Err(anyhow::anyhow!("cannot get any sizes")),
        many => {
            let sizes = many.iter().fold(String::new(), |mut a, (index, size)| {
                if !a.is_empty() {
                    a.push_str(", ")
                }
                a.push_str(&format!("#{}: {}", index + 1, size.as_file_size()));
                a
            });
            Output::Many { files: sizes }
        }
    };

    Ok(output)
}

async fn get_many_sizes(input: &[String], size_limit: u64) -> Vec<(usize, u64)> {
    let client = std::sync::Arc::new(reqwest::Client::new());

    let fut = concurrent_for_each(
        "link_size",
        input.len(),
        input.iter().enumerate(),
        |(i, link)| {
            let client = client.clone();
            async move {
                let size = get_size(&client, link).await?;
                let res = if size > size_limit {
                    (i, size).into()
                } else {
                    None
                };
                anyhow::Result::<_>::Ok(res)
            }
        },
    );

    let mut sizes = fut
        .await
        .filter_map(|s| async move { s })
        .collect::<Vec<_>>()
        .await;

    sizes.sort();
    sizes
}

async fn get_size(client: &reqwest::Client, link: &str) -> anyhow::Result<u64> {
    let resp = http::head(&client, link).await?;
    let size = resp
        .headers()
        .get("Content-Length")
        .and_then(|header| header.to_str().ok()?.parse::<u64>().ok())
        .ok_or_else(|| anyhow::anyhow!("cannot get valid Content-Length header"))?;
    Ok(size)
}
