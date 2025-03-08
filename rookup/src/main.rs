use anyhow::{
	anyhow, bail,
	Result as AResult,
};
use clap::{
	Parser, Subcommand,
};
use rookup_common::{
	version::{
		Version, version_ord,
	},
	find_latest_toolchain_of, is_installed, toolchain_home,
	Config, ConfigExt,
	ToolchainVersions, Selector,
};
use std::{
	ffi::OsStr,
	fs::{
		File, create_dir_all,
	},
	io::{
		copy as io_copy,
		ErrorKind as IoErrorKind,
	},
	path::PathBuf,
	process::ExitCode,
	str::FromStr,
};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use ureq::Agent;

mod smdrop;
mod smdrop_util;
use smdrop_util::*;
mod sp_from_sm;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
	#[command(subcommand)]
	pub command: Command,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
	/// Show current configuration data.
	Config,
	/// Get or set the default version selector.
	Default {
		/// If set, then this string will be the new default version selector.
		default: Option<String>,
	},
	/// Get or set an alias.
	Alias {
		alias: String,
		version: Option<String>,
	},
	/// Show a list of installed toolchains.
	Show,
	/// Fetch the latest version of SourcePawn, download it if needed, and default to it.
	Update {
		selector: Option<String>,
		/// Re-download the toolchain, regardless of whether it is already installed or not.
		#[arg(long)]
		redownload: bool,
		/// Set this alias to the version that was installed.
		/// 
		/// If not specified, then the selector string is used as the alias.
		#[arg(long)]
		alias: Option<String>,
	},
	/// Install a specific SourcePawn toolchain.
	Install {
		selector: String,
		/// Re-download the toolchain, regardless of whether it is already installed or not.
		#[arg(long)]
		redownload: bool,
	},
}

fn real_main() -> AResult<()> {
	let cli = Cli::parse();
	match cli.command {
		Command::Config => {
			let config = Config::open_default(false)?;
			println!("@{}", config.path.display());
			println!("{:#?}", config.with_doc.data());
		}

		Command::Default { default: new_default } => {
			if let Some(new_default) = new_default {
				let mut config = Config::open_default(true)?;
				let old_default = &config.with_doc.data().default;
				println!("{old_default} => {new_default}");
				if old_default != &new_default {
					config.with_doc.set_default(new_default);
					config.rewrite()?;
				}
			} else {
				println!("{}", Config::open_default(false)?.with_doc.data().default);
			}
		}

		Command::Alias { alias, version: value } => {
			let mut config = Config::open_default(true)?;
			if let Some(version) = value {
				config.with_doc.set_alias(alias, version);
				config.rewrite()?;
			} else if let Some(version) = config.with_doc.data().aliases.get(&alias) {
				println!("={version}");
			}
		}

		Command::Show => {
			for (home, version_names) in ToolchainVersions::new() {
				println!("{}:", home.display());
				let version_names = match version_names {
					Ok(i) => i,
					Err(e) if e.kind() == IoErrorKind::NotFound => {
						continue
					}
					Err(e) => bail!("couldn't read {}: {e}", home.display())
				};
				for result in version_names {
					let version_name = result.map_err(|e| {
						anyhow!("encountered error while iterating over {}: {e}", home.display())
					})?;
					println!("  {} => {}", version_name.to_string_lossy(), home.join(&version_name).to_string_lossy());
				}
			}
		}

		Command::Update { selector, redownload, alias } => {
			let mut config = Config::open_default(true)?;

			let selector = unwrap_selector(selector, &config);
			let parsed_selector = Selector::parse(&selector);

			let client = smdrop_client(&config);
			let branch = client.select_branch(parsed_selector)?;
			println!("Remote branch: {}", branch.name());

			let remote = branch.relevant_urls(&client)?
				.max_by(RelevantUrl::version_ord)
				.ok_or_else(|| anyhow!("received no versions for branch {:?}", branch.name()))?;

			let remote_ver = remote.version();
			println!("Remote version: {remote_ver}");

			let remote_url = remote.url();
			println!("Remote URL: {remote_url}");

			let installed_ver = find_latest_toolchain_of(branch.name()).map(move |(v, ..)| v);
			if let Some(latest_installed_ver) = installed_ver.as_ref() {
				println!("Installed version: {latest_installed_ver}");
			}

			let upgrading = installed_ver
				.is_none_or(|v| version_ord(v.as_str(), remote_ver).is_lt());
			println!("Is upgrade: {}", bool_display(upgrading));

			let needs_download = redownload || (upgrading && !is_installed(OsStr::new(remote_ver)));
			println!("Needs download: {}", bool_display(needs_download));
			if needs_download {
				let destination = toolchain_destination(remote_ver)?;
				println!("Destination: {}", destination.display());

				InstallVersion {
					agent: &client.agent,
					url: remote_url,
					max_bytes: config.with_doc.data().source.max_download_size,
					destination,
				}.call()?;
			}

			config.with_doc.set_alias(alias.as_deref().unwrap_or(selector.as_str()), remote_ver);
			config.rewrite().map_err(move |e| anyhow!("failed to write changes to config: {e}"))?;
		}
	
		Command::Install { selector, redownload } => {
			let mut config = Config::open_default(true)?;

			let parsed_selector = Selector::parse(&selector);

			let client = smdrop_client(&config);
			let branch = client.select_branch(parsed_selector)?;
			println!("Remote branch: {}", branch.name());

			let versions = branch.relevant_urls(&client)?;
			let version = match parsed_selector {
				Selector::Alias(..) => {
					versions.max_by(RelevantUrl::version_ord)
						.ok_or_else(move || anyhow!("received no versions for branch {:?}", branch.name()))?
				}
				Selector::Super(requested) => {
					versions.filter(move |v| v.version().is_sub_version_of(requested))
						.max_by(RelevantUrl::version_ord)
						.ok_or_else(move || anyhow!("couldn't find version {requested:?} in branch {:?}", branch.name()))?
				}
			};

			let remote_ver = version.version();
			println!("Remote version: {remote_ver}");

			let remote_url = version.url();
			println!("Remote URL: {remote_url}");

			let needs_download = redownload || !is_installed(OsStr::new(remote_ver));
			println!("Needs download: {}", bool_display(needs_download));

			if needs_download {
				let destination = toolchain_destination(remote_ver)?;
				println!("Destination: {}", destination.display());

				InstallVersion {
					agent: &client.agent,
					url: remote_url,
					max_bytes: config.with_doc.data().source.max_download_size,
					destination,
				}.call()?;

				if let Selector::Alias(alias) = parsed_selector {
					config.with_doc.set_alias(alias, remote_ver);
				}
			}
		}
	}

	const fn bool_display(b: bool) -> &'static str {
		if b { "Yes" } else { "No" }
	}

	Ok(())
}

fn toolchain_destination<P: AsRef<std::path::Path>>(version: P) -> AResult<PathBuf> {
	let mut buffer = toolchain_home()
		.ok_or_else(move || anyhow!("couldn't get toolchain destination directory"))?;
	buffer.push(version);
	Ok(buffer)
}

fn unwrap_selector(selector: Option<String>, config: &Config) -> String {
	selector.unwrap_or_else(move || config.with_doc.data().default.clone())
}

struct InstallVersion<'a> {
	pub agent: &'a Agent,
	pub url: &'a str,
	pub max_bytes: u64,
	pub destination: PathBuf,
}

impl InstallVersion<'_> {
	pub fn call(self) -> AResult<()> {
		let body = self.agent.get(self.url)
			.call().map_err(move |e| anyhow!("failed to fetch archive: {e}"))?
			.into_body().into_with_config()
			.limit(self.max_bytes);

		let archive_kind = smdrop::ArchiveKind::from_str(self.url)
			.map_err(move |e| anyhow!("failed to determine archive format: {e}"))?;
		let mut archive = smdrop::Archive::new(body, archive_kind)?;
	
		for (path, mut entry) in archive.entries()?
			.filter_map(move |(name, entry)| String::from_utf8(name).ok().map(move |path| (path, entry)))
			.filter_map(move |(name, entry)| sp_from_sm::map_to_sp_root(name).map(move |path| (path, entry)))
			.filter(move |(path, ..)| sp_from_sm::is_sp_file(path))
		{
			let destination_path = self.destination.join(&path);
			if !entry.is_dir() {
				if let Some(parent) = destination_path.parent() {
					create_dir_all(parent)
						.map_err(|e| anyhow!("failed to create directories up to {destination_path:?}: {e}"))?;
				}

				let mut options = File::options();
				#[cfg(unix)]
				if path.file_name().and_then(move |n| n.to_str()).is_some_and(rookup_common::is_compiler) {
					options.mode(0o777);
				}

				let mut file = options.create(true).truncate(true).write(true).open(&destination_path)
					.map_err(|e| anyhow!("failed to open {destination_path:?}: {e}"))?;
				eprintln!("{} => {}", path.display(), destination_path.display());

				io_copy(&mut entry, &mut file)
					.map_err(|e| anyhow!("failed to pipe data of {path:?} to {destination_path:?}: {e}"))?;
			}
		}
	
		Ok(())
	}
}

fn main() -> ExitCode {
	match real_main() {
		Ok(..) => ExitCode::SUCCESS,
		Err(e) => {
			eprintln!("Fatal error: {e}");
			ExitCode::FAILURE
		}
	}
}
