//! Application metadata shared between the runtime and its clients.

use serde::{Deserialize, Serialize};

/// Human readable application name.
pub const APP_NAME: &str = "platform-backend";

/// Application version taken from the crate manifest at compile time.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Build and version information exposed by the API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildInfo {
    name: String,
    version: String,
    rust_version: Option<String>,
}

impl BuildInfo {
    /// Creates build information for the current binary.
    #[must_use]
    pub fn current() -> Self {
        Self {
            name: APP_NAME.to_owned(),
            version: VERSION.to_owned(),
            rust_version: option_env!("PLATFORM_RUST_VERSION").map(str::to_owned),
        }
    }

    /// Creates build information with explicit values.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        rust_version: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            rust_version,
        }
    }

    /// Application name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Semantic version of the running binary.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Rust compiler version used for the build, when recorded.
    #[must_use]
    pub fn rust_version(&self) -> Option<&str> {
        self.rust_version.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::{APP_NAME, BuildInfo, VERSION};

    #[test]
    fn current_build_info_matches_crate_metadata() {
        let info = BuildInfo::current();
        assert_eq!(info.name(), APP_NAME);
        assert_eq!(info.version(), VERSION);
    }

    #[test]
    fn explicit_build_info_keeps_values() {
        let info = BuildInfo::new("svc", "1.2.3", Some("1.97.1".to_owned()));
        assert_eq!(info.name(), "svc");
        assert_eq!(info.version(), "1.2.3");
        assert_eq!(info.rust_version(), Some("1.97.1"));
    }
}
