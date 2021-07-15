use vergen::{vergen, Config};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    vergen(Config::default())?;
    println!("cargo:rerun-if-changed=../proto/route3.proto");
    tonic_build::configure()
        .build_client(false)
        .build_server(true)
        .out_dir("src/server/api/")
        .compile(&["../proto/route3.proto"], &["../proto"])?;
    Ok(())
}
