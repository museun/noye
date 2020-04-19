use anyhow::Context as _;

pub fn new_client() -> reqwest::Client {
    reqwest::Client::new()
}

#[derive(serde::Serialize)]
pub struct NoQuery;

#[allow(dead_code)]
pub async fn get_body(
    client: reqwest::Client,
    url: &str,
    headers: &[(&'static str, &str)],
) -> anyhow::Result<String> {
    let mut req = client.get(url);
    for &(k, v) in headers {
        req = req.header(k, v);
    }

    client
        .execute(req.build()?)
        .await
        .with_context(|| format!("cannot get url '{}'", url))?
        .error_for_status()
        .with_context(|| format!("cannot get url '{}'", url))?
        .text()
        .await
        .with_context(|| format!("cannot get body for '{}'", url))
}

pub async fn get_json<'a, T, Q>(
    client: reqwest::Client,
    url: &'a str,
    query: &'a Q,
    headers: &[(&'static str, &str)],
) -> anyhow::Result<T>
where
    for<'de> T: serde::Deserialize<'de>,
    Q: serde::Serialize + 'a,
{
    let mut req = client.get(url);
    if let Some(query) = query.into() {
        req = req.query(query);
    }

    for &(k, v) in headers {
        req = req.header(k, v);
    }

    client
        .execute(req.build()?)
        .await
        .with_context(|| format!("cannot get url '{}'", url))?
        .error_for_status()
        .with_context(|| format!("cannot get url '{}'", url))?
        .json()
        // .text()
        .await
        // .map(|s| {
        //     eprintln!("got json from: {}", url);
        //     let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        //     std::fs::write(
        //         format!(
        //             "{}.json",
        //             url.replace("http://", "")
        //                 .replace("https://", "")
        //                 .replace('/', "_")
        //                 .replace(".", "_")
        //         ),
        //         serde_json::to_string_pretty(&v).unwrap(),
        //     )
        //     .unwrap();
        //     serde_json::from_value(v).unwrap()
        // })
        .with_context(|| format!("cannot get json for '{}'", url))
}

pub async fn head(client: reqwest::Client, url: &str) -> anyhow::Result<reqwest::Response> {
    let req = client.head(url).build()?;
    client
        .execute(req)
        .await
        .with_context(|| format!("cannot get HEAD for '{}'", url))?
        .error_for_status()
        .with_context(|| format!("cannot get url '{}'", url))
}
