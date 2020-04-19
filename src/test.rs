#![allow(dead_code)]
pub use crate::{responses::*, *};
pub use futures::prelude::*;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Default, Clone)]
pub struct YamlResponder {
    pub responses: Arc<std::sync::Mutex<Vec<Response>>>,
}

impl YamlResponder {
    pub fn len(&self) -> usize {
        self.responses.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get_say<T>(&self) -> T
    where
        for<'de> T: Deserialize<'de>,
    {
        match self.remove() {
            Response::Say(say) => serde_yaml::from_str(&say).unwrap(),
            d => panic!("expected 'say' got {}", resp_kind(d)),
        }
    }

    pub fn get_reply<T>(&self) -> T
    where
        for<'de> T: Deserialize<'de>,
    {
        match self.remove() {
            Response::Reply(reply) => serde_yaml::from_str(&reply).unwrap(),
            d => panic!("expected 'reply' got {}", resp_kind(d)),
        }
    }

    pub fn get_raw(&self) -> String {
        match self.remove() {
            Response::Raw(raw) => raw,
            d => panic!("expected 'raw' got {}", resp_kind(d)),
        }
    }

    pub fn expect_empty(&self) {
        assert!(
            self.is_empty(),
            "expected no responses, got: {}",
            self.len()
        )
    }

    fn remove(&self) -> Response {
        let responses = &mut *self.responses.lock().unwrap();
        assert!(!responses.is_empty(), "expected a response");
        responses.remove(0)
    }
}

impl Responder for YamlResponder {
    fn say<T>(&mut self, _: Context, template: T) -> AnyhowFut<'_>
    where
        T: template::Template + Send + Sync + 'static,
        T: Serialize,
    {
        Box::pin(async move {
            self.responses
                .lock()
                .unwrap()
                .push(Response::Say(serde_yaml::to_string(&template).unwrap()));
            Ok(())
        })
    }

    fn reply<T>(&mut self, _: Context, template: T) -> AnyhowFut<'_>
    where
        T: template::Template + Send + Sync + 'static,
        T: Serialize,
    {
        Box::pin(async move {
            self.responses
                .lock()
                .unwrap()
                .push(Response::Reply(serde_yaml::to_string(&template).unwrap()));
            Ok(())
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Response {
    Say(String),
    Reply(String),
    Raw(String),
}

fn resp_kind(response: Response) -> &'static str {
    match response {
        Response::Reply(..) => "reply",
        Response::Raw(..) => "raw",
        Response::Say(..) => "say",
    }
}

pub fn set_snapshot_path() {
    let mut settings = insta::Settings::new();
    settings.set_snapshot_path(get_snapshot_path());
    settings.bind_to_thread();
}

pub fn get_snapshot_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("snapshots")
}

pub struct TestEnv {
    responder: YamlResponder,
    data: String,
    sender: String,
    channel: String,
    state: Arc<Mutex<State>>,
}

impl TestEnv {
    pub fn new(data: impl ToString) -> Self {
        let mut config = Config::default();
        config.irc_config.name = "test_bot".into();
        config.irc_config.channels.push("#test_channel".into());
        config.irc_config.owners.push("test_owner".into());

        let mut state = State::default();
        state.insert(CachedConfig::new(config, "noye.toml"));

        Self {
            responder: YamlResponder::default(),
            data: data.to_string(),
            sender: "test_user".into(),
            channel: "#test_channel".into(),
            state: Arc::new(Mutex::new(state)),
        }
    }

    pub fn owner(mut self) -> Self {
        self.sender = "test_owner".into();
        self
    }

    pub fn user(mut self, user: impl ToString) -> Self {
        self.sender = user.to_string();
        self
    }

    pub fn channel(mut self, channel: impl ToString) -> Self {
        self.channel = channel.to_string();
        self
    }

    pub fn insert<T: 'static + Send + Sync>(self, item: T) -> Self {
        assert!(self.state.lock().now_or_never().unwrap().insert(item));
        self
    }

    pub fn config(self, f: impl FnOnce(&mut Config)) -> Self {
        {
            let mut state = self.state.lock().now_or_never().unwrap();
            let mut config = state
                .expect_get_mut::<CachedConfig>()
                .unwrap()
                .get_mut()
                .now_or_never()
                .unwrap()
                .unwrap();
            f(&mut config);
        }
        self
    }

    pub async fn execute<H, F>(self, handler: H) -> YamlResponder
    where
        H: Handler<YamlResponder, Fut = F>,
        F: std::future::Future<Output = anyhow::Result<()>> + Send + 'static,
        F::Output: Send + 'static,
    {
        let msg = crate::Message {
            sender: self.sender,
            channel: self.channel,
            data: self.data,
        };

        let (tx, rx) = tokio::sync::mpsc::channel(32);
        match handler
            .call(
                crate::Context {
                    args: Arc::new(msg),
                    writer: crate::Writer(tx),
                    state: self.state.clone(),
                    quit: Default::default(),
                },
                self.responder.clone(),
            )
            .await
        {
            Ok(..) => {}
            Err(err) if err.is::<crate::util::DontCareSigil>() => {}
            Err(err) => {
                let err = err
                    .chain()
                    .enumerate()
                    .fold(String::new(), |mut a, (i, err)| {
                        a.extend(format!("\n[{}] --> ", i).drain(..));
                        a.extend(err.to_string().drain(..));
                        a
                    });
                panic!("{}", err)
            }
        }

        self.responder
            .responses
            .lock()
            .unwrap()
            .extend(rx.map(Response::Raw).collect::<Vec<_>>().await);

        self.responder
    }
}
