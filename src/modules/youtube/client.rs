use super::*;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct YoutubeClient {
    client: reqwest::Client,
    api_key: Arc<String>,
    ep: Option<String>,
}

impl YoutubeClient {
    pub fn new(api_key: impl ToString) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: Arc::new(api_key.to_string()),
            ep: None,
        }
    }

    #[cfg(test)]
    pub fn new_with_ep(end_point: impl ToString) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: Default::default(),
            ep: Some(end_point.to_string()),
        }
    }

    pub async fn lookup_video(&self, vid: &str) -> anyhow::Result<data::Item> {
        static BASE: &str = "https://www.googleapis.com/youtube/v3/videos";
        static PARTS: &str = "statistics,snippet,liveStreamingDetails,contentDetails";
        static FIELDS: &str = "items(id,statistics,liveStreamingDetails,\
            snippet(title,channelTitle,channelId,liveBroadcastContent,publishedAt),\
            contentDetails(duration,regionRestriction))";

        #[derive(Serialize)]
        struct Query<'a> {
            id: &'a str,
            part: &'a str,
            fields: &'a str,
            key: &'a str,
        }

        self.get_item(
            self.ep.as_deref().unwrap_or(BASE),
            Query {
                id: vid,
                part: PARTS,
                fields: FIELDS,
                key: &self.api_key,
            },
            || format!("video for {}", vid),
        )
        .await
    }

    pub async fn lookup_channel(&self, channel: Channel) -> anyhow::Result<data::Item> {
        static BASE: &str = "https://www.googleapis.com/youtube/v3/channels";
        static PART: &str = "snippet,statistics";
        static FIELDS: &str = "items(id,snippet(title,description,publishedAt),statistics,status)";

        let chan = channel.clone();
        let (id, username) = match channel {
            Channel::User(id) => (None, Some(id)),
            Channel::Channel(id) => (Some(id), None),
        };

        #[derive(Serialize)]
        struct Query<'a> {
            id: Option<&'a str>,
            #[serde(rename = "forUsername")]
            username: Option<&'a str>,
            part: &'a str,
            fields: &'a str,
            key: &'a str,
        }

        self.get_item(
            self.ep.as_deref().unwrap_or(BASE),
            Query {
                id: id.as_deref(),
                username: username.as_deref(),
                part: PART,
                fields: FIELDS,
                key: &self.api_key,
            },
            move || match &chan {
                Channel::User(id) => format!("user for {}", id),
                Channel::Channel(id) => format!("channel for {}", id),
            },
        )
        .await
    }

    async fn get_item(
        &self,
        base: &str,
        query: impl serde::Serialize,
        kind: impl Fn() -> String,
    ) -> anyhow::Result<data::Item> {
        #[derive(Deserialize)]
        struct Items {
            items: Vec<data::Item>,
        }

        self.client
            .get(base)
            .query(&query)
            .send()
            .await?
            .json()
            .await
            .map_err(Into::into)
            .and_then(|mut d: Items| {
                d.items
                    .pop()
                    .ok_or_else(|| anyhow::anyhow!("cannot get {}", kind()))
            })
    }
}

#[derive(Clone)]
pub enum Channel {
    User(String),
    Channel(String),
}
