use anyhow::{
	Result as AResult,
	anyhow,
};
use rookup_common::{
	current_toolchain, find_toolchain,
	Config, ConfigExt,
	ToolchainSource, Selector, FindToolchainError,
	SPCOMP_EXE,
};
use std::{
	env::args_os,
	error::Error,
	ffi::OsString,
	fmt,
	process::{
		exit, Command, ExitCode, Stdio,
	}
};

fn main() -> ExitCode {
	let mut args = args_os();
	let exe = args.next();
	match spcomp_main(args) {
		Ok(Some(code)) => exit(code),
		Ok(None) => {}
		Err(e) => {
			if let Some(exe) = exe.as_ref().and_then(move |s| s.to_str()) {
				eprint!("{exe}: ");
			}
			eprintln!("{e}");
		}
	}
	ExitCode::FAILURE
}

#[derive(Debug)]
enum NotFoundBailKind {
	LatestCompatibleWith {
		version: String,
	},
	Aliased {
		version: String,
		alias: String,
	},
}

fn spcomp_main(args: impl Iterator<Item = OsString>) -> AResult<Option<i32>> {
	let data = Config::open_default(false)?.with_doc.into();
	let (toolchain, source) = current_toolchain(&data)
		.map_err(move |e| anyhow!("failed to get current toolchain: {e}"))?;

	let parsed = Selector::parse(&toolchain);
	let toolchain_path = match find_toolchain(&data, parsed) {
		Ok(p) => p,
		Err(FindToolchainError::LatestNotFound(version)) => {
			return Err(NotFoundBail {
				source,
				kind: NotFoundBailKind::LatestCompatibleWith { version }
			}.into())
		}
		Err(FindToolchainError::NotFound { version, alias }) => {
			return Err(NotFoundBail {
				source,
				kind: NotFoundBailKind::Aliased { version, alias }
			}.into())
		}
		Err(e) => return Err(e.into()),
	};

	let spcomp_path = {
		let mut buffer = toolchain_path;
		buffer.push(SPCOMP_EXE);
		buffer
	};

	let mut spcomp = Command::new(&spcomp_path)
		.stdin(Stdio::inherit())
		.stdout(Stdio::inherit()).stderr(Stdio::inherit())
		.args(args)
		.spawn()
		.map_err(move |e| anyhow!("{}: {e}", spcomp_path.display()))?;
	let status = spcomp.wait()?;
	Ok(status.code())
}

#[derive(Debug)]
struct NotFoundBail {
	pub source: ToolchainSource,
	pub kind: NotFoundBailKind,
}
impl Error for NotFoundBail {}
impl fmt::Display for NotFoundBail {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self.source {
			ToolchainSource::Env => "the `ROOKUP_TOOLCHAIN` environment variable",
			ToolchainSource::Config => "the Rookup configuration file",
		})?;
		f.write_str(" specifies that a toolchain of ")?;
		match &self.kind {
			NotFoundBailKind::LatestCompatibleWith { version } => {
				write!(f, "the latest version compatible with {version:?}")?;
			}
			NotFoundBailKind::Aliased { version, alias } => {
				write!(f, "version {version:?} (as specified by alias {alias:?})")?;
			}
		}
		f.write_str(" should be used, but that toolchain is not installed")
	}
}
