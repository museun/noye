use serde::de::*;
use std::str::FromStr;

#[derive(Debug)]
pub struct DontCareSigil {}

impl std::fmt::Display for DontCareSigil {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl std::error::Error for DontCareSigil {}

pub trait DontCare<T> {
    fn dont_care(self) -> anyhow::Result<T>;
}

impl<T> DontCare<T> for Option<T> {
    fn dont_care(self) -> anyhow::Result<T> {
        self.ok_or_else(|| DontCareSigil {}.into())
    }
}

impl<T, E> DontCare<T> for Result<T, E> {
    fn dont_care(self) -> anyhow::Result<T> {
        self.map_err(|_err| DontCareSigil {}.into())
    }
}

pub fn dont_care<T>() -> anyhow::Result<T> {
    Err(DontCareSigil {}.into())
}

pub fn inspect_err<F, D>(err: &anyhow::Error, kind: F)
where
    F: Fn() -> D,
    D: std::fmt::Display,
{
    if err.is::<DontCareSigil>() {
        return;
    }

    let err = err
        .chain()
        .enumerate()
        .fold(String::new(), |mut a, (i, err)| {
            a.extend(format!("\n[{}] --> ", i).drain(..));
            a.extend(err.to_string().drain(..));
            a
        });

    log::error!("got an error: {} because: {}", kind(), err);
}

pub fn type_name_of_val<T>(_: &T) -> &'static str {
    std::any::type_name::<T>()
}

pub fn type_name<T>() -> &'static str {
    fn reduce_type_name(mut input: &str) -> &str {
        // this is .. totally not something you should do
        fn trim_type(input: &str) -> &str {
            let mut n = input.len();
            let left = input
                .chars()
                .take_while(|&c| {
                    if c == '<' {
                        n -= 1;
                    }
                    !c.is_ascii_uppercase()
                })
                .count();
            &input[left..n]
        }

        let original = input;
        loop {
            let start = input.len();
            input = trim_type(input);
            if input.contains('<') {
                input = trim_type(&input[1..]);
            }
            match input.len() {
                0 => break original,
                d if d == start => break input,
                _ => {}
            }
        }
    }

    reduce_type_name(std::any::type_name::<T>())
}

// pub fn assume_utc_date_time_opt<'de, D>(deser: D) -> Result<Option<time::OffsetDateTime>, D::Error>
// where
//     D: Deserializer<'de>,
// {
//     let d = match String::deserialize(deser).ok() {
//         Some(d) => d,
//         None => return Ok(None),
//     };
//     time::parse(&(d + " +0000"), "%FT%TZ %z")
//         .map(Some)
//         .map_err(Error::custom)
// }

// pub fn assume_utc_date_time<'de, D>(deser: D) -> Result<time::OffsetDateTime, D::Error>
// where
//     D: Deserializer<'de>,
// {
//     time::parse(&(String::deserialize(deser)? + " +0000"), "%FT%TZ %z").map_err(Error::custom)
// }

// pub fn utc_date_time_alt<'de, D>(deser: D) -> Result<time::OffsetDateTime, D::Error>
// where
//     D: Deserializer<'de>,
// {
//     time::parse(
//         String::deserialize(deser)?.replace(".000Z", "+0000"),
//         "%FT%T%z",
//     )
//     .map_err(Error::custom)
// }

// pub fn utc_date_time_alt_opt<'de, D>(deser: D) -> Result<Option<time::OffsetDateTime>, D::Error>
// where
//     D: Deserializer<'de>,
// {
//     let d = match String::deserialize(deser).ok() {
//         Some(d) => d,
//         None => return Ok(None),
//     };
//     time::parse(d.replace(".000Z", "+0000"), "%FT%T%z")
//         .map(Some)
//         .map_err(Error::custom)
// }

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
