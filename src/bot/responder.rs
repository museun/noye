use super::{AnyhowFut, Context, Message, Resolver};

use serde::{Deserialize, Serialize};
use template::{NameCasing::Original, Template};
use tokio::sync::mpsc;

pub trait Responder: Clone + Send + Sync {
    fn say<T>(&mut self, context: Context, template: T) -> AnyhowFut<'_>
    where
        T: Template + Send + Sync + 'static,
        for<'de> T: Serialize + Deserialize<'de>;

    fn reply<T>(&mut self, context: Context, template: T) -> AnyhowFut<'_>
    where
        T: Template + Send + Sync + 'static,
        for<'de> T: Serialize + Deserialize<'de>;
}

#[derive(Clone)]
pub struct WriterResponder {
    writer: mpsc::Sender<String>,
    resolver: Resolver,
}

impl WriterResponder {
    pub fn new(writer: mpsc::Sender<String>, resolver: Resolver) -> Self {
        Self { writer, resolver }
    }
}

impl Responder for WriterResponder {
    fn say<T>(&mut self, context: Context, template: T) -> AnyhowFut<'_>
    where
        T: Template + Send + Sync + 'static,
        for<'de> T: Serialize + Deserialize<'de>,
    {
        let (name, ns, var) = (
            T::name(Original),
            T::namespace(Default::default()),
            template.variant(Default::default()),
        );
        log::trace!("say {}: {}->{}", name, ns, var);

        let resolver = self.resolver.clone();
        let mut writer = self.writer.clone();

        Box::pin(async move {
            let resp = resolve_template(resolver, template).await?;
            writer
                .send(format!("PRIVMSG {} :{}\r\n", &context.args.channel, resp))
                .await?;
            Ok(())
        })
    }

    fn reply<T>(&mut self, context: Context, template: T) -> AnyhowFut<'_>
    where
        T: Template + Send + Sync + 'static,
        for<'de> T: Serialize + Deserialize<'de>,
    {
        let (name, ns, var) = (
            T::name(Original),
            T::namespace(Default::default()),
            template.variant(Default::default()),
        );
        log::trace!("reply {}: {}->{}", name, ns, var);

        let resolver = self.resolver.clone();
        let mut writer = self.writer.clone();

        Box::pin(async move {
            let resp = resolve_template(resolver, template).await?;
            let Message {
                channel, sender, ..
            } = &*context.args;

            writer
                .send(format!("PRIVMSG {} :{}: {}\r\n", &channel, &sender, resp))
                .await?;
            Ok(())
        })
    }
}

pub async fn resolve_template<T>(resolver: Resolver, template: T) -> anyhow::Result<String>
where
    T: Template,
{
    let (name, ns, var) = (
        T::name(Original),
        T::namespace(Default::default()),
        template.variant(Default::default()),
    );

    resolver
        .lock()
        .await
        .resolve(ns, var)
        .ok_or_else(|| anyhow::anyhow!("cannot resolve template for: {}: {}->{}", name, ns, var))
        .and_then(|data| {
            template
                .apply(data)
                .ok_or_else(|| anyhow::anyhow!("invalid template for: {}: {}->{}", name, ns, var))
        })
}
