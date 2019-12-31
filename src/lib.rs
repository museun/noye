#![cfg_attr(debug_assertions, allow(dead_code,))]
/*!
noye. an irc bot
# stuff goes here
*/

// TODO use time 0.2 instead of chrono

/// Configuration
#[macro_use]
pub mod config;

/// Bot types
pub mod bot;

/// IRC client and types
pub mod irc;

#[doc(hidden)]
pub mod command;

#[doc(hidden)]
pub mod matches;

mod de;
mod http;

/// Formatting utilities
pub mod format;

mod modules;

static DEFAULT_CONFIG: &str = include_str!("../default.toml");
static DEFAULT_TEMPLATES: &str = include_str!("../templates.toml");
