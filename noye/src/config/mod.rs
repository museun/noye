use futures::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

static DEFAULT_CONFIG: &str = include_str!("../../../default.toml");
static DEFAULT_TEMPLATES: &str = include_str!("../../../templates.toml");

mod log_level;
pub use log_level::LogLevel;

mod irc;
pub use irc::Irc;

mod disabled;
pub use disabled::{Channel, DisabledModules, Module};

mod modules;
pub use modules::{GDrive, LinkSize, Modules, Youtube};

pub(self) const CONFIG_FILE: &str = "noye.toml";
pub(self) const REPLACE_THIS: &str = "!! PLEASE REPLACE THIS !!";

/// Trait to get the api key + environment variable key for this module config
pub trait ApiKey {
    /// Get the api key from this type
    fn get_api_key(&self) -> &str;
    /// Get the ENV_VAR key for this type
    fn get_key() -> &'static str;
}

/// Generate an static, lazy api from the environment
///
/// Use `get_api_key() -> Option<Arc<String>>` to get it
///
/// This returns None if it was never set (e.g. not in the configuraiton)
#[macro_export]
macro_rules! load_api_key {
    ($key:expr) => {
        use once_cell::sync::Lazy;
        use std::sync::Arc;
        static API_KEY: Lazy<Option<Arc<String>>> =
            Lazy::new(|| std::env::var($key).ok().map(Arc::new));

        /// Get the api key, returning None if it was never set
        fn get_api_key(name: &str) -> anyhow::Result<Arc<String>> {
            API_KEY
                .as_ref()
                .map(Arc::clone)
                .ok_or_else(|| anyhow::anyhow!("no api key was set for: {}", name))
        }
    };
}

/// Configuration for restarting the bot
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Restart {
    pub address: String,
}

/// Configuration for the bot
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// The log level to use for the console output
    pub log_level: LogLevel,
    /// The irc configuration
    pub irc_config: Irc,
    /// The restart configuation
    pub restart_config: Restart,
    /// Configuration for motules
    pub modules_config: Modules,
    /// Disabled modules on a per channel basis
    pub disabled_modules: DisabledModules,
}

impl Config {
    /// Tries to load this config from the "noye.toml" in the current directory
    pub async fn load() -> anyhow::Result<Self> {
        enum LoadError {
            NotFound,
            FixQConfig,
            ApiKey(&'static str),
            InvalidSyntax(toml::de::Error),
        }

        async fn load() -> Result<Config, LoadError> {
            if cfg!(test) {
                log::warn!("test environment gets a default configuration");
                return Ok(Config::default());
            }

            let data = tokio::fs::read_to_string(CONFIG_FILE)
                .map_err(|_| LoadError::NotFound)
                .await?;

            let this: Config = toml::from_str(&data).map_err(LoadError::InvalidSyntax)?;
            if this.irc_config.q_name == REPLACE_THIS || this.irc_config.q_pass == REPLACE_THIS {
                return Err(LoadError::FixQConfig);
            }
            // TODO verify macro
            if this.modules_config.youtube.api_key == REPLACE_THIS {
                return Err(LoadError::ApiKey("youtube"));
            }
            Ok(this)
        }

        match load().await {
            Ok(ok) => {
                log::trace!("loading keys from the configuration");
                ok.load_keys_from_config();
                log::trace!("loaded keys from the configuration");
                Ok(ok)
            }
            Err(LoadError::NotFound) => {
                log::warn!(
                    "{} wasn't found. creating a new one. edit it and re-run",
                    CONFIG_FILE
                );
                tokio::fs::write(CONFIG_FILE, DEFAULT_CONFIG).await?;
                std::process::exit(1);
            }
            Err(LoadError::InvalidSyntax(inner)) => {
                log::error!("invalid configuration file, please check it: {}", inner);
                std::process::exit(1);
            }
            Err(LoadError::ApiKey(module)) => {
                log::error!(
                    "the api_key for 'modules_config.{}' should be replaced (empty if you don't want to use it)",
                    module
                );
                std::process::exit(1);
            }
            Err(LoadError::FixQConfig) => {
                log::error!(
                    "q_name and q_pass should be replaced (empty if you don't want to auth)"
                );
                std::process::exit(1);
            }
        }
    }

    fn load_keys_from_config(&self) {
        fn set_api_key<K: ApiKey>(apikey: &K) {
            let k = K::get_key();
            let v = apikey.get_api_key();
            std::env::set_var(k, v);
        }

        let Modules { youtube, .. } = &self.modules_config;
        set_api_key(youtube);
    }

    /// Print out the default configuration
    pub fn print_default() {
        println!("{}", DEFAULT_CONFIG)
    }

    // Print out the defaulte templates
    pub fn print_templates() {
        println!("{}", DEFAULT_TEMPLATES)
    }
}

/// Load environment variables from `.env` (or ../.env)
pub fn load_env() -> anyhow::Result<()> {
    use anyhow::Context as _;
    match load_env_from_file(".env") {
        Ok(ok) => ok,
        // try the parent
        Err(err) => load_env_from_file("../.env").context(err)?,
    }
    .into_iter()
    .inspect(|(k, v)| log::trace!("setting: {} to _ (len: {})", k, v.len()))
    .for_each(|(k, v)| std::env::set_var(k, v));
    Ok(())
}

/// Load environment variables from a file, into a HashMap
pub fn load_env_from_file(
    file: impl AsRef<std::path::Path>,
) -> anyhow::Result<HashMap<String, String>> {
    let map = std::fs::read_to_string(file)?
        .lines()
        .filter(|s| !s.starts_with('#'))
        .filter_map(|line| {
            let mut line = line.splitn(2, '=').map(str::trim);
            (line.next()?.into(), line.next()?.into()).into()
        })
        .collect();
    Ok(map)
}
