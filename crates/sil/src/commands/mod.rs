//! Command handlers — one module per `sil` subcommand.

mod build;
mod context;
mod init;
mod log;
mod parse;
mod search;
mod source;
mod status;

pub use build::run as build;
pub use context::run as context;
pub use init::run as init;
pub use log::run as log;
pub use parse::run as parse;
pub use search::run as search;
pub use source::fetch as source_fetch;
pub use status::run as status;
