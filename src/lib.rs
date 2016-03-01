#[macro_use]
extern crate quick_error;
#[macro_use]
extern crate nix;
extern crate void;
extern crate libc;

#[cfg(feature = "ocf")]
mod ocf;
mod config;
mod namespace;
mod root;
mod mount;
mod link;
mod user;
mod device;
mod hostname;
mod capabilities;
mod command;
mod run;
pub mod into_cow;

use std::ffi::CString;

#[cfg(feature = "ocf")]
pub use ocf::{OcfError, ocf_config};
pub use config::{Config, ConfigChain, Configs};
pub use namespace::{Namespace, NamespaceKind};
pub use root::Root;
pub use mount::{Mount, default_mounts};
pub use link::{Symlink, default_symlinks};
pub use user::User;
pub use device::{Device, default_devices};
pub use hostname::Hostname;
pub use command::Command;
pub use capabilities::Capabilities;
pub use run::run;

pub type CStringArgs = nix::null_terminated::NullTerminatedVec<CString, &'static <CString as nix::null_terminated::CMapping>::Target>;
