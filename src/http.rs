use futures::prelude::*;
use std::collections::HashMap;

use std::sync::Arc;

/// Make a sharable client
pub fn new_client() -> Arc<reqwest::Client> {
    Default::default()
}

/// so no query-strings can be used with get_json
#[derive(serde::Serialize)]
pub struct NoQuery;

pub async fn get_body(
    client: &reqwest::Client,
    url: &str,
    headers: &[(&'static str, &str)],
) -> anyhow::Result<String> {
    let mut req = client.get(url);
    for &(k, v) in headers {
        req = req.header(k, v);
    }
    client
        .execute(req.build()?)
        .and_then(|body| async move { body.text().await })
        .map_err(|err| anyhow::anyhow!("cannot get url '{}': {}", url, err))
        .await
}

pub async fn get_json<'a, T, Q>(
    client: &reqwest::Client,
    url: &'a str,
    query: &'a Q,
    headers: &[(&'static str, &str)],
) -> anyhow::Result<T>
where
    for<'de> T: serde::Deserialize<'de>,
    Q: serde::Serialize + 'a,
{
    let mut req = client.get(url);
    if let Some(query) = query.into() {
        req = req.query(query);
    }
    for &(k, v) in headers {
        req = req.header(k, v);
    }
    client
        .execute(req.build()?)
        .and_then(|body| async move { body.json().await })
        .map_err(|err| anyhow::anyhow!("cannot get url '{}': {}", url, err))
        .await
}

pub async fn head(client: &reqwest::Client, url: &str) -> anyhow::Result<reqwest::Response> {
    let req = client.head(url).build()?;
    client
        .execute(req)
        .map_err(|err| anyhow::anyhow!("failed to do HEAD on {}: {}", url, err))
        .await
}

pub struct Url {
    domain: String,
    path: String,
    query: HashMap<String, String>,
}

impl std::convert::TryFrom<&str> for Url {
    type Error = ();
    fn try_from(data: &str) -> std::result::Result<Self, Self::Error> {
        let url = data.parse::<http::Uri>().map_err(|_| ())?;
        let query_map = url
            .query()
            .into_iter()
            .flat_map(|s| s.split('&'))
            .filter_map(|s| {
                let mut s = s.split('=').map(ToString::to_string);
                let (k, v) = (s.next()?, s.next()?);
                (k, v).into()
            });

        Ok(Self {
            domain: url.authority().ok_or_else(|| ())?.host().to_string(),
            path: url.path().to_string(),
            query: query_map.collect(),
        })
    }
}

impl Url {
    pub fn domain(&self) -> &str {
        &self.domain
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn query<Q: ?Sized>(&self, key: &Q) -> Option<&str>
    where
        Q: std::hash::Hash + Eq,
        String: std::borrow::Borrow<Q>,
    {
        self.query.get(key).map(|s| s.as_str())
    }
}
