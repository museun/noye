use noye::{bot, config, irc};

const USAGE: &str = r##"
usage:
    noye run               -- run the bot

    noye help              -- show this help message

    noye default-config    -- prints out the default configuration
                              you can redirect this to a file with:
                              noye default-config > config.toml

    noye default-templates -- prints out the default templates
                              you can redirect this to a file with:
                              noye default-templates > templates.toml
"##;

enum Command {
    Run,
    DefaultConfig,
    DefaultTemplates,
    Help,
}

impl Command {
    fn parse() -> Command {
        match std::env::args().nth(1).as_ref().map(|s| s.as_str()) {
            Some("run") | None => Command::Run,
            Some("default-config") => Command::DefaultConfig,
            Some("default-templates") => Command::DefaultTemplates,
            Some("help") | _ => Command::Help,
        }
    }

    fn handle(self) {
        match self {
            Command::Run => return,
            Command::DefaultConfig => config::Config::print_default(),
            Command::DefaultTemplates => config::Config::print_templates(),
            Command::Help => println!("{}", USAGE),
        };
        std::process::exit(0);
    }
}

fn init_logger(level: config::LogLevel) -> anyhow::Result<()> {
    use fern::colors::{Color, ColoredLevelConfig};
    let level: log::LevelFilter = level.into();
    let colors = ColoredLevelConfig::new()
        .trace(Color::BrightBlack)
        .debug(Color::White)
        .info(Color::Green)
        .warn(Color::BrightYellow)
        .error(Color::BrightRed);

    let stdout = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "[{} {}][{}] {}",
                chrono::Utc::now().format("%F %H:%M:%S%.3f"),
                format!("{: >5}", colors.color(record.level())),
                record.target(),
                message,
            ))
        })
        .level_for("noye", level)
        .chain(std::io::stderr());

    let log_file = fern::Dispatch::new()
        .format(move |out, message, record| {
            out.finish(format_args!(
                "{} {} {} | {}",
                chrono::Utc::now().format("%F %H:%M:%S%.3f%:z"),
                record.level(),
                record.target(),
                message,
            ))
        })
        .level_for("noye", log::LevelFilter::Trace)
        .chain(fern::log_file("noye.log")?);

    fern::Dispatch::new()
        .chain(stdout)
        .chain(log_file)
        .apply()?;

    Ok(())
}

fn main() -> anyhow::Result<()> {
    Command::parse().handle();

    let mut runtime = tokio::runtime::Runtime::new().unwrap();
    let config = runtime.block_on(async move { config::Config::load().await })?;
    init_logger(config.log_level)?;

    runtime.block_on(async move {
        let config::Irc { address, port, .. } = &config.irc_config;
        let client = irc::Client::connect((address.as_str(), *port)).await?;
        bot::Bot::create(config, client).run_loop().await
    })
}
