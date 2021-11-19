use vergen::{vergen, Config};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=src-web");
    println!("cargo:rerun-if-changed=package.json");
    println!("cargo:rerun-if-changed=tsconfig.json");
    println!("cargo:rerun-if-changed=webpack.config.js");

    std::process::Command::new("npm").args(["i"]).status()?;
    std::process::Command::new("npm")
        .env(
            "NODE_ENV",
            match std::env::var("PROFILE")?.as_str() {
                "debug" => "development".to_string(),
                "release" => "production".to_string(),
                x => x.to_string(),
            },
        )
        .args(["run", "build-using-env"])
        .status()?;

    vergen(Config::default())?;
    Ok(())
}
