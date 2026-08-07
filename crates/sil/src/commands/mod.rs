//! Command handlers — one module per `sil` subcommand.

mod build;
mod cite;
mod context;
mod digest;
mod doctor;
mod init;
mod log;
mod mcp;
mod parse;
mod promote;
mod propose;
mod search;
mod settings_cmd;
mod source;
mod split;
mod status;
mod structure_cmd;
mod template_cmd;
mod todo;

pub use build::run as build;
pub use cite::run as cite;
pub use context::run as context;
pub use digest::run as digest;
pub use doctor::run as doctor;
pub use init::run as init;
pub use log::run as log;
pub use mcp::run as mcp;
pub use parse::run as parse;
pub use promote::run as promote;
pub use propose::run as propose;
pub use search::run as search;
pub use settings_cmd::run as settings;
pub use source::{
    doctor as source_doctor, fetch as source_fetch, list as source_list,
    rank_draft as source_rank_draft, read as source_read, remove as source_remove,
};
pub use split::run as split;
pub use status::run as status;
pub use structure_cmd::{list as structure_list, set_completion as structure_set};
pub use template_cmd::run as template;
pub use todo::run as todo;
