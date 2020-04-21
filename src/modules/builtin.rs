use super::*;

pub(super) async fn initialize_module<R>(init: &mut ModuleInit<R>) -> anyhow::Result<()>
where
    R: Responder + Send + 'static,
{
    init.commands.add("join", join)?;
    init.commands.add("part", part)?;
    init.commands.add("uptime", uptime)?;
    init.commands.add("restart", restart)?;
    init.commands.add("respawn", respawn)?;
    init.commands.add("logs", get_logs)?;

    init.state.expect_insert(StartTime::default())
}

pub async fn join<R: Responder>(context: Context, mut responder: R) -> Result {
    context.expect_owner(&mut responder).await?;
    if let Some(chan) = context.command_args().get(0) {
        return context.writer.clone().join(chan).await;
    }
    responder.reply(context, Join::ExpectedChannel).await
}

pub async fn part<R: Responder>(mut context: Context, mut responder: R) -> Result {
    context.expect_owner(&mut responder).await?;
    let chan = &context.args.channel;
    context.writer.part(chan).await
}

pub async fn uptime<R: Responder>(context: Context, mut responder: R) -> Result {
    let uptime = context
        .state
        .lock()
        .await
        .expect_get::<StartTime>()?
        .as_readable_time();
    responder.say(context, Uptime::Uptime { uptime }).await
}

pub async fn get_logs<R: Responder>(context: Context, mut responder: R) -> Result {
    let state = context.state.lock().await;
    let temp = state.expect_get::<crate::http::server::TempStore>()?;
    let log_file = state.expect_get::<crate::LogFile>()?;
    let ExternalIp { address, port } = state.expect_get::<ExternalIp>()?.clone();

    use rand::prelude::*;

    let mut rng = rand::rngs::SmallRng::from_entropy();
    let id = temp
        .inner
        .write()
        .await
        .insert_text_file(&mut rng, log_file.0.clone())
        .await?;

    let link = format!("http://{}:{}/t/{}", address, port, id);
    let template = responses::TempStore::Link { link };
    responder.say(context.clone(), template).await
}

pub async fn restart<R: Responder>(context: Context, mut responder: R) -> Result {
    context.expect_owner(&mut responder).await?;
    let addr = context.config().await?.modules.restart.address;
    Supervisor::Restart.write(&addr).await
}

pub async fn respawn<R: Responder>(context: Context, mut responder: R) -> Result {
    context.expect_owner(&mut responder).await?;
    let delay = context
        .command_args()
        .get(0)
        .and_then(|s| s.parse().ok())
        .unwrap_or(15);
    let addr = context.config().await?.modules.restart.address;
    Supervisor::Delay(delay).write(&addr).await
}

pub struct StartTime(pub tokio::time::Instant);

impl Default for StartTime {
    fn default() -> Self {
        Self(tokio::time::Instant::now())
    }
}

impl StartTime {
    fn as_readable_time(&self) -> String {
        self.0.elapsed().as_readable_time()
    }
}

#[derive(Debug, Copy, Clone)]
enum Supervisor {
    Restart,
    Delay(u16),
}

impl Supervisor {
    async fn write(self, addr: &str) -> anyhow::Result<()> {
        use tokio::{io::*, net::TcpStream};
        let mut stream = TcpStream::connect(addr)
            .await
            .with_context(|| format!("cannot connect to restart daemon at '{}'", addr))?;

        match self {
            Self::Restart => stream.write_all(b"RESTART\0").await?,
            Self::Delay(d) => {
                stream
                    .write_all(format!("DELAY {}\0", d).as_bytes())
                    .await?
            }
        }

        stream.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::test::*;
    use tokio::prelude::*;

    #[tokio::test]
    async fn join_not_owner() {
        set_snapshot_path();

        let responses = TestEnv::new("!join").execute(super::join).await;
        insta::assert_yaml_snapshot!(responses.get_reply::<responses::Builtin>());
        responses.expect_empty();
    }

    #[tokio::test]
    async fn join_no_channel() {
        set_snapshot_path();

        let responses = TestEnv::new("!join").owner().execute(super::join).await;
        insta::assert_yaml_snapshot!(responses.get_reply::<responses::Join>());
        responses.expect_empty();
    }

    #[tokio::test]
    async fn join_channel() {
        set_snapshot_path();

        let responses = TestEnv::new("!join #test")
            .owner()
            .execute(super::join)
            .await;
        insta::assert_yaml_snapshot!(responses.get_raw());
        responses.expect_empty();
    }

    #[tokio::test]
    async fn part_not_owner() {
        set_snapshot_path();

        let responses = TestEnv::new("!part").execute(super::part).await;
        insta::assert_yaml_snapshot!(responses.get_reply::<responses::Builtin>());
        responses.expect_empty();
    }

    #[tokio::test]
    async fn part_channel() {
        set_snapshot_path();

        let responses = TestEnv::new("!part").owner().execute(super::part).await;
        insta::assert_yaml_snapshot!(responses.get_raw());
        responses.expect_empty();
    }

    #[tokio::test]
    async fn uptime() {
        set_snapshot_path();

        tokio::time::pause();
        let start = super::StartTime::default();
        tokio::time::advance(std::time::Duration::from_secs(10)).await;

        let responses = TestEnv::new("!uptime")
            .insert(start)
            .execute(super::uptime)
            .await;
        insta::assert_yaml_snapshot!(responses.get_say::<responses::Uptime>());
        responses.expect_empty();
    }

    #[tokio::test]
    async fn restart_not_owner() {
        set_snapshot_path();

        let mut listen = tokio::net::TcpListener::bind("localhost:0").await.unwrap();
        let addr = listen.local_addr().unwrap().to_string();

        let responses = TestEnv::new("!restart")
            .config(|config| config.modules.restart.address = addr.clone())
            .execute(super::restart)
            .await;
        insta::assert_yaml_snapshot!(responses.get_reply::<responses::Builtin>());
        responses.expect_empty();
        assert!(listen.accept().now_or_never().is_none());
    }

    #[tokio::test]
    async fn restart() {
        set_snapshot_path();

        let mut listen = tokio::net::TcpListener::bind("localhost:0").await.unwrap();
        let addr = listen.local_addr().unwrap().to_string();

        let responses = TestEnv::new("!restart")
            .config(|config| config.modules.restart.address = addr)
            .owner()
            .execute(super::restart)
            .await;
        responses.expect_empty();
        let data = accept_and_read(&mut listen).await;
        assert_eq!(data, b"RESTART\0");
    }

    #[tokio::test]
    async fn respawn_not_owner() {
        set_snapshot_path();

        let mut listen = tokio::net::TcpListener::bind("localhost:0").await.unwrap();
        let addr = listen.local_addr().unwrap().to_string();

        let responses = TestEnv::new("!respawn")
            .config(|config| config.modules.restart.address = addr.clone())
            .execute(super::respawn)
            .await;
        insta::assert_yaml_snapshot!(responses.get_reply::<responses::Builtin>());
        responses.expect_empty();
        assert!(listen.accept().now_or_never().is_none());
    }

    #[tokio::test]
    async fn respawn_default() {
        set_snapshot_path();

        let mut listen = tokio::net::TcpListener::bind("localhost:0").await.unwrap();
        let addr = listen.local_addr().unwrap().to_string();

        let responses = TestEnv::new("!respawn")
            .config(|config| config.modules.restart.address = addr.clone())
            .owner()
            .execute(super::respawn)
            .await;
        responses.expect_empty();
        let data = accept_and_read(&mut listen).await;
        assert_eq!(data, b"DELAY 15\0");
    }

    #[tokio::test]
    async fn respawn_custom() {
        set_snapshot_path();

        let mut listen = tokio::net::TcpListener::bind("localhost:0").await.unwrap();
        let addr = listen.local_addr().unwrap().to_string();

        let responses = TestEnv::new("!respawn 30")
            .config(|config| config.modules.restart.address = addr)
            .owner()
            .execute(super::respawn)
            .await;
        responses.expect_empty();
        let data = accept_and_read(&mut listen).await;
        assert_eq!(data, b"DELAY 30\0");
    }

    async fn accept_and_read(listener: &mut tokio::net::TcpListener) -> Vec<u8> {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut data = vec![];
        let _ = socket.read_to_end(&mut data).await.unwrap();
        data
    }
}
