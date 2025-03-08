use std::{
	env::var_os,
	path::PathBuf,
};

pub use documented;
pub use field_access;
pub use toml_edit;

mod config;
pub use config::*;
mod toolchain;
pub use toolchain::*;
pub mod version;

mod spcomp_exe;

const HOME_DIR: &str = "rookup";

/// Consume a parent directory and return the home directory for Rookup.
fn home(mut parent_dir: PathBuf) -> PathBuf {
	parent_dir.push(HOME_DIR);
	parent_dir
}

/// Consume the config home directory and return the path to the config file.
fn config_file_path(mut config_home: PathBuf) -> PathBuf {
	config_home.push("config.toml");
	config_home
}

/// Consume the (either cache or data) home directory and return the path to the toolchain directory.
/// 
/// The toolchains are stored in cache because they are intended to be easily re-created if lost by re-downloading the
/// toolchain from a mirror.
fn toolchain_home_path(mut home: PathBuf) -> PathBuf {
	home.push("toolchains");
	home
}

/// Return the path to the configuration file, or [`None`] if it couldn't be determined.
pub fn config_path() -> Option<PathBuf> {
	var_os("ROOKUP_CONFIG_HOME").map(PathBuf::from)
		.or_else(dirs::config_dir)
		.map(home)
		.map(config_file_path)
}

/// File name of the compiler executable that is to be used by this target.
pub const SPCOMP_EXE: &str = spcomp_exe::spcomp_exe!();

/// Return `true` if `file_name` is the appropriate compiler executable for this target.
#[inline]
pub fn is_compiler(file_name: &str) -> bool {
	file_name == SPCOMP_EXE
}
