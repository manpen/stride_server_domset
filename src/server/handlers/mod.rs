pub mod status;
pub use status::status_handler;

#[cfg(feature = "admin-api")]
pub mod instance_upload;
#[cfg(feature = "admin-api")]
pub use instance_upload::instance_upload_handler;

#[cfg(feature = "admin-api")]
pub mod instance_delete;
#[cfg(feature = "admin-api")]
pub use instance_delete::instance_delete_handler;

pub mod instance_list;
pub use instance_list::{instance_list_download_handler, instance_list_handler};

pub mod instance_download;
pub use instance_download::instance_download_handler;

#[cfg(feature = "admin-api")]
pub mod instance_update_meta;
#[cfg(feature = "admin-api")]
pub use instance_update_meta::instance_update_meta_handler;

#[cfg(feature = "admin-api")]
pub mod tag_create;
#[cfg(feature = "admin-api")]
pub use tag_create::tag_create_handler;

#[cfg(feature = "admin-api")]
pub mod debug_restart;
#[cfg(feature = "admin-api")]
pub use debug_restart::debug_restart_handler;

pub mod tag_list;
pub use tag_list::tag_list_handler;

pub mod solution_upload;
pub use solution_upload::solution_upload_handler;

pub mod solution_hash_list;
pub use solution_hash_list::solution_hash_list_handler;

// imports used by pretty much every handler
mod common;
