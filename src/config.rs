use anyhow::Context as _;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

pub struct CachedConfig {
    config: Config,
    last: std::time::SystemTime,
    path: std::path::PathBuf,
}

impl CachedConfig {
    pub fn new(config: Config, path: impl Into<PathBuf>) -> Self {
        Self {
            config,
            path: path.into(),
            last: std::time::SystemTime::now(),
        }
    }

    pub async fn get(&mut self) -> anyhow::Result<&Config> {
        if cfg!(not(test)) {
            let md = std::fs::metadata(&self.path)?.modified()?;

            if self.last < md {
                self.config = Config::load(&self.path).await?;
                self.last = md;
            }
        }

        Ok(&self.config)
    }

    pub async fn get_mut(&mut self) -> anyhow::Result<&mut Config> {
        if cfg!(not(test)) {
            let md = std::fs::metadata(&self.path)?.modified()?;

            if self.last < md {
                self.config = Config::load(&self.path).await?;
                self.last = md;
            }
        }

        Ok(&mut self.config)
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub irc_config: Irc,
    pub modules: Modules,
    pub web: Web,
}

impl Config {
    pub async fn load(path: impl AsRef<std::path::Path>) -> anyhow::Result<Self> {
        let data = tokio::fs::read_to_string(&path)
            .await
            .with_context(|| "cannot read config file")?;
        toml::from_str(&data).with_context(|| "invalid config toml")
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Web {
    pub listen_port: u16,
    pub lookup_ip: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Irc {
    pub address: String,

    pub name: String,
    pub user: String,
    pub real: String,

    pub channels: Vec<String>,
    pub owners: Vec<String>,

    pub q_pass: Option<String>,
    pub q_name: Option<String>,
}

// TODO split this up into sub-configuration
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Modules {
    pub link_size: LinkSize,
    pub repost: Repost,
    pub restart: Restart,
    pub youtube: Youtube,
    pub gdrive: GDrive,
    pub pictures: Pictures,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct LinkSize {
    pub size_limit: u64,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Repost {
    pub staleness: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Restart {
    pub address: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Youtube {
    pub api_key: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct GDrive {
    pub api_key: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Pictures {
    pub cooldown: String,
    pub mention_chance: f64,
    pub passive_chance: f64,
    pub quiet_time: String,
    pub directories: HashMap<String, PicturesItem>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct PicturesItem {
    pub directory: String,
    pub command: String,
    pub banned_channels: Vec<String>,
}
