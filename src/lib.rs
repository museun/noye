pub static DEFAULT_TEMPLATES: &str = include_str!("../default_templates.toml");

#[macro_use]
pub mod db;

mod bot;
pub use bot::{Bot, CommandsMap, PassivesList};

mod responder;
pub use responder::{Responder, WriterResponder};

mod context;
pub use context::Context;

mod message;
pub use message::{Command, Irc, Message, Prefix};

mod state;
pub use state::State;

pub mod util;

mod handler;
pub use handler::{AnyhowFut, Handler};

mod format;
pub use format::{CommaSeparated, FileSize, Timestamp};

pub mod config;
pub use config::{CachedConfig, Config};

pub mod resolver;
pub use resolver::Resolver;

mod writer;
pub use writer::Writer;

mod responses;

pub mod modules;

mod http;

#[cfg(test)]
mod test;

#[derive(Debug, Clone)]
pub struct LogFile(pub std::path::PathBuf);

pub mod web;
