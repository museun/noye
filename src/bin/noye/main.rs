use noye::{Bot, WriterResponder};
use tokio::{io::BufStream, net::TcpStream, prelude::*, sync::mpsc};

const CONFIG_LOCATION: &str = "noye.toml";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "noye=trace");

    let opts = alto_logger::Options::default()
        .with_time(alto_logger::options::TimeConfig::date_time_format("%c"));
    let file = alto_logger::FileLogger::timestamp(opts.clone(), "noye.log")?;
    let log_file = file.file_name().map(ToOwned::to_owned).unwrap();

    let logger = alto_logger::MultiLogger::new()
        .with(alto_logger::TermLogger::new(opts.clone())?)
        .with(file);
    alto_logger::init(logger).expect("init logger");

    let config = noye::Config::load(CONFIG_LOCATION).await?;

    let noye::config::Irc {
        name,
        real,
        user,
        address,
        ..
    } = &config.irc_config;

    let (tx, mut rx) = mpsc::channel::<String>(64);

    let mut writer = noye::Writer(tx.clone());
    writer.raw(format!("NICK {}", &name)).await?;
    writer.raw(format!("USER {} * 8 :{}", &user, &real)).await?;

    let mut stream = BufStream::new(TcpStream::connect(&address).await?);

    let mut init = noye::modules::ModuleInit::default();

    init.state
        .insert(noye::CachedConfig::new(config, CONFIG_LOCATION));
    init.state.insert(noye::LogFile(log_file.into()));
    // to configure this
    let temp = noye::web::TempStore::default();
    temp.start_culling();
    init.state.insert(temp);

    noye::modules::initialize_modules(&mut init).await?;
    let noye::modules::ModuleInit {
        commands,
        passives,
        state,
        ..
    } = init;

    let mut bot = Bot::<WriterResponder>::new(state, writer, commands, passives);
    let responder = WriterResponder::new(
        tx,
        noye::resolver::new(template::MemoryStore::new(
            noye::DEFAULT_TEMPLATES, //
            template::load_toml,
        ))?,
    );

    let quit = bot.quit.clone();
    let mut string = String::new();
    loop {
        tokio::select! {
            Ok(_) = stream.read_line(&mut string) => {
                bot.handle(&string, responder.clone()).await?;
                string.clear();
            }
            Some(data) = rx.recv() => {
                stream.write_all(data.as_bytes()).await?;
                stream.flush().await?;
            }
            _ = quit.notified() => break,
            else => break
        }
    }

    Ok(())
}
