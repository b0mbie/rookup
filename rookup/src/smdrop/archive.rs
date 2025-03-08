use flate2::read::GzDecoder;
use std::{
	fmt,
	io::{
		Cursor, Read, Error as IoError, Result as IoResult,
	},
	ops::Range,
	str::FromStr,
};
use tar::{
	Archive as TarArchive,
	Entries as TarEntries,
	Entry as TarEntry,
};
use zip::{
	result::ZipError,
	ZipArchive,
};

pub trait ArchiveBody {
	type Error;
	fn into_boxed_slice(self) -> Result<Box<[u8]>, Self::Error>;
	type Reader: Read;
	fn into_reader(self) -> Self::Reader;
}

impl<'a> ArchiveBody for ureq::BodyWithConfig<'a> {
	type Error = ureq::Error;
	#[inline]
	fn into_boxed_slice(self) -> Result<Box<[u8]>, Self::Error> {
		self.read_to_vec().map(move |v| v.into_boxed_slice())
	}
	type Reader = ureq::BodyReader<'a>;
	#[inline]
	fn into_reader(self) -> Self::Reader {
		self.reader()
	}
}

pub enum Archive<R: Read> {
	Zip(ZipArchive<Cursor<Box<[u8]>>>),
	TarGz(Box<TarArchive<GzDecoder<R>>>),
}

impl<R: Read> Archive<R> {
	pub fn new<B>(body: B, kind: ArchiveKind) -> Result<Self, ArchiveError<B::Error>>
	where
		B: ArchiveBody<Reader = R>,
	{
		match kind {
			ArchiveKind::Zip => match ZipArchive::new(Cursor::new(body.into_boxed_slice()?)) {
				Ok(archive) => Ok(Self::Zip(archive)),
				Err(error) => Err(match error {
					ZipError::Io(e) => ArchiveError::Io(e),
					ZipError::InvalidArchive(m) => ArchiveError::ZipInvalid(m),
					ZipError::UnsupportedArchive(m) => ArchiveError::ZipUnsupported(m),
					_ => unreachable!(),
				}),
			},
			ArchiveKind::TarGz => {
				let archive = TarArchive::new(GzDecoder::new(body.into_reader()));
				Ok(Self::TarGz(Box::new(archive)))
			}
		}
	}

	pub fn entries(&mut self) -> IoResult<Entries<'_, R>> {
		match self {
			Self::Zip(archive) => Ok(Entries::Zip {
				indices: 0..archive.len(),
				archive,
			}),
			Self::TarGz(archive) => Ok(Entries::TarGz {
				entries: archive.entries()?,
			}),
		}
	}
}

#[derive(Debug, thiserror::Error)]
pub enum ArchiveError<E> {
	#[error("{0}")]
	Io(IoError),
	#[error("{0}")]
	ZipInvalid(&'static str),
	#[error("{0}")]
	ZipUnsupported(&'static str),
	#[error("{0}")]
	IntoVec(#[from] E),
}

pub enum Entry<'a, R: 'a + Read> {
	Zip {
		cursor: Cursor<Vec<u8>>,
		is_dir: bool,
	},
	TarGz(Box<TarEntry<'a, GzDecoder<R>>>),
}

impl<'a, R: 'a + Read> Entry<'a, R> {
	// TODO: Remove this method?
	#[allow(dead_code)]
	pub fn size(&self) -> usize {
		match self {
			Self::Zip { cursor, .. } => cursor.get_ref().len(),
			Self::TarGz(i) => i.size() as _,
		}
	}

	pub fn is_dir(&self) -> bool {
		match self {
			Self::Zip { is_dir, .. } => *is_dir,
			Self::TarGz(i) => i.header().entry_type().is_dir(),
		}
	}
}

impl<'a, R: 'a + Read> Read for Entry<'a, R> {
	#[inline]
	fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
		match self {
			Self::Zip { cursor, .. } => cursor.read(buf),
			Self::TarGz(i) => i.read(buf),
		}
	}
}

pub enum Entries<'a, R: 'a + Read> {
	Zip {
		archive: &'a mut ZipArchive<Cursor<Box<[u8]>>>,
		indices: Range<usize>,
	},
	TarGz {
		entries: TarEntries<'a, GzDecoder<R>>,
	},
}

impl<'a, R: 'a + Read> Iterator for Entries<'a, R> {
	type Item = (Vec<u8>, Entry<'a, R>);
	fn next(&mut self) -> Option<Self::Item> {
		match self {
			Self::Zip { archive, indices } => {
				let index = indices.next()?;
				let mut file = archive.by_index(index).ok()?;
				let name = file.name().as_bytes().to_vec();
				let bytes = {
					let mut buffer = Vec::with_capacity(file.size() as _);
					file.read_to_end(&mut buffer).ok()?;
					buffer
				};
				Some((name, Entry::Zip {
					cursor: Cursor::new(bytes),
					is_dir: file.is_dir(),
				}))
			}
			Self::TarGz { entries } => {
				let entry = entries.next()?.ok()?;
				let name = entry.path_bytes().into_owned();
				Some((name, Entry::TarGz(Box::new(entry))))
			}
		}
	}
}

impl<R: Read + fmt::Debug> fmt::Debug for Archive<R> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Archive::Zip(a) => {
				f.debug_tuple("Archive::Zip")
					.field(a)
					.finish_non_exhaustive()
			}
			Archive::TarGz(..) => {
				struct TarArchiveDbg;
				impl fmt::Debug for TarArchiveDbg {
					fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
						f.debug_struct("TarArchive").finish_non_exhaustive()
					}
				}

				f.debug_tuple("Archive::TarGz")
					.field(&TarArchiveDbg)
					.finish_non_exhaustive()
			}
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ArchiveKind {
	Zip,
	TarGz,
}

impl FromStr for ArchiveKind {
	type Err = ArchiveKindErr;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		if s.ends_with(".zip") {
			Ok(Self::Zip)
		} else if s.ends_with(".tar.gz") {
			Ok(Self::TarGz)
		} else {
			Err(ArchiveKindErr::Unsupported)
		}
	}
}

#[derive(Debug, thiserror::Error)]
pub enum ArchiveKindErr {
	#[error("unsupported archive format")]
	Unsupported,
}
