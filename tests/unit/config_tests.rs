use toggle::config::ToggleConfig;

#[test]
fn test_parse_full_config() {
    let toml_str = r##"
[global]
default_mode = "auto"
force_state = "none"
single_line_delimiter = "//"

[language.python]
single_line_delimiter = "#"

[language.javascript]
single_line_delimiter = "//"
"##;
    let config: ToggleConfig = toml::from_str(toml_str).unwrap();
    let global = config.global.unwrap();
    assert_eq!(global.default_mode.unwrap(), "auto");
    assert_eq!(global.force_state.unwrap(), "none");
    assert_eq!(global.single_line_delimiter.unwrap(), "//");

    let langs = config.language.unwrap();
    assert_eq!(
        langs
            .get("python")
            .unwrap()
            .single_line_delimiter
            .as_deref(),
        Some("#")
    );
    assert_eq!(
        langs
            .get("javascript")
            .unwrap()
            .single_line_delimiter
            .as_deref(),
        Some("//")
    );
}

#[test]
fn test_parse_global_only() {
    let toml_str = r##"
[global]
default_mode = "single"
"##;
    let config: ToggleConfig = toml::from_str(toml_str).unwrap();
    assert!(config.global.is_some());
    assert!(config.language.is_none());
    assert_eq!(config.global.unwrap().default_mode.unwrap(), "single");
}

#[test]
fn test_parse_empty_config() {
    let config: ToggleConfig = toml::from_str("").unwrap();
    assert!(config.global.is_none());
    assert!(config.language.is_none());
}

#[test]
fn test_get_language_delimiter_found() {
    let toml_str = r###"
[language.python]
single_line_delimiter = "##"
"###;
    let config: ToggleConfig = toml::from_str(toml_str).unwrap();
    assert_eq!(config.get_language_delimiter("python"), Some("##"));
}

#[test]
fn test_get_language_delimiter_not_found() {
    let toml_str = r##"
[language.python]
single_line_delimiter = "#"
"##;
    let config: ToggleConfig = toml::from_str(toml_str).unwrap();
    assert_eq!(config.get_language_delimiter("rust"), None);
}

#[test]
fn test_get_language_delimiter_no_languages() {
    let config: ToggleConfig = toml::from_str("").unwrap();
    assert_eq!(config.get_language_delimiter("python"), None);
}

#[test]
fn test_parse_invalid_toml() {
    let result: Result<ToggleConfig, _> = toml::from_str("invalid [[[toml");
    assert!(result.is_err());
}
