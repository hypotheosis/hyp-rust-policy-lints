use serde::Deserialize;

/// This library's table in the consuming workspace's `dylint.toml`.
pub const TABLE: &str = "hyp_tracing";

/// `[hyp_tracing]` in `dylint.toml`.
///
/// ```toml
/// [hyp_tracing]
/// enabled = false
/// reason = "CLI tool with no tracing subscriber"
/// ```
///
/// Unknown keys are rejected: `enable = false` is a typo that would otherwise
/// deserialize to the default and leave the policy silently on -- or, for a
/// misspelt `reason`, silently unjustified.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "enabled_by_default")]
    enabled: bool,
    #[serde(default)]
    reason: Option<String>,
}

fn enabled_by_default() -> bool {
    true
}

impl Default for Config {
    /// No `[hyp_tracing]` table: enforce.
    fn default() -> Self {
        Self {
            enabled: enabled_by_default(),
            reason: None,
        }
    }
}

/// What the pass does for the crate being compiled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Enforce,
    /// Switched off with a non-blank reason.
    Off,
    /// Switched off without a reason, or with a blank one: the pass reports
    /// the missing justification once and nothing else.
    OffWithoutReason,
}

impl Config {
    pub fn mode(&self) -> Mode {
        if self.enabled {
            return Mode::Enforce;
        }
        // As for `allow(...)`, only the presence of a non-blank reason is
        // checked, never its quality.
        match &self.reason {
            Some(reason) if !reason.trim().is_empty() => Mode::Off,
            _ => Mode::OffWithoutReason,
        }
    }
}

/// Read `[hyp_tracing]` from the workspace's `dylint.toml`.
///
/// `dylint_linting::config`, not `config_or_default`: the latter panics on a
/// bad value, which a consumer would see as a compiler crash rather than a
/// configuration error.
pub fn load() -> Result<Config, dylint_linting::ConfigError> {
    dylint_linting::config::<Config>(TABLE).map(Option::unwrap_or_default)
}
