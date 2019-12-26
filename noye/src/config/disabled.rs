use super::*;
use std::collections::HashMap;

/// Modules that are disabled per channel
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct DisabledModules {
    disabled: HashMap<String, Vec<String>>,
}

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub struct Module<'a>(pub &'a str);

#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub struct Channel<'a>(pub &'a str);

impl DisabledModules {
    pub fn check(&self, channel: Channel<'_>, name: Module<'_>) -> bool {
        for (k, v) in &self.disabled {
            if k == channel.0 && v.iter().any(|d| d == name.0) {
                return true;
            }
        }
        false
    }
}
