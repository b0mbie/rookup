use anyhow::{
	Result as AResult,
	anyhow, bail,
};
use core::cmp::Ordering;
use rookup_common::{
	version::{
		Version, version_ord,
	},
	Config, Selector,
};

use crate::smdrop::{
	Branch, Client, ClientParams, VersionUrl,
};

pub fn smdrop_client(config: &Config) -> Client {
	let params = ClientParams {
		root_url: config.with_doc.data().source.root_url.clone(),
	};
	Client::new(params)
}

#[derive(Debug)]
pub struct RelevantUrl {
	url: VersionUrl<Box<str>>,
	version: Box<str>,
}
impl RelevantUrl {
	#[inline]
	pub fn new(url: VersionUrl<Box<str>>) -> Option<Self> {
		if
			url.target().is_none_or(|t| t != std::env::consts::OS)
			|| url.version_str().is_none_or(move |v| v.0 == "latest")
		{
			return None
		}

		let version = url.version_str().map(move |v| v.normalized().into_owned().into_boxed_str())?;
		Some(Self {
			url,
			version,
		})
	}

	#[inline]
	pub fn url(&self) -> &str {
		&self.url
	}

	#[inline]
	pub fn version(&self) -> &str {
		&self.version
	}

	pub fn version_ord(&self, other: &Self) -> Ordering {
		version_ord(self.version.as_ref(), other.version.as_ref())
	}
}

pub trait ClientExt {
	fn select_branch(&self, selector: Selector<'_>) -> AResult<Branch>;
}
impl ClientExt for Client {
	fn select_branch(&self, selector: Selector<'_>) -> AResult<Branch> {
		fn branch_ord(a: &Branch, b: &Branch) -> Ordering {
			version_ord(a.name(), b.name())
		}
	
		let mut branches = self.branches().map_err(|e| anyhow!("couldn't fetch branches: {e}"))?;
		match selector {
			Selector::Alias("latest") => {
				branches.max_by(branch_ord)
					.ok_or_else(move || anyhow!("couldn't select latest branch"))
			}
			Selector::Alias("stable") => {
				let mut branches: Vec<_> = branches.collect();
				branches.sort_by(branch_ord);
				branches.pop();
				branches.pop().ok_or_else(move || anyhow!("couldn't select latest stable branch"))
			}
			Selector::Alias(s) => {
				bail!("alias {s:?} is not supported; supported aliases are `latest` and `stable`");
			}
			Selector::Super(s) => {
				branches.find(move |b| s.is_sub_version_of(b.name()))
					.ok_or_else(|| anyhow!("couldn't select branch with selector {s:?}"))
			}
		}
	}
}

pub trait BranchExt {
	fn relevant_urls(&self, client: &Client) -> AResult<impl Iterator<Item = RelevantUrl>>;
}
impl BranchExt for Branch {
	fn relevant_urls(&self, client: &Client) -> AResult<impl Iterator<Item = RelevantUrl>> {
		let versions = self.versions(client)
			.map_err(move |e| anyhow!("couldn't fetch versions for branch {:?}: {e}", self.name()))?;
		Ok(versions.map(move |v| v.into_url()).filter_map(RelevantUrl::new))
	}
}
