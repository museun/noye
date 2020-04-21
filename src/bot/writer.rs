use std::fmt::Display;
use tokio::sync::mpsc;

#[derive(Clone)]
pub struct Writer(pub mpsc::Sender<String>);

impl Writer {
    pub async fn join(&mut self, channel: impl Display) -> anyhow::Result<()> {
        let channel = channel.to_string();
        log::debug!("joining {}", channel);
        self.0.send(format!("JOIN {}\r\n", channel)).await?;
        Ok(())
    }

    pub async fn part(&mut self, channel: impl Display) -> anyhow::Result<()> {
        let channel = channel.to_string();
        log::debug!("leaving {}", channel);
        self.0.send(format!("PART {}\r\n", channel)).await?;
        Ok(())
    }

    pub async fn nick(&mut self, nick: impl Display) -> anyhow::Result<()> {
        self.0.send(format!("NICK {}\r\n", nick)).await?;
        Ok(())
    }

    pub async fn raw(&mut self, data: impl Display) -> anyhow::Result<()> {
        self.0.send(format!("{}\r\n", data)).await?;
        Ok(())
    }
}
