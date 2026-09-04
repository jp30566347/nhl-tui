//! Optional on-disk settings, so `--team` need not be retyped every launch.
//!
//! The file is read at startup and never written unless the user explicitly
//! asks with `--save-config`. A missing file is fine and yields defaults; a
//! malformed one is reported and stops startup, so a typo shows up as an
//! error rather than as a setting that silently does nothing.

use std::path::PathBuf;

use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Favorite team abbreviation, e.g. "TOR".
    pub team: Option<String>,
    /// Tab to open on, 1-5.
    pub tab: Option<u8>,
}

/// `$XDG_CONFIG_HOME/nhl-tui/config.toml`, or the platform equivalent.
pub fn path() -> Option<PathBuf> {
    Some(dirs::config_dir()?.join("nhl-tui").join("config.toml"))
}

impl Config {
    /// Reads the config file. Returns the defaults when it does not exist,
    /// and an error only when it exists but cannot be understood.
    pub fn load() -> Result<Self> {
        let Some(path) = path() else {
            return Ok(Self::default());
        };
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
        };
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
    }

    pub fn save(&self) -> Result<PathBuf> {
        let path = path().ok_or_else(|| color_eyre::eyre::eyre!("no config directory"))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let text = toml::to_string_pretty(self).context("serializing config")?;
        std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_config_is_valid() {
        assert_eq!(toml::from_str::<Config>("").unwrap(), Config::default());
    }

    #[test]
    fn round_trips() {
        let config = Config {
            team: Some("TOR".into()),
            tab: Some(2),
        };
        let text = toml::to_string_pretty(&config).unwrap();
        assert_eq!(toml::from_str::<Config>(&text).unwrap(), config);
    }

    /// A typo should be reported, not silently ignored, or the user will
    /// wonder why their setting does nothing.
    #[test]
    fn unknown_keys_are_rejected() {
        assert!(toml::from_str::<Config>("teem = \"TOR\"").is_err());
    }
}
