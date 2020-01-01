use crate::bot::prelude::*;
use crate::http;
use futures::prelude::*;

load_api_key!({ crate::config::GDrive::get_key() });

registry!("gdrive" => {
    passive => GDRIVE_REGEX, hear_gdrive
});

const GDRIVE_REGEX: &str = r"(?x)
    (drive\.google\.com/.*?/(?P<id1>[A-Za-z0-9_-]{33})/?)        # normal regex
        |
    (.*?drive\.google\.com/uc.+?id=(?P<id2>[A-Za-z0-9_-]{33})&?) # this was also there
";

#[derive(Template, Debug)]
#[parent("gdrive")]
pub enum Output {
    Video {
        title: String,
        created: String,
        size: String,
        width: i64,
        height: i64,
        duration: String,
    },
}

async fn hear_gdrive(context: Context, mut noye: Noye) -> anyhow::Result<()> {
    let query = Query {
        key: &get_api_key("gdrive")?,
    };

    let ids = context.matches().gather(&["id1", "id2"])?;
    let client = http::new_client();

    let mut stream = concurrent_map("gdrive", None, ids, |id| {
        let client = client.clone();
        let query = query;
        async move { lookup(&client, &id, query).await }
    })
    .await;

    while let Some(resp) = stream.next().await {
        noye.say_template(&context, resp)?;
    }

    noye.nothing()
}

async fn lookup(client: &reqwest::Client, id: &str, query: Query<'_>) -> anyhow::Result<Output> {
    let url = format!("https://www.googleapis.com/drive/v2/files/{}", id);
    let item: Item = crate::http::get_json(client, &url, &query, &[]).await?;

    let created = item
        .created_date
        .and_then(|then| {
            (chrono::Utc::now() - then)
                .to_std()
                .ok()?
                .as_readable_time()
                .into()
        })
        .unwrap_or_else(|| "unknown".to_string());

    let size = item
        .file_size
        .parse::<u64>()
        .unwrap_or_default()
        .as_file_size();

    let duration = std::time::Duration::from_millis(
        item.video_media_metadata
            .duration_millis
            .parse::<u64>()
            .unwrap_or_default(),
    )
    .as_timestamp();

    let output = Output::Video {
        title: item.title,
        created,
        size,
        width: item.video_media_metadata.width,
        height: item.video_media_metadata.height,
        duration,
    };
    Ok(output)
}

#[derive(serde::Serialize, Copy, Clone)]
struct Query<'a> {
    key: &'a str,
}

#[derive(serde::Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
struct Item {
    id: String,
    title: String,
    created_date: Option<chrono::DateTime<chrono::Utc>>,
    original_filename: String,
    file_extension: String,
    file_size: String,
    video_media_metadata: VideoMediaMetadata,
}

#[derive(serde::Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
struct VideoMediaMetadata {
    width: i64,
    height: i64,
    duration_millis: String,
}

// TODO figure out how to partialeq with the template before the string
// application in the check method
