#![cfg_attr(debug_assertions, allow(dead_code))]

pub static DEFAULT_TEMPLATES: &str = include_str!("../default_templates.toml");

#[macro_use]
pub mod db;

pub mod bot;
pub use bot::{Context, Handler, Message, Responder, Runner, Writer};

pub(crate) mod responses;

mod config;
pub use config::{CachedConfig, Config};

pub mod modules;

#[derive(Debug, Clone)]
pub struct LogFile(pub std::path::PathBuf);

mod http;
mod irc;
mod util;

#[cfg(test)]
mod test;
