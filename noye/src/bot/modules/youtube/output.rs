use super::data::*;
use crate::bot::prelude::*;

#[derive(Template, Debug)]
#[parent("youtube")]
pub enum Output {
    Video {
        channel: String,
        duration: String,
        id: String,
        title: String,
        ts: String,
        views: String,
    },
    Live {
        channel: String,
        id: String,
        title: String,
        ts: String,
        viewers: String,
    },
    Upcoming {
        channel: String,
        id: String,
        start: String,
        title: String,
    },
    Channel {
        id: String,
        title: String,
        videos: String,
        views: String,
    },
}

impl Output {
    pub fn from_channel(video: Item) -> Self {
        Output::Channel {
            id: video.id,
            title: video.snippet.title,
            videos: video.statistics.video_count.with_commas(),
            views: video.statistics.view_count.with_commas(),
        }
    }

    pub fn from_video(ts: Option<&str>, video: Item) -> Self {
        let ts = ts.map(|s| format!("?t={}", s)).unwrap_or_default();
        return match video.snippet.live_broadcast_content {
            LiveBroadcastContent::Live => for_live(ts, video),
            LiveBroadcastContent::Upcoming => for_upcoming(video),
            LiveBroadcastContent::None => for_none(ts, video),
        };

        fn for_live(ts: String, video: Item) -> Output {
            Output::Live {
                viewers: video
                    .live_streaming_details
                    .unwrap()
                    .concurrent_viewers
                    .with_commas(),
                channel: video.snippet.channel_title,
                id: video.id,
                title: video.snippet.title,
                ts,
            }
        }

        fn for_upcoming(video: Item) -> Output {
            Output::Upcoming {
                start: video
                    .live_streaming_details
                    .unwrap()
                    .scheduled_start_time
                    .and_then(|time| {
                        (time - chrono::Utc::now())
                            .to_std()
                            .ok()?
                            .as_readable_time()
                            .into()
                    })
                    .unwrap_or_default(),
                channel: video.snippet.channel_title,
                id: video.id,
                title: video.snippet.title,
            }
        }

        fn for_none(ts: String, video: Item) -> Output {
            Output::Video {
                duration: video
                    .content_details
                    .unwrap()
                    .duration
                    .from_iso8601()
                    .as_timestamp(),
                id: video.id,
                channel: video.snippet.channel_title,
                title: video.snippet.title,
                views: video.statistics.view_count.with_commas(),
                ts,
            }
        }
    }
}
