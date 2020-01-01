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

async fn link_size(context: Context, mut noye: Noye) -> anyhow::Result<()> {
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

    noye.say_template(context, output)
}

async fn get_many_sizes(input: &[String], size_limit: u64) -> Vec<(usize, u64)> {
    let client = http::new_client();

    let mut sizes = concurrent_map(
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
    )
    .await
    .filter_map(|s| async move { s })
    .collect::<Vec<_>>()
    .await;

    sizes.sort();
    sizes
}

async fn get_size(client: &reqwest::Client, link: &str) -> anyhow::Result<u64> {
    http::head(&client, link)
        .await?
        .headers()
        .get("Content-Length")
        .and_then(|header| header.to_str().ok()?.parse().ok())
        .ok_or_else(|| anyhow::anyhow!("cannot get valid Content-Length header"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bot::test::*;

    // TODO mock this

    #[test]
    #[ignore] // disabled so we don't hit that ep every time we run a test
    fn one_big_file() {
        let ctx = Context::mock_context_regex("https://speed.hetzner.de/100MB.bin", LINK_REGEX);
        let resp = say_template(
            ctx.clone(),
            Output::Single {
                size: "100.00 MB".into(),
            },
        );

        check(super::link_size, ctx, vec![&resp]);
    }

    #[test]
    #[ignore] // disabled so we don't hit that ep every time we run a test
    fn multiple_big_files() {
        let ctx = Context::mock_context_regex(
            "https://speed.hetzner.de/100MB.bin https://speed.hetzner.de/1GB.bin",
            LINK_REGEX,
        );

        let resp = say_template(
            ctx.clone(),
            Output::Many {
                files: "#1: 100.00 MB, #2: 1000.00 MB".into(),
            },
        );

        check(super::link_size, ctx, vec![&resp]);
    }

    #[test]
    #[ignore] // disabled so we don't hit that ep every time we run a test
    fn multiple_mix_big_files() {
        let mut ctx = Context::mock_context_regex(
            "https://speed.hetzner.de/100MB.bin http://speedtest.tele2.net/1MB.zip https://speed.hetzner.de/1GB.bin",
            LINK_REGEX,
        );

        ctx.config_mut().modules_config.link_size.size_limit = 1024 * 1024 * 10;

        let resp = say_template(
            ctx.clone(),
            Output::Many {
                files: "#1: 100.00 MB, #3: 1000.00 MB".into(),
            },
        );

        check(super::link_size, ctx, vec![&resp]);
    }
}
