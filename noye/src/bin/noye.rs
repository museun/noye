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
            Command::Run => (),
            Command::DefaultConfig => {
                config::Config::print_default();
                std::process::exit(0);
            }
            Command::DefaultTemplates => {
                config::Config::print_templates();
                std::process::exit(0);
            }
            Command::Help => {
                println!("{}", USAGE);
                std::process::exit(0);
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    flexi_logger::Logger::with_env_or_str("noye=trace")
        .log_to_file()
        .o_append(true)
        .duplicate_to_stderr(flexi_logger::Duplicate::Warn)
        .start()
        .unwrap();

    Command::parse().handle();

    // TODO the bot should be able to hand out LocalSets (via context?)
    let mut runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async move {
        let config = config::Config::load().await?;
        let config::Irc { address, port, .. } = &config.irc_config;

        let client = irc::Client::connect((address.as_str(), *port)).await?;
        bot::Bot::create(config, client).run_loop().await
    })
}
