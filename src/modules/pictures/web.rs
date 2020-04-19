use rand::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::sync::RwLock;
use warp::{
    http::{header, Response},
    Filter,
};

#[derive(Clone, Default)]
pub struct Db {
    pub(super) inner: Arc<RwLock<Mapping>>,
}

impl Db {
    pub fn new(mapping: Mapping) -> Self {
        Self {
            inner: Arc::new(RwLock::new(mapping)),
        }
    }
}

pub fn lookup(db: Db) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    fn with_mapping(
        db: Db,
    ) -> impl Filter<Extract = (Db,), Error = std::convert::Infallible> + Clone {
        warp::any().map(move || db.clone())
    }

    warp::path!("p" / String / usize)
        .and(warp::get())
        .and(with_mapping(db))
        .and_then(lookup_item)
}

async fn lookup_item(
    name: String,
    id: usize,
    db: Db,
) -> std::result::Result<impl warp::Reply, warp::Rejection> {
    use headers::{HeaderMapExt, *};
    use tokio::prelude::*;

    fn content_disposition(path: &Path) -> Option<String> {
        format!(
            "inline; filename*=UTF-8''{}",
            percent_encoding::utf8_percent_encode(
                path.file_name()?.to_str()?,
                percent_encoding::NON_ALPHANUMERIC
            )
        )
        .into()
    }

    let path = db
        .inner
        .read()
        .await
        .get(&name)
        .and_then(|entry| entry.lookup(id))
        .ok_or_else(warp::reject::not_found)?
        .clone();

    let mut file = tokio::fs::OpenOptions::new()
        .read(true)
        .open(&path)
        .await
        .map_err(|_| warp::reject::not_found())?;

    let modified = file
        .metadata()
        .await
        .map_err(|_| warp::reject::not_found())?
        .modified()
        .ok()
        .map(LastModified::from);

    let mut data = vec![];
    file.read_to_end(&mut data)
        .await
        .map_err(|_| warp::reject::not_found())?;

    let mut resp = Response::new(data);
    let headers = resp.headers_mut();

    headers.insert(
        header::CONTENT_DISPOSITION,
        content_disposition(&path)
            .and_then(|cd| header::HeaderValue::from_str(&cd).ok())
            .ok_or_else(warp::reject::not_found)?,
    );
    headers.typed_insert(ContentType::from(
        mime_guess::from_path(&path).first_or_octet_stream(),
    ));
    headers.typed_insert(AcceptRanges::bytes());
    headers.typed_insert(
        CacheControl::new()
            .with_public()
            .with_max_age(std::time::Duration::from_secs(15 * 60)),
    );
    if let Some(modified) = modified {
        headers.typed_insert(modified)
    }

    Ok(resp)
}

#[derive(Debug)]
pub struct Entry {
    name: String,
    start: usize,
    map: HashMap<usize, PathBuf>,
    banned: HashSet<String>,
}

impl Entry {
    pub fn new(name: impl ToString) -> Self {
        let (start, map, banned) = Default::default();
        Self {
            name: name.to_string(),
            start,
            map,
            banned,
        }
    }

    pub fn blacklist(&mut self, channel: impl ToString) {
        self.banned.insert(channel.to_string());
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn total(&self) -> usize {
        self.map.len()
    }

    pub fn lookup(&self, id: usize) -> Option<&PathBuf> {
        self.map.get(&id)
    }

    pub fn choose<R: ?Sized + Rng>(&self, hint: &str, rng: &mut R) -> Option<(usize, &PathBuf)> {
        if self.banned.contains(hint) {
            return None;
        }

        self.map.iter().choose(rng).map(|(i, e)| (*i, e))
    }

    pub fn index(&mut self, root: impl AsRef<Path>) -> usize {
        fn filter(entry: walkdir::DirEntry) -> Option<walkdir::DirEntry> {
            #[cfg(not(test))]
            const EXTS: &[&str] = &["jpg", "jpeg", "png", "gif"];
            #[cfg(test)]
            const EXTS: &[&str] = &["rs"];
            entry
                .path()
                .extension()
                .and_then(|s| s.to_str())
                .filter(|ext| EXTS.contains(ext))?;
            Some(entry)
        }

        let start = self.start;
        let old = self.map.len();

        let index = walkdir::WalkDir::new(root)
            .into_iter()
            .flatten()
            .filter_map(filter)
            .enumerate()
            .map(move |(i, e)| (i + start, e.into_path()));

        self.map.extend(index);

        let count = self.map.len() - old;
        self.start += count;
        count
    }
}

#[derive(Default)]
pub struct Mapping {
    mapping: HashMap<String, Entry>,
    seen: HashMap<String, HashSet<usize>>,
}

impl Mapping {
    pub fn clear(&mut self) {
        self.mapping.clear();
        self.seen.clear();
    }

    pub fn insert(&mut self, entry: Entry) {
        self.seen
            .insert(entry.name().to_string(), Default::default());
        self.mapping.insert(entry.name().to_string(), entry);
    }

    pub fn names(&self) -> impl Iterator<Item = &str> + '_ {
        self.mapping.keys().map(|s| s.as_str())
    }

    pub fn get(&self, name: &str) -> Option<&Entry> {
        self.mapping.get(name)
    }

    pub fn contains(&self, name: &str) -> bool {
        self.mapping.contains_key(name)
    }

    pub fn choose<'a, R: ?Sized + Rng>(
        &'a mut self,
        rng: &mut R,
        channel: &str,
        name: impl Into<Option<&'a str>>,
    ) -> Option<(&'a str, usize)> {
        let name = match name.into() {
            Some(name) => name,
            None => self.mapping.keys().choose(rng)?.as_str(),
        };

        let entry = self.mapping.get(name)?;
        loop {
            let (index, _) = entry.choose(channel, rng)?;
            let set = self
                .seen
                .get_mut(name)
                // TODO for dynamic re-indexing this is a huge problem
                .expect("seen must stay in sync with the mapping");
            // we've seen them all
            if set.len() == entry.total() {
                set.clear();
            }
            if set.insert(index) {
                break (name, index).into();
            }
        }
    }
}
