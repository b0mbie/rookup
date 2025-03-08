use ureq::{
	Agent, Error,
};

pub(crate) mod listing;

mod archive;
pub use archive::*;
mod branches;
pub use branches::*;
mod versions;
pub use versions::*;

/// `User-Agent` used when making HTTP requests.
pub const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), '/', env!("CARGO_PKG_VERSION"));

/// Client used for interacting with `smdrop`.
#[derive(Debug, Clone)]
pub struct Client {
	pub agent: Agent,
	pub params: ClientParams,
}

/// Parameters for an `smdrop` client.
#[derive(Debug, Clone, PartialEq, PartialOrd, Hash)]
pub struct ClientParams {
	pub root_url: String,
}

impl Client {
	const fn with_agent(params: ClientParams, agent: Agent) -> Self {
		Self {
			agent,
			params,
		}
	}

	/// Create a new client, given its client parameters.
	pub fn new(params: ClientParams) -> Self {
		Self::with_agent(params, Agent::new_with_config(Agent::config_builder().user_agent(USER_AGENT).build()))
	}

	/// Return an iterator over all branches available on the server.
	/// 
	/// # Errors
	/// This method will return an error if making the request to the server or reading the response body fails.
	pub fn branches(&self) -> Result<Branches, Error> {
		let response = self.agent.get(self.params.root_url.as_str()).call()?
			.into_body().read_to_string()?;
	
		Ok(Branches(listing::OwnedDirectoryItems::new(response)))
	}
}
