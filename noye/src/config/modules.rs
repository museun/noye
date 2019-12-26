use super::ApiKey;
use serde::{Deserialize, Serialize};

macro_rules! just_an_api_key {
    ($ident:ident => $key:expr) => {
        #[derive(Default, Debug, Clone, Serialize, Deserialize)]
        pub struct $ident {
            pub api_key: String,
        }

        impl ApiKey for $ident {
            fn get_api_key(&self) -> &str {
                &self.api_key
            }
            fn get_key() -> &'static str {
                $key
            }
        }
    };
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Modules {
    pub youtube: Youtube,
    pub gdrive: GDrive,
    pub link_size: LinkSize,
}

just_an_api_key!(Youtube => "NOYE_YOUTUBE_API_KEY");
just_an_api_key!(GDrive => "NOYE_GDRIVE_API_KEY");

#[derive(Default, Debug, Copy, Clone, Serialize, Deserialize)]
pub struct LinkSize {
    pub size_limit: u64,
}
