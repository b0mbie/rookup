use std::{
	env::{
		var, VarError,
	},
	fs::write,
};

pub use rookup_common_base::*;

/// Return the name and source (as [`ToolchainSource`]) of the current toolchain.
pub fn current_toolchain() -> Result<(String, ToolchainSource), CurrentToolchainError> {
	match var("ROOKUP_TOOLCHAIN") {
		Ok(toolchain) => {
			return Ok((toolchain, ToolchainSource::Env))
		}
		Err(VarError::NotPresent) => {}
		Err(VarError::NotUnicode(..)) => return Err(CurrentToolchainError::ToString),
	}

	let config = Config::open_default(false)?;
	let data: ConfigData = config.with_doc.into();
	Ok((data.default.to_string(), ToolchainSource::Config))
}

macro_rules! handle_err {
	($expr:expr; $error:ident => $err:expr) => {
		match $expr {
			Ok(v) => v,
			Err($error) => return Err($err),
		}
	};
}

pub trait ConfigExt: Sized {
	fn open_default(with_write: bool) -> Result<Self, ConfigError>;
}
impl ConfigExt for Config {
	fn open_default(with_write: bool) -> Result<Self, ConfigError> {
		let Some(config_path) = crate::config_path() else {
			return Err(ConfigError::ConfigPath)
		};
		
		if !config_path.exists() {
			handle_err!(
				write(&config_path, include_bytes!(concat!(env!("OUT_DIR"), "/config.toml")));
				error => ConfigError::ConfigCreateDefault {
					error,
					config_path,
				}
			);
		}

		Self::open(config_path, with_write)
	}
}
