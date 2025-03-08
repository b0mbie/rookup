use std::fmt;
use ureq::Error;

use super::{
	listing::{
		DirectoryItem, OwnedDirectoryItems,
	},
	Client,
	Versions,
};

/// Branch available on a remote server.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Branch {
	id: String,
}

impl Branch {
	/// Return an iterator of all versions available on this branch.
	/// 
	/// # Errors
	/// This method will return an error if making the request to the server or reading the response body fails.
	pub fn versions(&self, client: &Client) -> Result<Versions, Error> {
		let root = format!("{}{}/", client.params.root_url, self.id);
		let response = client.agent.get(root.as_str()).call()?
			.into_body().read_to_string()?;

		Ok(Versions {
			inner: OwnedDirectoryItems::new(response),
			root,
		})
	}

	/// Return the name of this branch.
	#[inline]
	pub fn name(&self) -> &str {
		self.id.as_str()
	}

	/// Return the root URL of this branch.
	#[inline]
	pub fn url(&self, client: &Client) -> String {
		format!("{}{}", client.params.root_url, self.id)
	}
}

impl From<Branch> for String {
	#[inline]
	fn from(value: Branch) -> Self {
		value.id
	}
}

impl fmt::Display for Branch {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Display::fmt(&self.id, f)
	}
}

/// Iterator over [`Branch`]es available on a remote server.
pub struct Branches(pub(crate) OwnedDirectoryItems);
impl Iterator for Branches {
	type Item = Branch;
	fn next(&mut self) -> Option<Self::Item> {
		loop {
			let item = self.0.next()?.ok()?;
			if let DirectoryItem::Directory(mut path) = item {
				if !path.starts_with('/') {
					path.pop();
					break Some(Branch {
						id: path,
					})
				}
			}
		}
	}
}
