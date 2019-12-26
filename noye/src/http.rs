#![cfg_attr(debug_assertions, allow(dead_code,))]
use std::collections::HashMap;

/// so no query-strings can be used with get_json
#[derive(serde::Serialize)]
pub struct NoQuery;

pub async fn get_body<C>(
    client: &surf::Client<C>,
    url: &str,
    headers: &[(&'static str, &str)],
) -> anyhow::Result<Vec<u8>>
where
    C: surf::middleware::HttpClient,
{
    let mut req = client.get(url);
    for (k, v) in headers {
        req = req.set_header(k, v);
    }
    req.recv_bytes()
        .await
        .map_err(|err| anyhow::anyhow!("cannot get url '{}': {}", url, err))
}

pub async fn get_json<'a, T, C, Q>(
    client: &surf::Client<C>,
    url: &'a str,
    query: &'a Q,
    headers: &[(&'static str, &str)],
) -> anyhow::Result<T>
where
    for<'de> T: serde::Deserialize<'de>,
    C: surf::middleware::HttpClient,
    Q: serde::Serialize + 'a,
{
    let mut req = client.get(url);
    if let Some(query) = query.into() {
        req = req.set_query(query).unwrap();
    }
    for (k, v) in headers {
        req = req.set_header(k, v);
    }
    req.recv_json()
        .await
        .map_err(|err| anyhow::anyhow!("cannot get url '{}': {}", url, err))
}

pub async fn head<C>(client: &surf::Client<C>, url: &str) -> anyhow::Result<surf::Response>
where
    C: surf::middleware::HttpClient,
{
    client
        .head(url)
        .await
        .map_err(|err| anyhow::anyhow!("failed to do HEAD on {}: {}", url, err))
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
