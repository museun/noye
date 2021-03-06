pub(self) type Result = anyhow::Result<()>;
use anyhow::Context as _;

// TODO don't do this
pub(self) use crate::{bot::*, db::Table, responses::*, util::*, *};
pub(self) use futures::prelude::*;

mod builtin;
mod gdrive;
mod gfycat;
mod hp;
mod instagram;
mod link_size;
mod pictures;
mod repost;
mod vimeo;
mod youtube;

#[non_exhaustive]
pub struct ModuleInit<R> {
    pub commands: CommandsMap<R>,
    pub passives: PassivesList<R>,
    pub state: State,
}

impl<R> Default for ModuleInit<R> {
    fn default() -> Self {
        Self {
            commands: Default::default(),
            passives: Default::default(),
            state: Default::default(),
        }
    }
}

pub async fn initialize_modules<R>(init: &mut ModuleInit<R>) -> anyhow::Result<()>
where
    R: Responder + Send + 'static,
{
    builtin::initialize_module(init).await?;
    link_size::initialize_module(init).await?;
    repost::initialize_module(init).await?;
    vimeo::initialize_module(init).await?;
    gdrive::initialize_module(init).await?;
    youtube::initialize_module(init).await?;
    instagram::initialize_module(init).await?;
    pictures::initialize_module(init).await?;
    hp::initialize_module(init).await?;
    gfycat::initialize_module(init).await?;

    let config::Web {
        listen_port,
        lookup_ip,
    } = init.state.config().await?.web.clone();
    let addr = format!("0.0.0.0:{}", listen_port)
        .parse::<std::net::SocketAddr>()
        .with_context(|| "cannot parse listen address")?;

    add_external_ip(&mut init.state, &lookup_ip, listen_port).await?;

    let db = init.state.expect_get::<pictures::web::Db>()?.clone();
    let temp = init
        .state
        .expect_get::<crate::http::server::TempStore>()?
        .clone();

    use warp::Filter as _;
    // TODO abstract this out
    let routes = pictures::web::lookup(db).or(crate::http::server::temporary(temp));
    tokio::spawn(warp::serve(routes).run(addr));

    Ok(())
}

async fn add_external_ip(state: &mut State, host: &str, port: u16) -> anyhow::Result<()> {
    let address = reqwest::get(host)
        .await?
        .error_for_status()?
        .text()
        .await
        .map(|s| s.trim().to_string())?;

    state.expect_insert(ExternalIp { address, port })
}

#[derive(Debug, Clone)]
pub struct ExternalIp {
    pub address: String,
    pub port: u16,
}
