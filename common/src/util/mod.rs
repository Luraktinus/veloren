pub mod logging;
pub use logging::VelorenLogger;

pub const GIT_HASH: &str = include_str!(concat!(env!("OUT_DIR"), "/githash"));
