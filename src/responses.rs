use serde::{Deserialize, Serialize};
use template::*;

#[derive(Template, Debug, Clone, Serialize, Deserialize)]
#[namespace("builtin")]
pub enum Builtin {
    NotOwner,
    UnknownSubcommand { command: String },
}

#[derive(Template, Debug, Clone, Serialize, Deserialize)]
#[namespace("uptime")]
pub enum Uptime {
    Uptime { uptime: String },
}

#[derive(Template, Debug, Clone, Serialize, Deserialize)]
#[namespace("join")]
pub enum Join {
    ExpectedChannel,
}

#[derive(Template, Debug, Clone, Serialize, Deserialize)]
#[namespace("link_size")]
pub enum LinkSize {
    Single { size: String },
    Many { files: String },
}

#[derive(Template, Debug, Clone, Serialize, Deserialize)]
#[namespace("repost")]
pub enum Repost {
    AlreadyPosted {
        nick: String,
        count: String,
        ago: String,
    },
    SelfPosted {
        count: String,
        ago: String,
    },
    NoLinkProvided,
    AlreadyIgnored,
    Ignored,
}

#[derive(Template, Debug, Clone, Serialize, Deserialize)]
#[namespace("youtube")]
pub enum Youtube {
    Video {
        channel: String,
        duration: String,
        id: String,
        title: String,
        ts: String,
        views: String,
    },
    Live {
        channel: String,
        id: String,
        title: String,
        ts: String,
        viewers: String,
    },
    Upcoming {
        channel: String,
        id: String,
        start: String,
        title: String,
    },
    Channel {
        id: String,
        title: String,
        videos: String,
        views: String,
    },
}

#[derive(Template, Debug, Clone, Serialize, Deserialize)]
#[namespace("vimeo")]
pub enum Vimeo {
    Video {
        id: i64,
        width: i64,
        height: i64,
        duration: String,
        fps: String,
        title: String,
        owner: String,
    },
}

#[derive(Template, Debug, Clone, Serialize, Deserialize)]
#[namespace("pictures")]
pub enum Pictures {
    Listing { commands: String },
    Say { link: String },
    Reply { link: String },
    Refreshed { new: String }, // TODO this should accept Vecs
}

#[derive(Template, Debug, Clone, Serialize, Deserialize)]
#[namespace("gdrive")]
pub enum GDrive {
    Video {
        title: String,
        created: String,
        size: String,
        width: i64,
        height: i64,
        duration: String,
    },
}

#[derive(Template, Debug, Clone, Serialize, Deserialize)]
#[namespace("instagram")]
pub enum Instagram {
    Title { title: String },
}

#[derive(Template, Debug, Clone, Serialize, Deserialize)]
#[namespace("tempstore")]
pub enum TempStore {
    Link { link: String },
}

#[derive(Template, Debug, Clone, Serialize, Deserialize)]
#[namespace("concert")]
pub enum Concert {
    NoIdProvided,
    Title {
        id: String,
        title: String,
    },
    Total {
        num: String,
        chapter_count: String,
        length: String,
    },
    Talks {
        num: String,
        chapter_count: String,
        length: String,
        chapters: String,
    },
}
