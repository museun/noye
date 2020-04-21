use super::*;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

pub(super) async fn initialize_module<R>(init: &mut ModuleInit<R>) -> Result
where
    R: Responder + Send + 'static,
{
    init.state.expect_insert(Cache::default())?;
    init.commands.add("hp", get_info)?;
    Ok(())
}

async fn get_info<R: Responder>(context: Context, mut responder: R) -> Result {
    let state = context.state.lock().await;
    let cache = state.expect_get::<Cache>()?.clone();

    let id = match context.without_command() {
        Some(args) => args.trim().to_uppercase(),
        None => {
            let resp = responses::Concert::NoIdProvided;
            return responder.reply(context.clone(), resp).await;
        }
    };

    let concert = cache.lookup(&id).await?;
    let mcs = concert.sum_mcs();
    let totals = concert.sum_all();

    responder
        .say(
            context.clone(),
            responses::Concert::Title {
                id,
                title: concert.title.clone(),
            },
        )
        .await?;

    for (i, total) in totals.into_iter().enumerate() {
        responder
            .say(
                context.clone(),
                responses::Concert::Total {
                    num: (i + 1).to_string(),
                    chapter_count: total.count.to_string(),
                    length: total.total,
                },
            )
            .await?;
    }
    for (i, mc) in mcs.into_iter().enumerate() {
        if mc.count == 0 {
            continue;
        }
        responder
            .say(
                context.clone(),
                responses::Concert::Talks {
                    num: (i + 1).to_string(),
                    chapter_count: mc.count.to_string(),
                    length: mc.total,
                    chapters: mc
                        .chapters
                        .into_iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                        .join(", "),
                },
            )
            .await?;
    }

    Ok(())
}

#[derive(Default)]
struct Cache {
    data: Arc<Mutex<HashMap<String, Arc<Concert>>>>,
}

impl Cache {
    async fn lookup(&self, id: impl ToString) -> anyhow::Result<Arc<Concert>> {
        use select::predicate::*;

        let mut data = self.data.lock().await;
        let id = id.to_string();
        if let Some(id) = data.get(&id) {
            return Ok(id.clone());
        }

        let client = crate::http::client::new_client();
        let body = client
            .get(&format!(
                "http://www.helloproject.com/release/detail/{}",
                id
            ))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let doc = select::document::Document::from(body.as_str());
        let root = match doc.find(Attr("id", "rd_right")).next() {
            Some(root) => root,
            None => anyhow::bail!("cannot find table"),
        };

        let title = root
            .find(Name("h2").descendant(Text))
            .map(|s| s.text())
            .next()
            .unwrap();

        let mut tracks = vec![];

        let mut discs = 0;
        for (disc, body) in root
            .find(Class("typeB"))
            .enumerate()
            .map(|(i, body)| (i + 1, body))
        {
            discs += 1;
            for tr in body.find(Name("tbody").descendant(Name("tr"))) {
                let mut v = tr
                    .find(Name("td").and(Not(Class("hide_cell"))).descendant(Text))
                    .flat_map(|s| s.as_text());

                let track = match v.next().and_then(|s| s.parse().ok()) {
                    Some(id) => id,
                    _ => continue,
                };

                let v = v.collect::<Vec<_>>();

                let song = v
                    .iter()
                    .take_while(|k| !k.starts_with('\n'))
                    .map(|k| k.trim())
                    .collect::<String>();

                let kind = if song.contains("MC") {
                    Kind::Mc
                } else if song.contains("VTR") {
                    Kind::Vtr
                } else {
                    Kind::Song(song)
                };

                let seconds = match v
                    .iter()
                    .skip_while(|k| !k.starts_with('\n'))
                    .map(|s| s.trim())
                    .map(parse_seconds)
                    .next()
                {
                    Some(dur) => dur,
                    None => continue,
                };

                tracks.push(Track {
                    disc,
                    track,
                    seconds,
                    kind,
                });
            }
        }

        let concert = Arc::new(Concert {
            title,
            discs,
            tracks,
        });
        data.insert(id, concert.clone());
        Ok(concert)
    }
}

#[derive(Debug)]
struct Concert {
    title: String,
    discs: usize,
    tracks: Vec<Track>,
}
impl Concert {
    fn sum(&self, kinds: &[Kind]) -> Vec<Summary> {
        let kinds = kinds.iter().map(std::mem::discriminant).collect::<Vec<_>>();

        let totals = self.tracks.iter().enumerate().fold(
            vec![(0, 0, vec![]); self.discs],
            |mut v, (i, t)| {
                if !kinds.contains(&std::mem::discriminant(&t.kind)) {
                    return v;
                }

                v[t.disc - 1].0 += t.seconds;
                v[t.disc - 1].1 += 1;
                v[t.disc - 1].2.push(i + 1);
                v
            },
        );

        totals
            .into_iter()
            .map(|(secs, breaks, chapters)| Summary {
                count: breaks,
                total: format_seconds(secs),
                chapters,
            })
            .collect()
    }

    fn sum_mcs(&self) -> Vec<Summary> {
        self.sum(&[Kind::Mc])
    }

    fn sum_all(&self) -> Vec<Summary> {
        self.sum(&[Kind::Mc, Kind::Vtr, Kind::Song("".into())])
    }
}

#[derive(Debug)]
struct Track {
    disc: usize,
    track: usize,
    seconds: u64,
    kind: Kind,
}

#[derive(Debug)]
enum Kind {
    Vtr,
    Mc,
    Song(String),
}

#[derive(Debug)]
struct Summary {
    count: usize,
    total: String,
    chapters: Vec<usize>,
}

fn parse_seconds(input: &str) -> u64 {
    let mut t = input.split(':').flat_map(|s| s.parse::<u64>().ok());
    let (mins, secs) = (t.next().unwrap(), t.next().unwrap());
    secs + (mins * 60)
}

// TODO this is duplicated from format
fn format_seconds(time: u64) -> String {
    let hours = time / (60 * 60);
    let minutes = (time / 60) % 60;
    let seconds = time % 60;
    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}
