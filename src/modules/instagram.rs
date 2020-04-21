use super::*;

use select::{
    document::Document,
    predicate::{Name, Predicate, Text},
};

pub(super) async fn initialize_module<R>(init: &mut ModuleInit<R>) -> anyhow::Result<()>
where
    R: Responder + Send + 'static,
{
    init.passives.add(hear_instagram);
    Ok(())
}

pub async fn hear_instagram<R: Responder>(context: Context, responder: R) -> Result {
    let client = crate::http::client::new_client();

    let set: futures::stream::FuturesUnordered<_> = context
        .get_links_filter(filter)?
        .into_iter()
        .map(|url| {
            let (context, responder, client) = (context.clone(), responder.clone(), client.clone());
            async move {
                let title = match get_title(client, url.as_str()).await? {
                    Some(title) => title,
                    None => return Ok(()),
                };
                { responder }
                    .say(context, responses::Instagram::Title { title })
                    .await
            }
        })
        .collect();

    set.for_each(|_| async move {}).await;
    Ok(())
}

fn filter(url: &url::Url) -> bool {
    url.domain().filter(|&s| s == "www.instagram.com").is_some() && !url.path().is_empty()
}

async fn get_title(client: reqwest::Client, url: &str) -> anyhow::Result<Option<String>> {
    const BAD: &[&str] = &[
        "on Instagram",
        "• Instagram photos",
        "’s Instagram",
        "Instagram profile",
    ];

    let body = crate::http::client::get_body(client, url, &[]).await?;

    let document = Document::from(body.as_str());
    let title = match document
        .find(Name("title").descendant(Text))
        .flat_map(|d| d.as_text())
        .map(|s| s.trim())
        .next()
    {
        Some(title) => title,
        None => return Ok(None),
    };

    let mut out = None;
    for bad in BAD {
        if let Some(head) = title
            .splitn(2, bad)
            .next()
            .filter(|head| head.len() < title.len())
            .map(|s| s.trim())
        {
            out.replace(head.to_string());
            break;
        }
    }
    out.get_or_insert_with(|| title.to_string());
    Ok(out.filter(|s| s != "Instagram"))
}
