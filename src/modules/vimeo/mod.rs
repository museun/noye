use super::*;

mod client;

#[cfg(test)]
mod tests;

pub(super) async fn initialize_module<R>(init: &mut ModuleInit<R>) -> anyhow::Result<()>
where
    R: Responder + Send + 'static,
{
    init.passives.add(hear_video);
    init.state.expect_insert(client::VimeoClient::default())
}

fn filter(url: &url::Url) -> bool {
    const ACCEPTED: [&str; 2] = ["vimeo.com", "www.vimeo.com"];
    url.domain().filter(|s| ACCEPTED.contains(s)).is_some()
}

pub async fn hear_video<R: Responder>(context: Context, responder: R) -> Result {
    let state = context.state.lock().await;
    let vimeo = state.expect_get::<client::VimeoClient>()?;

    let mut http_client = None;

    context
        .get_links_filter(filter)?
        .into_iter()
        .filter_map(|link| {
            link.path_segments()
                .and_then(|mut s| s.next())
                .map(|s| s.to_string())
        })
        .map(|vid| {
            let (context, responder, vimeo) = (context.clone(), responder.clone(), vimeo.clone());
            let client = http_client
                .get_or_insert_with(crate::http::client::new_client)
                .clone();
            async move {
                let video = vimeo.lookup_video(&vid, client).await?;
                { responder }.say(context, resp_for_video(video)).await
            }
        })
        .collect::<futures::stream::FuturesUnordered<_>>()
        .for_each(|_| async move {})
        .await;

    Ok(())
}

fn resp_for_video(video: client::Video) -> responses::Vimeo {
    responses::Vimeo::Video {
        id: video.id,
        width: video.width,
        height: video.height,
        duration: video.duration.as_timestamp(),
        fps: video.fps.to_string(),
        title: video.title,
        owner: video.owner.name,
    }
}
