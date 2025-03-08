use rustc_hash::FxHashMap;
use serde::Deserialize;
use std::{
	fs::File,
	io::{
		Error as IoError, Result as IoResult,
		Read, Write, Seek,
	},
	path::PathBuf,
};
use toml_edit::{
	de::from_document,
	DocumentMut, TomlError,
};

/// Configuration for the main Rookup CLI and Rookup proxies.
// TODO: Documentation for this should be public!
#[derive(documented::Documented, documented::DocumentedFields, field_access::FieldAccess, serde::Serialize)]
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct ConfigData {
	/// Selector for the toolchain to use by default when invoking Rookup proxies.
	pub default: String,
	/// Map of aliases to their associated version.
	pub aliases: FxHashMap<String, String>,
	/// See [`Source`].
	pub source: Source,
}

impl Default for ConfigData {
	fn default() -> Self {
		Self {
			default: "stable".into(),
			aliases: FxHashMap::default(),
			source: Source::default(),
		}
	}
}

/// Configuration for downloading SourcePawn toolchains from an external server.
// TODO: Documentation for this should be public!
#[derive(documented::Documented, documented::DocumentedFields, field_access::FieldAccess, serde::Serialize)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "kebab-case")]
pub struct Source {
	/// Root URL for a static file server to fetch SourceMod (with SourcePawn packaged) from.
	pub root_url: String,
	/// Maximum size, in bytes, that is allowed to be downloaded from the server.
	pub max_download_size: u64,
}

impl Default for Source {
	fn default() -> Self {
		Self {
			root_url: "https://sm.alliedmods.net/smdrop/".into(),
			max_download_size: 75_000_000,
		}
	}
}

/// Structure that holds the configuration file along with its path and structured data.
#[derive(Debug)]
pub struct Config {
	pub path: PathBuf,
	pub file: File,
	pub with_doc: ConfigDoc,
}

/// Error that occurred while opening a [`Config`].
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
	#[error("couldn't get config path")]
	ConfigPath,
	#[error("failed to open {config_path}: {error}")]
	ConfigOpen {
		error: IoError,
		config_path: PathBuf,
	},
	#[error("failed to create config home directory at {config_home}: {error}")]
	ConfigCreateHome {
		error: IoError,
		config_home: PathBuf,
	},
	#[error("failed to create default config at {config_path}: {error}")]
	ConfigCreateDefault {
		error: IoError,
		config_path: PathBuf,
	},
	#[error("{config_path}: {error}")]
	ConfigIo {
		error: IoError,
		file: File,
		config_path: PathBuf,
	},
	#[error("failed to parse {config_path}: {error}")]
	ConfigParse {
		error: Box<TomlError>,
		file: File,
		config_path: PathBuf,
	},
}

macro_rules! handle_err {
	($expr:expr; $error:ident => $err:expr) => {
		match $expr {
			Ok(v) => v,
			Err($error) => return Err($err),
		}
	};
}

impl Config {
	pub fn with_file(mut file: File, config_path: PathBuf) -> Result<Self, ConfigError> {
		let text = {
			let mut buffer = String::new();
			handle_err!(
				file.read_to_string(&mut buffer);
				error => ConfigError::ConfigIo {
					error,
					file,
					config_path,
				}
			);
			buffer
		};
		let config = handle_err!(
			text.parse::<DocumentMut>().and_then(ConfigDoc::from_document);
			error => ConfigError::ConfigParse {
				error: Box::new(error),
				file,
				config_path,
			}
		);
		Ok(Config {
			path: config_path,
			file,
			with_doc: config,
		})
	}

	pub fn open(config_path: PathBuf, write: bool) -> Result<Self, ConfigError> {
		let file = handle_err!(
			File::options().read(true).write(write).open(&config_path);
			error => ConfigError::ConfigOpen {
				error,
				config_path,
			}
		);
		Self::with_file(file, config_path)
	}

	pub fn rewrite(&mut self) -> IoResult<String> {
		let data = self.with_doc.document().to_string();
		self.file.rewind()?;
		self.file.write_all(data.as_bytes())?;
		self.file.set_len(data.len() as _)?;
		Ok(data)
	}
}

/// Main container for configuration data that holds both the formatted TOML document and the structured in-memory
/// representation.
#[derive(Debug, Clone)]
pub struct ConfigDoc {
	document: DocumentMut,
	data: ConfigData,
}

impl From<ConfigDoc> for ConfigData {
	#[inline]
	fn from(value: ConfigDoc) -> Self {
		value.data
	}
}

impl ConfigDoc {
	pub fn from_document(document: DocumentMut) -> Result<Self, TomlError> {
		// FIXME: This shouldn't copy the entire document!
		let data = from_document(document.clone())?;
		Ok(Self {
			document,
			data,
		})
	}
	
	#[inline]
	pub const fn document(&self) -> &DocumentMut {
		&self.document
	}

	#[inline]
	pub const fn data(&self) -> &ConfigData {
		&self.data
	}

	pub fn set_default(&mut self, default: impl Clone + Into<String>) {
		self.document["default"] = default.clone().into().into();
		self.data.default = default.into();
	}

	pub fn set_alias(&mut self, alias: impl AsRef<str> + Into<String>, version: impl Clone + Into<String>) {
		self.document["aliases"][alias.as_ref()] = version.clone().into().into();
		self.data.aliases.insert(alias.into(), version.into());
	}
}
