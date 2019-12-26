use super::*;
use futures::prelude::*;

mod data;
use data::*;

mod output;
use output::*;

load_api_key!({ crate::config::GDrive::get_key() });

registry!("gdrive" => {
    passive => GDRIVE_REGEX, hear_gdrive
});

const GDRIVE_REGEX: &str = r"(?x)
    (drive\.google\.com/.*?/(?P<id1>[A-Za-z0-9_-]{33})/?)        # normal regex
        |
    (.*?drive\.google\.com/uc.+?id=(?P<id2>[A-Za-z0-9_-]{33})&?) # this was also there
";

async fn hear_gdrive(context: Context) -> impl IntoResponse {
    let query = Query {
        key: &get_api_key("gdrive")?,
    };

    let ids = context.matches().get_many("id1")?;
    let client = std::sync::Arc::new(surf::Client::new());

    let ok = concurrent_for_each("gdrive", None, ids, |id| {
        let client = client.clone();
        let query = query;
        async move { lookup(&client, &id, query).await }
    })
    .await
    .collect::<Vec<_>>()
    .await;

    Ok(ok)
}

#[derive(serde::Serialize, Copy, Clone)]
struct Query<'a> {
    key: &'a str,
}

async fn lookup<C>(client: &surf::Client<C>, id: &str, query: Query<'_>) -> anyhow::Result<Output>
where
    C: surf::middleware::HttpClient,
{
    let url = format!("https://www.googleapis.com/drive/v2/files/{}", id);
    let item: Item = crate::http::get_json(client, &url, &query, &[]).await?;

    let output = Output::Video {
        title: item.title,
        created: item
            .created_date
            .and_then(|then| {
                (chrono::Utc::now() - then)
                    .to_std()
                    .ok()?
                    .as_readable_time()
                    .into()
            })
            .unwrap_or_else(|| "unknown".to_string()),
        size: item
            .file_size
            .parse::<u64>()
            .unwrap_or_default()
            .as_file_size(),
        width: item.video_media_metadata.width,
        height: item.video_media_metadata.height,
        duration: std::time::Duration::from_millis(
            item.video_media_metadata
                .duration_millis
                .parse::<u64>()
                .unwrap_or_default(),
        )
        .as_timestamp(),
    };
    Ok(output)
}
