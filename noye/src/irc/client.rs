use super::*;

use tokio::io::{BufReader, ReadHalf, WriteHalf};
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio::prelude::*;

use anyhow::{Context, Result};

#[derive(Copy, Clone, Debug)]
pub struct Registration<'a> {
    pub nick: &'a str,
    pub user: &'a str,
    pub real: &'a str,
}

pub struct Client<T> {
    read: BufReader<ReadHalf<T>>,
    write: WriteHalf<T>,
}

impl Client<TcpStream> {
    pub async fn connect(addr: impl ToSocketAddrs) -> Result<Self> {
        TcpStream::connect(addr)
            .await
            .map(Self::from_read_write)
            .map_err(Into::into)
    }
}

// TODO remove basically every allocation here by not using the "easy" functions
impl<T> Client<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub fn from_read_write(inner: T) -> Self {
        let (read, write) = tokio::io::split(inner);
        let read = BufReader::new(read);
        Self { read, write }
    }

    pub async fn register(&mut self, reg: Registration<'_>) -> Result<()> {
        let Registration { nick, user, real } = reg;
        self.write(format!("NICK {}", nick)).await?;
        self.write(format!("USER {} * 8 :{}", user, real)).await
    }

    pub async fn read(&mut self) -> Result<Message> {
        let mut line = String::new();
        if 0 == self
            .read
            .read_line(&mut line)
            .await
            .context("cannot read message")?
        {
            anyhow::bail!("disconnected");
        }

        Message::parse(&line).with_context(|| format!("input '{}'", line.escape_debug()))
    }

    pub async fn write(&mut self, resp: impl ToString) -> Result<()> {
        let resp = resp.to_string();
        log::trace!("-> {}", resp);

        macro_rules! map {
            ($expr:expr) => {
                $expr.with_context(|| format!("line: {}", resp.clone().escape_debug()))
            };
        }

        map!(self.write.write_all(resp.as_bytes()).await)?;
        map!(self.write.write_all(b"\r\n").await)?;
        map!(self.write.flush().await)
    }
}
