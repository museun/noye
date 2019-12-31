#[doc(inline)]
pub use crate::command::{Context, Dispatcher, Event, Noye};

#[doc(inline)]
pub use template::*;

#[doc(inline)]
pub use crate::config::{self, ApiKey, Config};

#[doc(inline)]
pub use crate::irc::{Command, Target};

#[doc(inline)]
pub use crate::matches::Matches;

#[doc(inline)]
pub use crate::format::{CommaSeparated, FileSize, Iso8601, Timestamp};

use futures::prelude::*;
pub async fn concurrent_map<Iter, Item, Func, Fut, Ok, Err, Hint>(
    namespace: &str,
    hint: Hint,
    input: Iter,
    func: Func,
) -> impl Stream<Item = Ok>
where
    Iter: IntoIterator<Item = Item>,
    Func: Copy + FnOnce(Item) -> Fut,
    Fut: TryFuture<Ok = Ok, Error = Err>,
    Err: std::fmt::Display,
    Hint: Into<Option<usize>>,
{
    let (buf_tx, buf_rx) = tokio::sync::mpsc::unbounded_channel();
    stream::iter(input)
        .for_each_concurrent(hint, |item| {
            let buf_tx = buf_tx.clone();
            async move {
                if let Ok(ok) = func(item)
                    .inspect_err(|err| log::warn!("{} produced an error: {}", namespace, err))
                    .await
                {
                    let _ = buf_tx.send(ok);
                }
            }
        })
        .await;
    drop(buf_tx); // drop so the stream will end
    buf_rx
}
