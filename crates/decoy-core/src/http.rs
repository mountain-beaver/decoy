//! Minimal blocking HTTPS GET shared by the public lookups.

use std::time::Duration;

use crate::error::{Error, Result};

/// GET `url` and return the body, failing with `NotFound` on 404.
pub(crate) fn get_bytes(url: &str, timeout: Duration, max_bytes: u64) -> Result<Vec<u8>> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(timeout))
        .build()
        .into();
    let mut resp = agent
        .get(url)
        .header("User-Agent", "decoy")
        .call()
        .map_err(|e| match e {
            ureq::Error::StatusCode(404) => Error::NotFound,
            e => Error::Network(e.to_string()),
        })?;
    resp.body_mut()
        .with_config()
        .limit(max_bytes)
        .read_to_vec()
        .map_err(|e| Error::Network(e.to_string()))
}
