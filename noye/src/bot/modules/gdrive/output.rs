use crate::bot::prelude::*;

#[derive(Template, Debug)]
#[parent("gdrive")]
pub enum Output {
    Video {
        title: String,
        created: String,
        size: String,
        width: i64,
        height: i64,
        duration: String,
    },
}
