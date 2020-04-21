#![allow(clippy::future_not_send)]
pub static DEFAULT_TEMPLATES: &str = include_str!("../default_templates.toml");

#[derive(Debug, Clone)]
pub struct LogFile(pub std::path::PathBuf);

#[macro_use]
pub mod db;

mod bot;
pub use bot::{resolver, Context, Handler, Message, Responder, Runner, Writer, WriterResponder};

pub(crate) mod responses;

pub mod config;
pub use config::{CachedConfig, Config};

pub mod http;
pub mod modules;

mod irc;
mod util;

#[cfg(test)]
mod test;
