use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Irc {
    pub address: String,
    pub port: u16,
    pub nick: String,
    pub user: String,
    pub real: String,
    pub owners: Vec<String>,
    pub channels: Vec<String>,
    pub q_name: String,
    pub q_pass: String,
}
