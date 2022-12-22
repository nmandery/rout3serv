pub fn git_comit_sha() -> &'static str {
    env!("VERGEN_GIT_SHA")
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn build_timestamp() -> &'static str {
    env!("VERGEN_BUILD_TIMESTAMP")
}

pub fn long_version() -> &'static str {
    concat!(
        env!("CARGO_PKG_VERSION"),
        " (git: ",
        env!("VERGEN_GIT_SHA"),
        ", build on ",
        env!("VERGEN_BUILD_TIMESTAMP"),
        ")"
    )
}
