pub mod status;
pub use status::status_handler;

pub mod instance_upload;
pub use instance_upload::instance_upload_handler;

pub mod instance_list;
pub use instance_list::instance_list_handler;

pub mod instance_download;
pub use instance_download::instance_download_handler;

pub mod instance_fetch_unsolved;
pub use instance_fetch_unsolved::instance_fetch_unsolved_handler;

pub mod tag_create;
pub use tag_create::tag_create_handler;

pub mod tag_list;
pub use tag_list::tag_list_handler;

pub mod solution_upload;
pub use solution_upload::solution_upload_handler;

pub mod solution_hash_list;
pub use solution_hash_list::solution_hash_list_handler;

// imports used by pretty much every handler
mod common;
