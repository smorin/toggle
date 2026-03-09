// Configuration file support for the Toggle CLI

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Deserialize, Default)]
pub struct ToggleConfig {
    pub global: Option<GlobalConfig>,
    pub language: Option<HashMap<String, LanguageConfig>>,
}

#[derive(Debug, Deserialize, Default)]
pub struct GlobalConfig {
    pub default_mode: Option<String>,
    pub force_state: Option<String>,
    pub single_line_delimiter: Option<String>,
    pub multi_line_delimiter_start: Option<String>,
    pub multi_line_delimiter_end: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct LanguageConfig {
    pub single_line_delimiter: Option<String>,
    pub multi_line_delimiter_start: Option<String>,
    pub multi_line_delimiter_end: Option<String>,
}

impl ToggleConfig {
    /// Load a toggle config from a TOML file.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read config file '{}': {}", path.display(), e))?;
        let config: ToggleConfig = toml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse config file '{}': {}", path.display(), e))?;
        Ok(config)
    }

    /// Get the single-line comment delimiter for a given language name.
    /// Returns None if no language-specific override is configured.
    pub fn get_language_delimiter(&self, lang: &str) -> Option<&str> {
        self.language
            .as_ref()
            .and_then(|langs| langs.get(lang))
            .and_then(|lc| lc.single_line_delimiter.as_deref())
    }
}
