use super::{IntoResponse, Response};
use crate::command::Context;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::SystemTime;

pub trait Template {
    fn parent() -> &'static str;
    fn name() -> &'static str;
    fn variant(&self) -> &'static str;
    fn apply(&self, input: &str) -> Option<String>;
}

impl<T> IntoResponse for T
where
    T: Template,
{
    fn into_response(self, context: Context) -> Response {
        let (parent, name) = (T::parent(), self.variant());
        TemplateResolver::load(parent, name)
            .and_then(|data| self.apply(&data))
            .map(|data| Response::say(context, data))
            .or_else(|| {
                log::warn!("template resolver failed to find {}::{}", parent, name);
                None
            })
            .unwrap_or_default()
    }
}

pub struct TemplateResolver;
impl TemplateResolver {
    fn load(parent: &str, name: &str) -> Option<String> {
        STORE
            .lock()
            .unwrap()
            .refresh_and_get(parent)?
            .get(name)
            .cloned()
    }
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct Pair<T, V = T>(HashMap<T, V>)
where
    T: std::hash::Hash + Eq + Sized;

impl<T> Pair<T>
where
    T: std::hash::Hash + Eq,
{
    fn get<K: ?Sized>(&self, k: &K) -> Option<&T>
    where
        K: std::hash::Hash + Eq + std::fmt::Display,
        T: std::borrow::Borrow<K>,
    {
        self.0.get(k)
    }
}

pub type TemplateMap<T> = HashMap<T, Pair<T>>;

#[derive(Debug, serde::Deserialize)]
pub struct Templates {
    #[serde(skip)]
    start: Option<SystemTime>,
    templates: TemplateMap<String>,
}

impl Default for Templates {
    fn default() -> Self {
        Self::new()
    }
}

impl Templates {
    #[cfg(not(test))]
    const TEMPLATE_FILE: &'static str = "./templates.toml";
    #[cfg(test)]
    const TEMPLATE_FILE: &'static str = "../templates.toml";

    fn new() -> Self {
        let mut this = Self {
            start: SystemTime::now().into(),
            templates: Default::default(),
        };
        this.refresh();
        this
    }

    pub fn refresh_and_get<K: ?Sized>(&mut self, parent: &K) -> Option<&Pair<String>>
    where
        K: std::hash::Hash + Eq + std::fmt::Display,
        String: std::borrow::Borrow<K>,
    {
        self.refresh();
        self.templates.get(parent)
    }

    fn refresh(&mut self) {
        let file = Self::TEMPLATE_FILE;
        let mtime = match std::fs::metadata(&file).and_then(|md| md.modified()) {
            Ok(mtime) => mtime,
            Err(err) => {
                log::error!("cannot read template ({}) file: {}", file, err);
                return;
            }
        };

        let start = match self.start {
            Some(start) => start,
            None => {
                log::error!("template state is fatally invalid. please restart the bot");
                return;
            }
        };

        if start < mtime || self.templates.is_empty() {
            match Self::read_data() {
                Some(map) => {
                    log::info!("reloaded templates");
                    self.start.replace(mtime);
                    std::mem::replace(&mut self.templates, map);
                }
                None => log::info!("cannot read templates from '{}'. not updating them", file),
            }
        }
    }

    fn read_data() -> Option<TemplateMap<String>> {
        let file = Self::TEMPLATE_FILE;
        std::fs::read_to_string(file)
            .ok()
            .and_then(|data| toml::from_str::<TemplateMap<String>>(&data).ok())
    }
}

static STORE: Lazy<Mutex<Templates>> = Lazy::new(|| Mutex::new(Templates::new()));
