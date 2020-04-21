use ::serde::de::*;
use std::str::FromStr;

pub fn rfc3339<'de, D>(deser: D) -> Result<time::OffsetDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    time::OffsetDateTime::parse(&String::deserialize(deser)?, time::Format::Rfc3339)
        .map_err(Error::custom)
}

pub fn rfc3339_opt<'de, D>(deser: D) -> Result<Option<time::OffsetDateTime>, D::Error>
where
    D: Deserializer<'de>,
{
    let d = match String::deserialize(deser).ok() {
        Some(d) => d,
        None => return Ok(None),
    };

    time::OffsetDateTime::parse(&d, time::Format::Rfc3339)
        .map(Some)
        .map_err(Error::custom)
}

pub fn prim_date_time<'de, D>(deser: D) -> Result<time::PrimitiveDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    time::parse(&String::deserialize(deser)?, "%FT%TZ").map_err(Error::custom)
}

pub fn from_str<'de, D, T>(deser: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Display,
{
    String::deserialize(deser)?.parse().map_err(Error::custom)
}
