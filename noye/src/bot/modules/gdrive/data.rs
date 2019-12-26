#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    #[serde(default)]
    pub id: String,

    #[serde(default)]
    pub title: String,

    #[serde(default)]
    pub created_date: Option<chrono::DateTime<chrono::Utc>>,

    #[serde(default)]
    pub original_filename: String,

    #[serde(default)]
    pub file_extension: String,

    #[serde(default)]
    pub file_size: String,

    pub video_media_metadata: VideoMediaMetadata,
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VideoMediaMetadata {
    pub width: i64,
    pub height: i64,

    #[serde(default)]
    pub duration_millis: String,
}
