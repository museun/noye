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
