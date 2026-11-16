use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::content::types::Icon;

/// `Implementation.websiteUrl`: an RFC 3986 URI.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct WebsiteUrl(String);

/// A website URL that is not an RFC 3986 URI.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("website URL must be a URI, got: {0:?}")]
pub struct InvalidWebsiteUrl(pub String);

impl WebsiteUrl {
    /// The URI.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for WebsiteUrl {
    type Error = InvalidWebsiteUrl;

    fn try_from(url: String) -> Result<Self, InvalidWebsiteUrl> {
        if fluent_uri::Uri::parse(url.as_str()).is_ok() {
            Ok(Self(url))
        } else {
            Err(InvalidWebsiteUrl(url))
        }
    }
}

impl FromStr for WebsiteUrl {
    type Err = InvalidWebsiteUrl;

    fn from_str(url: &str) -> Result<Self, InvalidWebsiteUrl> {
        url.to_owned().try_into()
    }
}

/// Server/Client implementation information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Implementation {
    /// Implementation name
    pub name: String,
    /// Implementation version
    pub version: String,
    /// Human-readable title for display
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Description of the implementation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Sized icons for client display
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::content::types::received_icons"
    )]
    pub icons: Option<Vec<Icon>>,
    /// Website URL
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "received_website_url"
    )]
    pub website_url: Option<WebsiteUrl>,
}

/// Decode a received `websiteUrl`, dropping one that is not a URI.
fn received_website_url<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<WebsiteUrl>, D::Error> {
    Ok(
        Option::<String>::deserialize(deserializer)?.and_then(|url| {
            WebsiteUrl::try_from(url)
                .inspect_err(|error| tracing::debug!(%error, "Dropping received websiteUrl"))
                .ok()
        }),
    )
}

impl Implementation {
    /// Create new implementation info (name + version only)
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            title: None,
            description: None,
            icons: None,
            website_url: None,
        }
    }

    fluent_setter!(
        /// Set human-readable title
        pub fn with_title(title: impl Into<String>) => some_into title
    );

    fluent_setter!(
        /// Set description
        pub fn with_description(description: impl Into<String>) => some_into description
    );

    fluent_setter!(
        /// Set icons
        pub fn with_icons(icons: Vec<Icon>) => some icons
    );

    fluent_setter!(
        /// Set website URL
        pub fn with_website_url(url: WebsiteUrl) => some website_url
    );
}

/// Client information with version validation
#[derive(Debug, Clone, Default)]
pub struct ClientInfo {
    /// Client name
    pub name: String,
    /// Client version
    pub version: String,
}

impl ClientInfo {
    /// Create new client info
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
        }
    }

    /// Check if client version meets minimum requirements
    ///
    /// Uses the configurable `MIN_CLIENT_VERSIONS` from version module
    pub fn is_supported(&self, min_versions: &[(&str, &str)]) -> bool {
        for (name, min_version) in min_versions {
            if self.name == *name {
                return version_gte(&self.version, min_version);
            }
        }
        true
    }
}

/// Compare semantic versions (simple: major.minor.patch)
///
/// Returns true if version >= min
pub fn version_gte(version: &str, min: &str) -> bool {
    let parse = |v: &str| -> (u32, u32, u32) {
        let mut parts = v.split('.').filter_map(|p| p.parse().ok());
        (
            parts.next().unwrap_or(0),
            parts.next().unwrap_or(0),
            parts.next().unwrap_or(0),
        )
    };

    let (v_maj, v_min, v_patch) = parse(version);
    let (m_maj, m_min, m_patch) = parse(min);

    (v_maj, v_min, v_patch) >= (m_maj, m_min, m_patch)
}

#[cfg(test)]
mod tests;
