pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn engine_version() -> &'static str {
    ENGINE_VERSION
}
