use rookup_common_build::{
	anyhow::{
		anyhow, Result as AResult,
	},
	create_default_config,
};
use std::{
	env::var_os,
	fs::File,
	io::Write,
	path::PathBuf,
};

fn main() -> AResult<()> {
	println!("cargo::rerun-if-changed=rookup-common-base/src/config.rs");

	let out_dir = PathBuf::from(var_os("OUT_DIR").expect("`OUT_DIR` should be set for the build script"));

	// Create default config.
	{
		let config_path = out_dir.join("config.toml");
		let config_toml = create_default_config()?;
		let mut config_file = File::options()
			.create(true).truncate(true)
			.write(true)
			.open(&config_path)
			.map_err(|e| anyhow!("Couldn't open default config file at {config_path:?}: {e}"))?;
		write!(config_file, "{config_toml}")?;
	}

	Ok(())
}
