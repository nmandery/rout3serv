use npm_rs::*;
use vergen::{vergen, Config};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=ui");

    let _exit_status = NpmEnv::default()
        .with_node_env(&NodeEnv::from_cargo_profile().unwrap_or_default())
        .set_path("ui")
        .init_env()
        .install(None)
        .run("build-using-env")
        .exec()?;

    vergen(Config::default())?;

    Ok(())
}
