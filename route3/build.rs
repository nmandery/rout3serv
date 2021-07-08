fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_client(false)
        .build_server(true)
        .out_dir("src/server/api/")
        .compile(&["../proto/route3.proto"], &["../proto"])?;
    Ok(())
}
