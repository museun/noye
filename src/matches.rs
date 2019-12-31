use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;

/// A mapping of regular expression group names to their matches
#[derive(Default, Debug, Clone)]
pub struct Matches {
    map: HashMap<String, Vec<String>>,
}

impl Matches {
    pub(crate) fn from_regex(re: &regex::Regex, input: &str) -> Self {
        let mut map = HashMap::<_, Vec<_>>::new();
        for captures in re.captures_iter(&input) {
            for (cap, name) in re.capture_names().flatten().filter_map(|name| {
                let cap = captures.name(name)?;
                (cap.as_str().trim(), name.to_string()).into()
            }) {
                let entry = map.entry(name).or_default();
                if !cap.is_empty() {
                    entry.push(cap.into())
                }
            }
        }
        Self { map }
    }

    /// Get the key (capture name) and returns the first match
    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&String>
    where
        Q: Hash + Eq,
        String: Borrow<Q>,
    {
        self.map
            .get(&key)
            .and_then(|k| k.first())
            .filter(|s| !s.is_empty())
    }

    /// Get the key (capture name) and returns all matches
    pub fn get_many<Q: ?Sized>(&self, key: &Q) -> anyhow::Result<&[String]>
    where
        Q: Hash + Eq + std::fmt::Display,
        String: Borrow<Q>,
    {
        self.map
            .get(&key)
            .map(|k| k.as_slice())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow::anyhow!("'{}' not found", key))
    }

    pub fn gather<Q: ?Sized>(&self, keys: &[&Q]) -> anyhow::Result<Vec<&'_ String>>
    where
        Q: Hash + Eq + std::fmt::Display,
        String: Borrow<Q>,
    {
        let mut result = vec![];
        for key in keys {
            if let Ok(many) = self.get_many(key) {
                result.extend(many)
            }
        }

        if result.is_empty() {
            anyhow::bail!(
                "nothing found for: [{}]",
                keys.iter().fold(String::new(), |mut a, c| {
                    if a.is_empty() {
                        a.push_str(", ");
                    }
                    a.push_str(&c.to_string());
                    a
                })
            )
        }

        Ok(result)
    }

    /// Determine if this key (capture name) is in the mapping
    pub fn has<Q: ?Sized>(&self, key: &Q) -> bool
    where
        Q: Hash + Eq,
        String: Borrow<Q>,
    {
        self.get(key).is_some()
    }

    /// Get an iterator over all of the keys (capture names)
    pub fn names(&self) -> impl Iterator<Item = &'_ String> + '_ {
        self.map.keys()
    }
}
