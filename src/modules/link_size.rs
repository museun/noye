use super::*;

pub(super) async fn initialize_module<R>(init: &mut ModuleInit<R>) -> anyhow::Result<()>
where
    R: Responder + Send + 'static,
{
    init.passives.add(link_size);
    Ok(())
}

pub async fn link_size<R: Responder>(context: Context, mut responder: R) -> Result {
    let urls = context.get_links()?;

    // do this 2nd to save a trip to the disk
    let mut client = None;
    let size_limit = context.config().await?.modules.link_size.size_limit;

    let futs: futures::stream::FuturesUnordered<_> = urls
        .iter()
        .enumerate()
        .map(|(i, url)| {
            let client = client.get_or_insert_with(crate::http::new_client).clone();
            async move {
                tokio::time::timeout(
                    tokio::time::Duration::from_secs(10),
                    crate::http::head(client, url.as_str()),
                )
                .await
                .ok()
                .and_then(|resp| resp.ok())
                .and_then(|resp| {
                    resp.headers()
                        .get("Content-Length")?
                        .to_str()
                        .ok()?
                        .parse::<u64>()
                        .ok()
                })
                .filter(|&size| size >= size_limit)
                .map(|size| (i, size))
            }
        })
        .collect();

    let mut res = futs
        .filter_map(|x| async move { x })
        .collect::<Vec<(usize, u64)>>()
        .await;
    res.sort();

    let template = match (res.len(), urls.len()) {
        (0, _) => return util::dont_care(),
        (1, 1) => responses::LinkSize::Single {
            size: res[0].1.as_file_size(),
        },
        (_, _) => responses::LinkSize::Many {
            files: res.into_iter().fold(String::new(), |mut a, (i, size)| {
                if !a.is_empty() {
                    a.push_str(", ")
                }
                a.push_str(&format!("#{}: {}", i + 1, size.as_file_size()));
                a
            }),
        },
    };
    responder.say(context, template).await
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    #[tokio::test]
    async fn link_size() {
        use httptest::{matchers::*, responders::*, Expectation, Server};

        set_snapshot_path();

        let sizes: &[(&str, u64)] = &[
            ("/1kb", 1024),
            ("/1mb", 1024 * 1024),
            ("/10mb", 1024 * 1024 * 10),
        ];

        let server = Server::run();

        for (ep, size) in sizes {
            server.expect(
                Expectation::matching(request::method_path("HEAD", *ep))
                    .times(..)
                    .respond_with(status_code(200).insert_header("Content-Length", *size)),
            );
        }

        let limit = 10 * 1024 * 1024;

        let responses = TestEnv::new(server.url_str("/1mb"))
            .config(|config| config.modules.link_size.size_limit = limit)
            .execute(super::link_size)
            .await;
        responses.expect_empty();

        let responses = TestEnv::new(format!(
            "{} {}",
            server.url_str("/1kb"),
            server.url_str("/1mb")
        ))
        .config(|config| config.modules.link_size.size_limit = limit)
        .execute(super::link_size)
        .await;
        responses.expect_empty();

        let responses = TestEnv::new(server.url_str("/10mb"))
            .config(|config| config.modules.link_size.size_limit = limit)
            .execute(super::link_size)
            .await;
        insta::assert_yaml_snapshot!(responses.get_say::<responses::LinkSize>());
        responses.expect_empty();

        let responses = TestEnv::new(format!(
            "{} {}",
            server.url_str("/1mb"),
            server.url_str("/10mb")
        ))
        .config(|config| config.modules.link_size.size_limit = limit)
        .execute(super::link_size)
        .await;
        insta::assert_yaml_snapshot!(responses.get_say::<responses::LinkSize>());
        responses.expect_empty();
    }
}
