pub use crate::bot::prelude::*;
pub use crate::irc::*;

pub use futures::prelude::*;
pub use tokio::sync::mpsc;

pub fn say_template<T: Template>(context: &Context, template: T) -> String {
    let data = crate::command::resolve_template(template).unwrap();
    match context.target().unwrap() {
        Target::Channel(target) | Target::Private(target) => {
            format!("PRIVMSG {} :{}", target, data)
        }
    }
}

pub fn reply_template<T: Template>(context: &Context, template: T) -> String {
    let data = crate::command::resolve_template(template).unwrap();
    match context.target().unwrap() {
        Target::Channel(target) => format!(
            "PRIVMSG {} :{}: {}",
            target,
            context.nick().expect("nick must be attached to message"),
            data
        ),
        Target::Private(target) => format!("PRIVMSG {} :{}", target, data),
    }
}

// TODO allow for asserting on errors
pub fn check_error<F, Fut>(func: F, input: Context)
where
    F: Copy + FnOnce(Context, Noye) -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    let (tx, mut rx) = mpsc::channel(32);
    let noye = Noye::new(tx);
    tokio_test::block_on(async move {
        func(input, noye).await.unwrap_err();
        assert!(rx.next().await.is_none())
    });
}

pub fn check<F, Fut>(func: F, input: Context, mut output: Vec<&str>)
where
    F: Copy + FnOnce(Context, Noye) -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    let (tx, mut rx) = mpsc::channel(32);
    let noye = Noye::new(tx);

    tokio_test::block_on(async move {
        if let Err(err) = func(input.clone(), noye).await {
            panic!("failed to run '{}' on '{:#?}'", err, input.message());
        }
        let mut index = 0_usize;
        while let Some(msg) = rx.next().await {
            if output.is_empty() {
                panic!("got input: ({}) '{}' but output was empty", index, msg);
            }
            let left = output.remove(0);
            assert_eq!(
                left,
                msg,
                "expected at output pos: {}. '{}' != '{}'",
                index,
                left.escape_debug(),
                msg.escape_debug()
            );
            index += 1;
        }
    });
}
