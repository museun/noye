use crate::de;

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: String,
    pub snippet: Snippet,
    pub content_details: Option<ContentDetails>,
    pub statistics: Statistics,
    pub live_streaming_details: Option<LiveStreamingDetails>,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    pub published_at: chrono::DateTime<chrono::Utc>,

    #[serde(default)]
    pub channel_id: String,

    pub title: String,

    #[serde(default)]
    pub channel_title: String,

    #[serde(default)]
    pub live_broadcast_content: LiveBroadcastContent,
}

#[derive(Debug, Copy, Clone, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LiveBroadcastContent {
    Live,
    Upcoming,
    None,
}
impl Default for LiveBroadcastContent {
    fn default() -> Self {
        LiveBroadcastContent::None
    }
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentDetails {
    pub duration: String, // isn't this chrono?
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Statistics {
    #[serde(deserialize_with = "de::fromstr")]
    #[serde(default)]
    pub view_count: i64,

    #[serde(deserialize_with = "de::fromstr")]
    #[serde(default)]
    pub like_count: i64,

    #[serde(deserialize_with = "de::fromstr")]
    #[serde(default)]
    pub dislike_count: i64,

    #[serde(deserialize_with = "de::fromstr")]
    #[serde(default)]
    pub favorite_count: i64,

    #[serde(deserialize_with = "de::fromstr")]
    #[serde(default)]
    pub subscriber_count: i64,

    #[serde(default)]
    pub hidden_subscriber_count: bool,

    #[serde(deserialize_with = "de::fromstr")]
    #[serde(default)]
    pub video_count: i64,

    #[serde(deserialize_with = "de::fromstr")]
    #[serde(default)]
    pub comment_count: i64,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveStreamingDetails {
    pub actual_start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub actual_end_time: Option<chrono::DateTime<chrono::Utc>>,

    pub scheduled_start_time: Option<chrono::DateTime<chrono::Utc>>,
    pub scheduled_end_time: Option<chrono::DateTime<chrono::Utc>>,

    #[serde(deserialize_with = "de::fromstr")]
    #[serde(default)]
    pub concurrent_viewers: i64,
}
