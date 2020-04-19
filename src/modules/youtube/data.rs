use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: String,
    pub snippet: Snippet,
    pub content_details: Option<ContentDetails>,
    pub statistics: Statistics,
    pub live_streaming_details: Option<LiveStreamingDetails>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    #[serde(deserialize_with = "crate::util::rfc3339")]
    pub published_at: time::OffsetDateTime,

    #[serde(default)]
    pub channel_id: String,

    pub title: String,

    #[serde(default)]
    pub channel_title: String,

    #[serde(default)]
    pub live_broadcast_content: LiveBroadcastContent,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LiveBroadcastContent {
    Live,
    Upcoming,
    None,
}

impl Default for LiveBroadcastContent {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentDetails {
    pub duration: String, // time?
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Statistics {
    #[serde(deserialize_with = "crate::util::from_str", default)]
    pub view_count: i64,

    #[serde(deserialize_with = "crate::util::from_str", default)]
    pub like_count: i64,

    #[serde(deserialize_with = "crate::util::from_str", default)]
    pub dislike_count: i64,

    #[serde(deserialize_with = "crate::util::from_str", default)]
    pub favorite_count: i64,

    #[serde(deserialize_with = "crate::util::from_str", default)]
    pub subscriber_count: i64,

    #[serde(default)]
    pub hidden_subscriber_count: bool,

    #[serde(deserialize_with = "crate::util::from_str", default)]
    pub video_count: i64,

    #[serde(deserialize_with = "crate::util::from_str", default)]
    pub comment_count: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveStreamingDetails {
    #[serde(deserialize_with = "crate::util::rfc3339_opt", default)]
    pub actual_start_time: Option<time::OffsetDateTime>,

    #[serde(deserialize_with = "crate::util::rfc3339_opt", default)]
    pub actual_end_time: Option<time::OffsetDateTime>,

    #[serde(deserialize_with = "crate::util::rfc3339_opt", default)]
    pub scheduled_start_time: Option<time::OffsetDateTime>,

    #[serde(deserialize_with = "crate::util::rfc3339_opt", default)]
    pub scheduled_end_time: Option<time::OffsetDateTime>,

    #[serde(deserialize_with = "crate::util::from_str", default)]
    pub concurrent_viewers: i64,
}
