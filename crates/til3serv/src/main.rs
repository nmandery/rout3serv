#![warn(
    clippy::all,
    clippy::correctness,
    clippy::suspicious,
    clippy::style,
    clippy::complexity,
    clippy::perf,
    nonstandard_style
)]

use clap::{App, Arg, ArgMatches};
use eyre::Result;

use crate::config::ServerConfig;
use crate::server::run_server;
use crate::util::Validate;

mod build_info;
mod config;
mod response;
mod server;
mod state;
mod tile;
mod ui;
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

    let app = App::new(crate::build_info::app_name())
        .version(crate::build_info::version())
        .long_version(long_version.as_str())
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .subcommand(
            App::new("server").about("Start the HTTP server").arg(
                Arg::new("CONFIG-FILE")
                    .help("server configuration file")
                    .required(true),
            ),
        );

    let matches = app.get_matches();

    match matches.subcommand() {
        Some(("server", sc_matches)) => subcommand_server(sc_matches)?,
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
