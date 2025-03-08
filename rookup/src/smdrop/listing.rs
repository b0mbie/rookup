use quick_xml::events::{
	attributes::Attributes, Event
};

pub use quick_xml::{
	events::attributes::AttrError,
	escape::EscapeError,
	Reader as XmlReader, Error as XmlError,
};

// KLUDGE: `BytesStart`, for some unknown reason, never provides a way to get `Cow<'a, str>` from itself.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DirectoryItem {
	Directory(String),
	File(String),
}

impl From<String> for DirectoryItem {
	fn from(href: String) -> Self {
		if href.ends_with('/') {
			// href.pop().expect("`href` ends with a character that can be popped");
			DirectoryItem::Directory(href)
		} else {
			DirectoryItem::File(href)
		}
	}
}

impl From<&str> for DirectoryItem {
	fn from(href: &str) -> Self {
		Self::from(String::from(href))
	}
}

pub struct DirectoryItems<'a> {
	/// [`XmlReader`] that iterates over bytes, which are *always* valid UTF-8.
	reader: XmlReader<&'a [u8]>,
}

impl<'a> DirectoryItems<'a> {
	#[inline]
	pub const unsafe fn from_utf8_reader(reader: XmlReader<&'a [u8]>) -> Self {
		Self {
			reader,
		}
	}

	// TODO: Remove this?
	#[allow(dead_code)]
	pub fn from_str(s: &'a str) -> Self {
		unsafe { Self::from_utf8_reader(XmlReader::from_str(s)) }
	}

	#[cfg(debug_assertions)]
	#[inline]
	unsafe fn str_from_utf8_unchecked(b: &[u8]) -> &str {
		core::str::from_utf8(b).expect("`str_from_utf8_unchecked` failed")
	}

	#[cfg(not(debug_assertions))]
	#[inline]
	unsafe fn str_from_utf8_unchecked(b: &[u8]) -> &str {
		core::str::from_utf8_unchecked(b)
	}
}

impl Iterator for DirectoryItems<'_> {
	type Item = Result<DirectoryItem, DirectoryItemError>;
	fn next(&mut self) -> Option<Self::Item> {
		loop {
			let event = match self.reader.read_event() {
				Ok(e) => e,
				Err(e) => break Some(Err(e.into())),
			};
			match event {
				Event::Eof => break None,
				Event::Start(e) => {
					let tag_name = e.name();
					if tag_name.0 != b"a" { continue }

					let mut href = None;
					for result in Attributes::html(unsafe { Self::str_from_utf8_unchecked(e.attributes_raw()) }, 0) {
						let attr = match result {
							Ok(a) => a,
							Err(e) => return Some(Err(e.into())),
						};
						if attr.key.0 != b"href" { continue }
						match attr.unescape_value() {
							Ok(v) => {
								href = Some(v);
								break
							}
							Err(e) => return Some(Err(e.into())),
						}
					}

					let Some(href) = href else {
						continue
					};

					break Some(Ok(DirectoryItem::from(href.into_owned())))
				}
				_ => {}
			}
		}
	}
}

#[derive(Debug, thiserror::Error)]
pub enum DirectoryItemError {
	#[error("{0}")]
	Xml(#[from] XmlError),
	#[error("{0}")]
	Attr(#[from] AttrError),
	#[error("{0}")]
	Escape(#[from] EscapeError),
}

pub struct OwnedDirectoryItems {
	inner: DirectoryItems<'static>,
	owned_ptr: *mut u8,
	len_cap: usize,
}

impl OwnedDirectoryItems {
	pub fn new(s: String) -> Self {
		// SAFETY: `String` always contains valid UTF-8 bytes.
		unsafe { Self::from_utf8_unchecked(s.into_bytes()) }
	}

	pub unsafe fn from_utf8_unchecked(mut utf8_bytes: Vec<u8>) -> Self {
		utf8_bytes.shrink_to_fit();
		let owned = utf8_bytes.leak();
		let owned_ptr = owned.as_mut_ptr();
		let len_cap = owned.len();
		Self {
			inner: DirectoryItems::from_utf8_reader(XmlReader::from_reader(owned)),
			owned_ptr,
			len_cap,
		}
	}
}

impl Drop for OwnedDirectoryItems {
	fn drop(&mut self) {
		// SAFETY: We always construct `OwnedDirectoryItems` with an exclusive slice that we own.
		drop(unsafe { Vec::from_raw_parts(self.owned_ptr, self.len_cap, self.len_cap) });
	}
}

impl Iterator for OwnedDirectoryItems {
	type Item = Result<DirectoryItem, DirectoryItemError>;
	#[inline]
	fn next(&mut self) -> Option<Self::Item> {
		self.inner.next()
	}
}

#[test]
fn listing_works() {
	let listing_str = r#"
<!DOCTYPE HTML PUBLIC "-//W3C//DTD HTML 3.2 Final//EN">
<html>
 <head>
  <title>Index of /smdrop/1.12</title>
 </head>
 <body>
<h1>Index of /smdrop/1.12</h1>
<ul><li><a href="/smdrop/"> Parent Directory</a></li>
<li><a href="sourcemod-1.12.0-git7177-linux.tar.gz"> sourcemod-1.12.0-git7177-linux.tar.gz</a></li>
<li><a href="sourcemod-1.12.0-git7177-windows.zip"> sourcemod-1.12.0-git7177-windows.zip</a></li>
<li><a href="sourcemod-latest-linux"> sourcemod-latest-linux</a></li>
<li><a href="sourcemod-latest-windows"> sourcemod-latest-windows</a></li>
</ul>
<address>Apache/2.4.41 (Unix) OpenSSL/1.1.1n mod_fcgid/2.3.9 mod_wsgi/4.7.1 Python/2.7 mod_perl/2.0.11 Perl/v5.28.1 Server at sm.alliedmods.net Port 80</address>
</body></html>
"#;

	check_items({
		let result: Result<Vec<_>, _> = DirectoryItems::from_str(listing_str).collect();
		result.unwrap()
	});
	check_items({
		let result: Result<Vec<_>, _> = OwnedDirectoryItems::new(listing_str.into()).collect();
		result.unwrap()
	});
	check_items({
		let result: Result<Vec<_>, _> = OwnedDirectoryItems::new(listing_str.into()).collect();
		result.unwrap()
	});

	fn check_items(items: Vec<DirectoryItem>) {
		assert_eq!(items[0], DirectoryItem::Directory("/smdrop/".into()));
		assert_eq!(items[1], DirectoryItem::File("sourcemod-1.12.0-git7177-linux.tar.gz".into()));
		assert_eq!(
			items,
			vec![
				"/smdrop/".into(),
				"sourcemod-1.12.0-git7177-linux.tar.gz".into(), "sourcemod-1.12.0-git7177-windows.zip".into(),
				"sourcemod-latest-linux".into(), "sourcemod-latest-windows".into(),
			]
		);
	}
}
