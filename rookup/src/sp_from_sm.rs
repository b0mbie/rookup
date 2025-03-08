use clean_path::clean;
use std::path::{
	Path, PathBuf,
};

pub const SM_SP_ROOT: &str = "addons/sourcemod/scripting/";

pub fn map_to_sp_root(mut name: String) -> Option<PathBuf> {
	if !name.starts_with(SM_SP_ROOT) {
		return None
	}
	name.drain(..SM_SP_ROOT.len());
	(!name.is_empty()).then(move || clean(name))
}

pub fn is_sp_file(path: &Path) -> bool {
	if path.starts_with("include") {
		true
	} else {
		let file_name = path.file_name().and_then(move |n| n.to_str());
		file_name.is_some_and(rookup_common::is_compiler)
	}
}
