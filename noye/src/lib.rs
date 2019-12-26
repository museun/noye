/*!
noye. an irc bot
# stuff goes here
*/

// TODO use time 0.2 instead of chrono

/// The configuration module
#[macro_use]
pub mod config;

/// This bot module
pub mod bot;

/// The IRC module
pub mod irc;

#[doc(hidden)]
pub mod command;

#[doc(hidden)]
pub mod matches;

mod de;
mod http;

pub mod format;
