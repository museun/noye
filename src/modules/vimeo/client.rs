#[derive(Clone, Default)]
pub struct VimeoClient {
    ep: Option<String>,
}

impl VimeoClient {
    #[cfg(test)]
    pub fn with_ep(ep: impl ToString) -> Self {
        Self {
            ep: Some(ep.to_string()),
        }
    }

    pub async fn lookup_video(&self, vid: &str, client: reqwest::Client) -> anyhow::Result<Video> {
        let url = format!(
            "{}video/{}/config",
            self.ep
                .as_deref()
                .unwrap_or_else(|| "https://player.vimeo.com/"),
            vid
        );

        crate::http::client::get_json(
            client,
            &url,
            &crate::http::client::NoQuery,
            &[("Accept-Encoding", "identity")],
        )
        .await
        .map(|resp: Response| resp.video)
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct Response {
    pub video: Video,
}

#[derive(Debug, serde::Deserialize)]
pub struct Video {
    pub id: i64,
    pub width: i64,
    pub height: i64,
    pub duration: i64,
    pub fps: f64,
    pub title: String,
    pub owner: Owner,
}

#[derive(Debug, serde::Deserialize)]
pub struct Owner {
    pub name: String,
}
