#[cfg(target_pointer_width = "64")]
macro_rules! spcomp_exe_stem {
	() => { "spcomp64" };
}
#[cfg(target_pointer_width = "32")]
macro_rules! spcomp_exe_stem {
	() => { "spcomp" };
}
#[cfg(not(any(target_pointer_width = "64", target_pointer_width = "32")))]
macro_rules! spcomp_exe_stem {
	() => { compile_error!("can't determine file stem for `target_pointer_width`") };
}
pub(crate) use spcomp_exe_stem;

#[cfg(windows)]
macro_rules! spcomp_exe {
	() => { concat!(crate::spcomp_exe::spcomp_exe_stem!(), ".exe") };
}
#[cfg(not(windows))]
macro_rules! spcomp_exe {
	() => { crate::spcomp_exe::spcomp_exe_stem!() };
}
pub(crate) use spcomp_exe;
