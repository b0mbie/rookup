//! Definitions for Rookup toolchains.

use std::{
	env::var_os,
	ffi::{
		OsStr, OsString,
	},
	fmt::{
		self, Write,
	},
	fs::{
		read_dir, ReadDir,
	},
	io::Result as IoResult,
	ops::Deref,
	path::PathBuf,
};

use crate::{
	config::{
		ConfigError, ConfigData,
	},
	version::{
		Version, version_ord,
	},
	home, toolchain_home_path,
};

/// Path to the global includes directory.
pub const INCLUDES_PATH: &str = "includes";

/// Return the path to the toolchain directory, or [`None`] if it couldn't be determined.
pub fn toolchain_home() -> Option<PathBuf> {
	var_os("ROOKUP_TOOLCHAIN_HOME").map(PathBuf::from)
		.or_else(dirs::cache_dir)
		.map(home)
		.map(toolchain_home_path)
}

/// Return the path to the custom toolchain directory, or [`None`] if it couldn't be determined.
pub fn custom_toolchain_home() -> Option<PathBuf> {
	var_os("ROOKUP_CUSTOM_TOOLCHAIN_HOME").map(PathBuf::from)
		.or_else(dirs::data_dir)
		.map(home)
		.map(toolchain_home_path)
}

macro_rules! res_unwrap_or_return {
	($expr:expr) => {
		match $expr {
			Ok(v) => v,
			Err(e) => return Some(Err(e)),
		}
	};
}

/// Parsed toolchain selector of the format `':' super_version | alias`.
// TODO: Documentation for this should be public!
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Selector<'a> {
	Super(&'a str),
	Alias(&'a str),
}

impl<'a> Selector<'a> {
	pub const SUPER_PREFIX: char = ':';

	pub fn parse(s: &'a str) -> Self {
		s.strip_prefix(Self::SUPER_PREFIX)
			.map(Self::Super)
			.unwrap_or(Self::Alias(s))
	}

	pub fn test(&self, data: &ConfigData, version: &str) -> bool {
		match self {
			Self::Alias(name) => {
				data.aliases.get(*name).is_some_and(move |a| a == version)
			}
			Self::Super(super_version) => version.is_sub_version_of(super_version),
		}
	}

	pub const fn is_alias(&self) -> bool {
		matches!(self, Self::Alias(..))
	}

	pub const fn to_alias(self) -> Option<&'a str> {
		match self {
			Self::Alias(s) => Some(s),
			_ => None,
		}
	}
}

impl Deref for Selector<'_> {
	type Target = str;
	#[inline]
	fn deref(&self) -> &Self::Target {
		match self {
			Self::Super(s) => s,
			Self::Alias(s) => s,
		}
	}
}

impl fmt::Display for Selector<'_> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Super(s) => {
				f.write_char(Self::SUPER_PREFIX)?;
				f.write_str(s)
			}
			Self::Alias(s) => f.write_str(s),
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FoundToolchain {
	pub name: String,
	pub kinded: FoundToolchainKinded,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FoundToolchainKinded {
	Latest {
		home: PathBuf,
	},
	Aliased {
		path: PathBuf,
	},
}

impl FoundToolchain {
	pub fn into_path(self) -> PathBuf {
		match self.kinded {
			FoundToolchainKinded::Latest { mut home } => {
				home.push(self.name);
				home
			}
			FoundToolchainKinded::Aliased { path } => path,
		}
	}
}

/// Search for a toolchain using `selector`, given `config`.
pub fn find_toolchain(config: &ConfigData, selector: Selector<'_>) -> Result<FoundToolchain, FindToolchainError> {
	match selector {
		Selector::Super(s) => {
			let (name, home) = find_latest_toolchain_of(s)
				.ok_or_else(move || FindToolchainError::LatestNotFound(s.to_string()))?;
			Ok(FoundToolchain {
				name,
				kinded: FoundToolchainKinded::Latest { home },
			})
		}
		Selector::Alias(s) => {
			let version = config.aliases.get(s)
				.ok_or_else(move || FindToolchainError::NoAliasDefault(s.to_string()))?;
			let path = find_toolchain_path(OsStr::new(version))
				.ok_or_else(move || FindToolchainError::NotFound {
					version: version.to_string(),
					alias: s.to_string(),
				})?;
			Ok(FoundToolchain {
				name: version.clone(),
				kinded: FoundToolchainKinded::Aliased { path },
			})
		}
	}
}

/// Error that occurred in [`find_toolchain`].
#[derive(Debug, thiserror::Error)]
pub enum FindToolchainError {
	#[error("latest toolchain compatible with version {0} was not found")]
	LatestNotFound(String),
	#[error("version {version} (as specified by alias {alias:?}) was not found")]
	NotFound {
		version: String,
		alias: String,
	},
	#[error("{0}")]
	Config(#[from] ConfigError),
	#[error("alias {0:?} has no default version set")]
	NoAliasDefault(String),
}

/// Return `true` if a toolchain of `version` is installed.
pub fn is_installed(version: &OsStr) -> bool {
	ToolchainHomes::new().any(move |home| home.join(version).exists())
}

/// Find the location of an installed toolchain of the specified `version`.
pub fn  find_toolchain_path(version: &OsStr) -> Option<PathBuf> {
	ToolchainHomes::new().find_map(move |home| {
		let path = home.join(version);
		path.exists().then_some(path)
	})
}

/// Find the location of an installed toolchain of the specified `super_version` (e.g. `1.12`).
pub fn find_latest_toolchain_of(super_version: &str) -> Option<(String, PathBuf)> {
	ToolchainVersions::new()
		.flat_map(move |(home, result)| result.map(move |names| (home, names)))
		.find_map(move |(home, names)| {
			names.flatten()
				.map(move |name| name.to_string_lossy().into_owned())
				.filter(move |name| name.as_str().is_sub_version_of(super_version))
				.max_by(version_ord)
				.map(move |name| (name, home))
		})
}

/// Iterator over installed toolchain locations and iterators over toolchains installed in those locations.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ToolchainVersions {
	homes: ToolchainHomes,
}

impl ToolchainVersions {
	pub const fn new() -> Self {
		Self {
			homes: ToolchainHomes::new(),
		}
	}
}

impl Iterator for ToolchainVersions {
	type Item = (PathBuf, IoResult<DirNames>);
	fn next(&mut self) -> Option<Self::Item> {
		let home = self.homes.next()?;
		let dirs = read_dir(&home).map(DirNames);
		Some((home, dirs))
	}
}

/// Iterator over directories located inside of another directory.
#[derive(Debug)]
#[repr(transparent)]
pub struct DirNames(pub ReadDir);
impl Iterator for DirNames {
	type Item = IoResult<OsString>;
	fn next(&mut self) -> Option<Self::Item> {
		loop {
			match self.0.next() {
				Some(Ok(entry)) => {
					let file_type = res_unwrap_or_return!(entry.file_type());
					if file_type.is_dir() {
						break Some(Ok(entry.file_name()))
					}
				}
				Some(Err(e)) => break Some(Err(e)),
				None => break None,
			}
		}
	}
}

/// Iterator over possible locations for installed toolchains.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ToolchainHomes {
	#[default]
	Custom,
	Cached,
	Done,
}

impl ToolchainHomes {
	#[inline]
	pub const fn new() -> Self {
		Self::Custom
	}
}

impl Iterator for ToolchainHomes {
	type Item = PathBuf;
	fn next(&mut self) -> Option<Self::Item> {
		match self {
			Self::Custom => {
				*self = Self::Cached;
				custom_toolchain_home()
			}
			Self::Cached => {
				*self = Self::Done;
				toolchain_home()
			}
			Self::Done => None,
		}
	}
}
