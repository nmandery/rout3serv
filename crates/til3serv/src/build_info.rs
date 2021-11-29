pub const fn git_comit_sha() -> &'static str {
    env!("VERGEN_GIT_SHA")
}

pub const fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub const fn build_timestamp() -> &'static str {
    env!("VERGEN_BUILD_TIMESTAMP")
}

pub const fn app_name() -> &'static str {
    env!("CARGO_PKG_NAME")
}
