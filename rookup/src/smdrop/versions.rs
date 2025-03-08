use rookup_common::version::Version as _;
use std::{
	borrow::Cow,
	convert::Infallible,
	fmt,
	ops::{
		Deref, DerefMut,
	},
};

use super::listing::{
	DirectoryItem, OwnedDirectoryItems
};

/// Version available on a [`Branch`](super::Branch) of a remote server.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
	url: VersionUrl<Box<str>>,
}

impl Version {
	/// Convert this version into the URL pointing to the archive with the toolchain.
	#[inline]
	pub fn into_url(self) -> VersionUrl<Box<str>> {
		self.url
	}
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct VersionUrl<S: AsRef<str>>(pub S);
impl<S: AsRef<str>> VersionUrl<S> {
	/// Return the name of the file pointed to by this URl.
	pub fn file_name(&self) -> &str {
		let url = self.0.as_ref();
		url.rsplit_once('/').map(after).unwrap_or(url)
	}

	/// Return the target platform of this URL.
	pub fn target(&self) -> Option<&str> {
		let target_suffix = self.file_name().rsplit_once('-').map(after)?;
		Some(
			target_suffix.split_once('.').map(before)
				.unwrap_or(target_suffix)
		)
	}
 
	/// Return the version string associated with this URL.
	pub fn version_str(&self) -> Option<VersionStr<&str>> {
		self.file_name().split_once('-').map(after)?
			.rsplit_once('-').map(before)
			.map(VersionStr)
	}
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct UrlVersionStr<S: AsRef<str>>(pub S);
impl<S: AsRef<str>> fmt::Display for UrlVersionStr<S> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut parts = self.0.as_ref().iter_parts().peekable();
		let Some(first) = parts.next() else {
			return Ok(())
		};
	
		f.write_str(first.0)?;
		while let Some(part) = parts.next() {
			if parts.peek().is_some() {
				f.write_str(".")?;
			} else {
				f.write_str("-git")?;
			}
			f.write_str(part.0)?;
		}
	
		Ok(())
	}
}

impl<S: AsRef<str>> Deref for VersionUrl<S> {
	type Target = S;
	#[inline]
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
impl<S: AsRef<str>> DerefMut for VersionUrl<S> {
	#[inline]
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

/// A sort-of-SemVer version string used in SourceMod.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct VersionStr<S: AsRef<str>>(pub S);
impl<S: AsRef<str>> VersionStr<S> {
	fn write_to<'a, W: WriteNormalized<'a>>(&'a self, w: W) -> Result<W::Result, W::Error> {
		if let Some((a, b)) = self.0.as_ref().split_once("-git") {
			w.write_split(a, b)
		} else {
			w.write_whole(self.0.as_ref())
		}
	}

	/// Return the "normalized" version string.
	/// 
	/// For SourceMod versions, this will replace `-git` with a `.` to make the git revision number part of SemVer.
	#[inline]
	pub fn normalized(&self) -> Cow<'_, str> {
		match self.write_to(StrWriter) {
			Ok(s) => s,
			Err(..) => unreachable!(),
		}
	}
}

impl<S: AsRef<str>> fmt::Display for VersionStr<S> {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.write_to(FmtWriter(f))
	}
}

impl<S: AsRef<str>> Deref for VersionStr<S> {
	type Target = S;
	#[inline]
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<S: AsRef<str>> DerefMut for VersionStr<S> {
	#[inline]
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

/// Iterator over [`Version`]s available on a remote server.
pub struct Versions {
	pub(crate) inner: OwnedDirectoryItems,
	pub(crate) root: String,
}
impl Iterator for Versions {
	type Item = Version;
	fn next(&mut self) -> Option<Self::Item> {
		loop {
			let item = self.inner.next()?.ok()?;

			if let DirectoryItem::File(mut file_name) = item {
				file_name.insert_str(0, &self.root);
				let version = Version {
					url: VersionUrl(file_name.into_boxed_str()),
				};
				break Some(version)
			}
		}
	}
}

/// Helper trait for writing out normalized version strings without extra allocations.
trait WriteNormalized<'a>: Sized {
	type Error;
	type Result;
	fn write_whole(self, s: &'a str) -> Result<Self::Result, Self::Error>;
	fn write_split(self, a: &'a str, b: &'a str) -> Result<Self::Result, Self::Error>;
}

struct StrWriter;
impl<'a> WriteNormalized<'a> for StrWriter {
	type Error = Infallible;
	type Result = Cow<'a, str>;
	#[inline]
	fn write_whole(self, s: &'a str) -> Result<Self::Result, Self::Error> {
		Ok(Cow::Borrowed(s))
	}
	#[inline]
	fn write_split(self, a: &'a str, b: &'a str) -> Result<Self::Result, Self::Error> {
		Ok(Cow::Owned(format!("{a}.{b}")))
	}
}

struct FmtWriter<'a, 'f>(pub &'a mut fmt::Formatter<'f>);
impl<'a> WriteNormalized<'a> for FmtWriter<'_, '_> {
	type Error = fmt::Error;
	type Result = ();
	#[inline]
	fn write_whole(self, s: &'a str) -> Result<Self::Result, Self::Error> {
		self.0.write_str(s)
	}
	fn write_split(self, a: &'a str, b: &'a str) -> Result<Self::Result, Self::Error> {
		self.0.write_str(a)?;
		self.0.write_str(".")?;
		self.0.write_str(b)
	}
}

#[inline]
const fn before<'a>(p: (&'a str, &'a str)) -> &'a str {
	p.0
}

#[inline]
const fn after<'a>(p: (&'a str, &'a str)) -> &'a str {
	p.1
}
