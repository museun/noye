/// Serde conversion using FromStr
pub fn fromstr<'de, T, D>(de: D) -> Result<T, D::Error>
where
    D: serde::de::Deserializer<'de>,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    use serde::Deserialize as _;
    String::deserialize(de)?
        .parse()
        .map_err(serde::de::Error::custom)
}
