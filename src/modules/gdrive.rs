use super::*;
use serde::Deserialize;
use std::collections::HashMap;

pub(super) async fn initialize_module<R>(init: &mut ModuleInit<R>) -> Result
where
    R: Responder + Send + 'static,
{
    init.passives.add(hear_gdrive);

    let client = GDriveClient::new(
        &init.state.config().await?.modules.gdrive.api_key,
        crate::http::new_client(),
    );
    init.state.expect_insert(client)
}

fn filter(url: &url::Url) -> bool {
    const ACCEPTED: [&str; 1] = ["drive.google.com"];
    url.domain().filter(|s| ACCEPTED.contains(s)).is_some()
}

pub async fn hear_gdrive<R: Responder>(context: Context, responder: R) -> Result {
    let state = context.state.lock().await;
    let client = state.expect_get::<GDriveClient>()?.clone();

    let set: futures::stream::FuturesUnordered<_> = context
        .get_links_filter(filter)?
        .into_iter()
        .filter_map(|url| {
            let mut segments = url.path_segments().into_iter().flatten();
            match (segments.next()?, segments.next(), segments.next()) {
                ("d", id, ..) => id.map(ToString::to_string),
                ("file", Some("d"), id) => id.map(ToString::to_string),
                ("open", ..) => url
                    .query_pairs()
                    .collect::<HashMap<_, _>>()
                    .remove("id")
                    .map(|s| s.to_string()),
                _ => return None,
            }
        })
        .map(|id| {
            let (context, responder, client) = (context.clone(), responder.clone(), client.clone());
            async move {
                let resp = client.lookup(&id).await?;

                let created = resp
                    .created_date
                    .map(|dt| time::OffsetDateTime::now() - dt)
                    .map(|t| t.as_readable_time())
                    .unwrap_or_else(|| "just now".into());

                let duration = std::time::Duration::from_millis(
                    resp.video_media_metadata.duration_millis as _,
                )
                .as_timestamp();

                let template = responses::GDrive::Video {
                    title: resp.title,
                    created,
                    size: resp.file_size.as_file_size(),
                    width: resp.video_media_metadata.width,
                    height: resp.video_media_metadata.height,
                    duration,
                };
                { responder }.say(context.clone(), template).await
            }
        })
        .collect();

    set.for_each(|_| async move {}).await;
    Ok(())
}

#[derive(Clone)]
struct GDriveClient {
    ep: Option<String>,
    api_key: std::sync::Arc<String>,
    client: reqwest::Client,
}

impl GDriveClient {
    #[cfg(test)]
    pub fn with_ep(ep: impl ToString, client: reqwest::Client) -> Self {
        Self {
            ep: Some(ep.to_string()),
            api_key: Default::default(),
            client,
        }
    }

    pub fn new(api_key: impl ToString, client: reqwest::Client) -> Self {
        Self {
            ep: None,
            api_key: std::sync::Arc::new(api_key.to_string()),
            client,
        }
    }

    pub async fn lookup(&self, id: &str) -> anyhow::Result<Item> {
        let url = self
            .ep
            .as_ref()
            .map(|ep| format!("{}{}", ep, id))
            .unwrap_or_else(|| format!("https://www.googleapis.com/drive/v2/files/{}", id));

        #[derive(Debug, serde::Serialize)]
        struct Query<'a> {
            key: &'a str,
        }

        crate::http::get_json(
            self.client.clone(),
            &url,
            &Query { key: &self.api_key },
            &[],
        )
        .await
    }
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Item {
    pub id: String,
    pub title: String,
    #[serde(deserialize_with = "crate::util::rfc3339_opt", default)]
    pub created_date: Option<time::OffsetDateTime>,
    pub original_filename: String,
    pub file_extension: String,
    #[serde(deserialize_with = "crate::util::from_str", default)]
    pub file_size: i64,
    pub video_media_metadata: VideoMediaMetadata,
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct VideoMediaMetadata {
    pub width: i64,
    pub height: i64,
    #[serde(deserialize_with = "crate::util::from_str", default)]
    pub duration_millis: i64,
}

#[cfg(test)]
mod tests {
    use crate::test::*;
    use httptest::{matchers::*, responders::*, Expectation, Server};

    #[tokio::test]
    async fn lookup() {
        set_snapshot_path();
        let server = Server::run();

        // redacted
        let id = "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz";
        assert_eq!(id.len(), 33);

        server.expect(
            Expectation::matching(request::method_path("GET", format!("/{}", id)))
                .times(..)
                .respond_with(status_code(200).body(
                    std::fs::read_to_string("./snapshots/inputs/gdrive/input.json").unwrap(),
                )),
        );

        let input = &[
            format!("open?id={}", id),
            format!("d/{}/view", id),
            format!("file/d/{}/view", id),
        ];

        let client = crate::http::new_client();
        for input in input {
            let resp = TestEnv::new(format!("https://drive.google.com/{}", input))
                .insert(super::GDriveClient::with_ep(
                    server.url_str(""),
                    client.clone(),
                ))
                .execute(super::hear_gdrive)
                .await;

            insta::assert_yaml_snapshot!(resp.get_say::<responses::GDrive>(), {
                ".title" => "[title]",
                ".created" => "[created]",
            });
            resp.expect_empty();
        }
    }
}
