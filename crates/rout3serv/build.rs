use std::fs::rename;
use std::path::Path;

use vergen::{vergen, Config};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    vergen(Config::default())?;
    println!("cargo:rerun-if-changed=proto/rout3serv.proto");
    tonic_build::configure()
        .build_client(false)
        // do not format, as we need to move the output first to make the module name
        // match. All for proper IDE support to avoid including the output.
        .format(false)
        .build_server(true)
        .out_dir("src/server/api/")
        .compile(&["proto/rout3serv.proto"], &["proto"])?;

    let tonic_output_path = Path::new("src/server/api/rout3serv.rs");
    if tonic_output_path.exists() {
        rename(tonic_output_path, Path::new("src/server/api/generated.rs"))?;
    }
    Ok(())
}