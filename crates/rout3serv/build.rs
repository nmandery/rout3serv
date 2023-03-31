use std::fs::rename;
use std::path::Path;

use vergen::EmitBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    EmitBuilder::builder()
        .build_timestamp()
        .git_sha(true)
        .emit()?;

    println!("cargo:rerun-if-changed=proto/rout3serv.proto");
    tonic_build::configure()
        .build_client(false)
        .build_server(true)
        .out_dir("src/grpc/api/")
        .compile(&["proto/rout3serv.proto"], &["proto"])?;

    let tonic_output_path = Path::new("src/grpc/api/rout3serv.rs");
    if tonic_output_path.exists() {
        rename(tonic_output_path, Path::new("src/grpc/api/generated.rs"))?;
    }
    Ok(())
}
