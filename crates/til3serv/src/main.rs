use clap::{App, Arg, ArgMatches, SubCommand};
use eyre::Result;

use crate::config::ServerConfig;
use crate::server::run_server;
use crate::util::Validate;

mod build_info;
mod config;
mod response;
mod server;
mod tile;
mod util;

fn main() -> Result<()> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let long_version = format!(
        "{} (git: {}, build on {})",
        crate::build_info::version(),
        crate::build_info::git_comit_sha(),
        crate::build_info::build_timestamp()
    );

    let app = App::new(env!("CARGO_PKG_NAME"))
        .version(crate::build_info::version())
        .long_version(long_version.as_str())
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .subcommand(
            SubCommand::with_name("server")
                .about("Start the HTTP server")
                .arg(
                    Arg::with_name("CONFIG-FILE")
                        .help("server configuration file")
                        .required(true),
                ),
        );

    let matches = app.get_matches();

    match matches.subcommand() {
        ("server", Some(sc_matches)) => subcommand_server(sc_matches)?,
        _ => {
            println!("unknown subcommand");
        }
    }
    Ok(())
}

fn subcommand_server(sc_matches: &ArgMatches) -> eyre::Result<()> {
    let config_contents = std::fs::read_to_string(sc_matches.value_of("CONFIG-FILE").unwrap())?;
    let config: ServerConfig = serde_yaml::from_str(&config_contents)?;
    config.validate()?;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(run_server(config))?;
    Ok(())
}
