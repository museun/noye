#![cfg_attr(debug_assertions, allow(dead_code))]

use rand::prelude::*;
use std::{collections::HashMap, path::Path, sync::Arc};
use tokio::sync::RwLock;
use warp::{http::Response, Filter};

#[derive(Clone)]
pub struct TempStore {
    pub inner: Arc<RwLock<Temporary>>,
}

impl Default for TempStore {
    fn default() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Temporary::new(std::time::Duration::from_secs(
                3 * 60,
            )))),
        }
    }
}

impl TempStore {
    pub fn start_culling(&self) {
        let inner = self.inner.clone();
        tokio::spawn(async move {
            use futures::prelude::*;
            let mut tick = tokio::time::interval(tokio::time::Duration::from_secs(60));
            while let Some(..) = tick.next().await {
                let n = inner.write().await.cull();
                if n > 0 {
                    log::debug!("culled {} items", n);
                }
            }
        });
    }
}

pub fn temporary(
    temp: TempStore,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    fn with_temp(
        temp: TempStore,
    ) -> impl Filter<Extract = (TempStore,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || temp.clone())
    }

    warp::path!("t" / String)
        .and(warp::get())
        .and(with_temp(temp))
        .and_then(lookup_item)
}

async fn lookup_item(
    name: String,
    temp: TempStore,
) -> std::result::Result<impl warp::Reply, warp::Rejection> {
    use headers::HeaderMapExt as _;
    let item = temp
        .inner
        .read()
        .await
        .get(&name)
        .ok_or_else(|| warp::reject::not_found())?;

    let mut resp = Response::new(item.body);
    let headers = resp.headers_mut();
    headers.typed_insert(headers::ContentType::from(item.content_type));

    Ok(resp)
}

pub type Id = String;

// TODO allow just 'pathbufs' so it can read from disk, instead of storing it in memory
#[derive(Clone)]
pub struct Body {
    pub name: String,
    pub created: std::time::Instant,
    pub content_type: headers::ContentType,
    pub body: Vec<u8>,
}

pub struct Temporary {
    max_age: std::time::Duration,
    map: HashMap<Id, Body>,
}

impl Temporary {
    pub fn new(max_age: std::time::Duration) -> Self {
        Self {
            max_age,
            map: HashMap::new(),
        }
    }

    pub fn cull(&mut self) -> usize {
        let old = self.map.len();
        let now = std::time::Instant::now();

        let mut bad = Vec::new();
        for (k, v) in &self.map {
            if now - v.created > self.max_age {
                bad.push(k.clone());
            }
        }
        for k in bad {
            self.map.remove(&k);
        }
        self.map.len() - old
    }

    pub async fn insert_text_file<R: ?Sized + Rng>(
        &mut self,
        rng: &mut R,
        path: impl AsRef<Path>,
    ) -> anyhow::Result<Id> {
        let body = tokio::fs::read(&path).await?;
        let name = path
            .as_ref()
            .file_name()
            .map(|s| s.to_string_lossy())
            .unwrap();
        Ok(self.insert(rng, Some(name), body, headers::ContentType::text_utf8()))
    }

    pub async fn insert_file<R: ?Sized + Rng>(
        &mut self,
        rng: &mut R,
        path: impl AsRef<Path>,
    ) -> anyhow::Result<Id> {
        let body = tokio::fs::read(&path).await?;
        let name = path
            .as_ref()
            .file_name()
            .map(|s| s.to_string_lossy())
            .unwrap();
        Ok(self.insert(
            rng,
            Some(name),
            body,
            mime_guess::from_path(&path).first_or_octet_stream(),
        ))
    }

    pub fn insert_text<R: ?Sized + Rng>(
        &mut self,
        rng: &mut R,
        body: impl Into<Vec<u8>>,
        name: impl ToString,
    ) -> Id {
        self.insert(rng, Some(name), body, headers::ContentType::text_utf8())
    }

    pub fn insert<R: ?Sized + Rng>(
        &mut self,
        rng: &mut R,
        name: Option<impl ToString>,
        body: impl Into<Vec<u8>>,
        content_type: impl Into<headers::ContentType>,
    ) -> Id {
        loop {
            let id: Id = rng
                .sample_iter(&rand::distributions::Alphanumeric)
                .take(4)
                .collect();

            if self.map.contains_key(&id) {
                continue;
            }

            let body = Body {
                name: name.map(|s| s.to_string()).unwrap_or_else(|| id.clone()),
                created: std::time::Instant::now(),
                content_type: content_type.into(),
                body: body.into(),
            };

            self.map.insert(id.clone(), body);
            break id;
        }
    }

    pub fn get<I: ?Sized>(&self, id: &I) -> Option<Body>
    where
        I: std::hash::Hash + Eq,
        String: std::borrow::Borrow<I>,
    {
        self.map.get(id).cloned()
    }

    pub fn remove<I: ?Sized>(&mut self, id: &I) -> Option<Body>
    where
        I: std::hash::Hash + Eq,
        String: std::borrow::Borrow<I>,
    {
        self.map.remove(id)
    }
}
