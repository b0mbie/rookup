use std::{
	env::{
		var, VarError,
	},
	fs::{
		File, create_dir_all,
	},
	io::{
		Write, Seek, Result as IoResult,
	},
	path::Path,
};

pub use rookup_common_base::*;

/// Return the name and source (as [`ToolchainSource`]) of the current toolchain.
pub fn current_toolchain(data: &ConfigData) -> Result<(String, ToolchainSource), CurrentToolchainError> {
	match var("ROOKUP_TOOLCHAIN") {
		Ok(toolchain) => {
			return Ok((toolchain, ToolchainSource::Env))
		}
		Err(VarError::NotPresent) => {}
		Err(VarError::NotUnicode(..)) => return Err(CurrentToolchainError::ToString),
	}

	Ok((data.default.clone(), ToolchainSource::Config))
}

pub trait ConfigExt: Sized {
	/// Open the configuration file at its default path.
	fn open_default(with_write: bool) -> Result<Self, ConfigError>;

	/// Open the configuration file at its default path, writing a file with default values if necessary.
	fn open_create(with_write: bool) -> Result<Self, ConfigError>;
}
impl ConfigExt for Config {
	fn open_default(with_write: bool) -> Result<Self, ConfigError> {
		let Some(config_home) = config_home() else {
			return Err(ConfigError::ConfigPath)
		};
		Self::open(config_file_path(config_home.clone()), with_write)
	}

	fn open_create(with_write: bool) -> Result<Self, ConfigError> {
		let Some(config_home) = config_home() else {
			return Err(ConfigError::ConfigPath)
		};
		
		let config_path = config_file_path(config_home.clone());
		let file = if !config_path.exists() {
			create_dir_all(&config_home)
				.map_err(|error| ConfigError::ConfigCreateHome {
					error,
					config_home: config_home.clone(),
				})?;

			fn create_default_config(config_path: &Path) -> IoResult<File> {
				let mut file = File::options()
					.create(true).truncate(true)
					.write(true)
					.read(true)
					.open(config_path)?;
				file.write_all(include_bytes!(concat!(env!("OUT_DIR"), "/config.toml")))?;
				file.flush()?;
				file.rewind()?;
				Ok(file)
			}

			create_default_config(&config_path)
				.map_err(|error| ConfigError::ConfigCreateDefault {
					error,
					config_path: config_path.clone(),
				})?
		} else {
			File::options()
				.read(true).write(with_write)
				.open(&config_path)
				.map_err(|error| ConfigError::ConfigOpen {
					error,
					config_path: config_path.clone(),
				})?
		};

		Self::with_file(file, config_path)
	}
}

/// Enumeration of sources that specify the current toolchain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ToolchainSource {
	/// Current toolchain is specified by an environment variable.
	Env,
	/// Current toolchain is specified by the configuration file.
	Config,
}

/// Error that occurred in [`current_toolchain`].
#[derive(Debug, thiserror::Error)]
pub enum CurrentToolchainError {
	#[error("{0}")]
	Config(#[from] ConfigError),
	#[error("toolchain string does not contain valid UTF-8")]
	ToString,
}
