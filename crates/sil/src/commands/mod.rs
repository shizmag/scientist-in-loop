//! Command handlers — one module per `sil` subcommand.

mod build;
mod cite;
mod context;
mod init;
mod log;
mod parse;
mod promote;
mod propose;
mod search;
mod source;
mod split;
mod status;
mod structure_cmd;

pub use build::run as build;
pub use cite::run as cite;
pub use context::run as context;
pub use init::run as init;
pub use log::run as log;
pub use parse::run as parse;
pub use promote::run as promote;
pub use propose::run as propose;
pub use search::run as search;
pub use source::{fetch as source_fetch, list as source_list, remove as source_remove};
pub use split::run as split;
pub use status::run as status;
pub use structure_cmd::{list as structure_list, set_completion as structure_set};
