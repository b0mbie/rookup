use anyhow::{
	anyhow, Context, Result as AResult
};
use core::cmp::Ordering;
use rookup_common::{
	version::{
		version_ord, Version
	},
	Config, ConfigData, Selector,
};

use crate::smdrop::{
	Branch, Branches, Client, ClientParams, VersionUrl,
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

fn select_branch_with_ver(mut branches: Branches, version: &str) -> AResult<Branch> {
	branches.find(move |b| version.is_sub_version_of(b.name()))
		.with_context(|| anyhow!("couldn't select branch with selector {version:?}"))
}

pub trait ClientExt {
	fn select_branch(&self, data: &ConfigData, selector: Selector<'_>) -> AResult<Branch>;
}
impl ClientExt for Client {
	fn select_branch(&self, data: &ConfigData, selector: Selector<'_>) -> AResult<Branch> {
		fn branch_ord(a: &Branch, b: &Branch) -> Ordering {
			version_ord(a.name(), b.name())
		}
	
		let branches = self.branches().context("couldn't fetch branches")?;
		match selector {
			Selector::Alias("latest") => {
				branches.max_by(branch_ord).context("couldn't select latest branch")
			}
			Selector::Alias("stable") => {
				let mut branches: Vec<_> = branches.collect();
				branches.sort_by(branch_ord);
				branches.pop();
				branches.pop().context("couldn't select latest stable branch")
			}
			Selector::Alias(s) => {
				let version = data.aliases.get(s).with_context(|| anyhow!("failed to resolve alias {s:?}"))?;
				select_branch_with_ver(branches, version)
			}
			Selector::Super(s) => {
				select_branch_with_ver(branches, s)
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
