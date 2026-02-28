use super::traits::{Tool, ToolResult};
use super::url_validation::normalize_allowed_domains;
use crate::config::{BrowserConfig, Config};
use crate::security::SecurityPolicy;
use crate::util::MaybeSet;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::sync::Arc;

const DEFAULT_AGENT_BROWSER_COMMAND: &str = "agent-browser";
const DEFAULT_NATIVE_WEBDRIVER_URL: &str = "http://127.0.0.1:9515";
const DEFAULT_COMPUTER_USE_ENDPOINT: &str = "http://127.0.0.1:8787/v1/actions";

pub struct BrowserConfigTool {
    config: Arc<Config>,
    security: Arc<SecurityPolicy>,
}

impl BrowserConfigTool {
    pub fn new(config: Arc<Config>, security: Arc<SecurityPolicy>) -> Self {
        Self { config, security }
    }

    fn load_config_without_env(&self) -> anyhow::Result<Config> {
        let contents = fs::read_to_string(&self.config.config_path).map_err(|error| {
            anyhow::anyhow!(
                "Failed to read config file {}: {error}",
                self.config.config_path.display()
            )
        })?;

        let mut parsed: Config = toml::from_str(&contents).map_err(|error| {
            anyhow::anyhow!(
                "Failed to parse config file {}: {error}",
                self.config.config_path.display()
            )
        })?;
        parsed.config_path = self.config.config_path.clone();
        parsed.workspace_dir = self.config.workspace_dir.clone();
        Ok(parsed)
    }

    fn require_write_access(&self) -> Option<ToolResult> {
        if !self.security.can_act() {
            return Some(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: autonomy is read-only".into()),
            });
        }

        if !self.security.record_action() {
            return Some(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: rate limit exceeded".into()),
            });
        }

        None
    }

    fn parse_string_list(raw: &Value, field: &str) -> anyhow::Result<Vec<String>> {
        if let Some(raw_string) = raw.as_str() {
            return Ok(raw_string
                .split(',')
                .map(str::trim)
                .filter(|entry| !entry.is_empty())
                .map(ToOwned::to_owned)
                .collect());
        }

        if let Some(array) = raw.as_array() {
            let mut out = Vec::new();
            for item in array {
                let value = item
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("'{field}' array must only contain strings"))?;
                let trimmed = value.trim();
                if !trimmed.is_empty() {
                    out.push(trimmed.to_string());
                }
            }
            return Ok(out);
        }

        anyhow::bail!("'{field}' must be a string or string[]")
    }

    fn parse_optional_string_update(args: &Value, field: &str) -> anyhow::Result<MaybeSet<String>> {
        let Some(raw) = args.get(field) else {
            return Ok(MaybeSet::Unset);
        };

        if raw.is_null() {
            return Ok(MaybeSet::Null);
        }

        let value = raw
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("'{field}' must be a string or null"))?
            .trim()
            .to_string();

        let output = if value.is_empty() {
            MaybeSet::Null
        } else {
            MaybeSet::Set(value)
        };
        Ok(output)
    }

    fn parse_optional_i64_update(args: &Value, field: &str) -> anyhow::Result<MaybeSet<i64>> {
        let Some(raw) = args.get(field) else {
            return Ok(MaybeSet::Unset);
        };

        if raw.is_null() {
            return Ok(MaybeSet::Null);
        }

        let value = raw
            .as_i64()
            .ok_or_else(|| anyhow::anyhow!("'{field}' must be an integer or null"))?;
        Ok(MaybeSet::Set(value))
    }

    fn normalize_browser_open(raw: &str) -> Option<&'static str> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "disable" => Some("disable"),
            "brave" => Some("brave"),
            "chrome" => Some("chrome"),
            "firefox" => Some("firefox"),
            "edge" | "msedge" => Some("edge"),
            "default" => Some("default"),
            _ => None,
        }
    }

    fn normalize_backend(raw: &str) -> Option<&'static str> {
        match raw.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "agent_browser" | "agentbrowser" => Some("agent_browser"),
            "rust_native" | "native" => Some("rust_native"),
            "computer_use" | "computeruse" => Some("computer_use"),
            "auto" => Some("auto"),
            _ => None,
        }
    }

    fn normalize_auto_backend(raw: &str) -> Option<&'static str> {
        match raw.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "agent_browser" | "agentbrowser" => Some("agent_browser"),
            "rust_native" | "native" => Some("rust_native"),
            "computer_use" | "computeruse" => Some("computer_use"),
            _ => None,
        }
    }

    fn normalize_auto_backend_list(raw: Vec<String>, field: &str) -> anyhow::Result<Vec<String>> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for entry in raw {
            let normalized = Self::normalize_auto_backend(&entry).ok_or_else(|| {
                anyhow::anyhow!(
                    "Invalid backend '{entry}' in {field}. Supported: agent_browser, rust_native, computer_use"
                )
            })?;
            if seen.insert(normalized) {
                out.push(normalized.to_string());
            }
        }
        Ok(out)
    }

    fn merge_auto_backend_list(base: &mut Vec<String>, additions: Vec<String>) {
        for backend in additions {
            if !base.contains(&backend) {
                base.push(backend);
            }
        }
    }

    fn remove_auto_backend_list(base: &mut Vec<String>, removals: Vec<String>) {
        let removal_set: HashSet<String> = removals.into_iter().collect();
        base.retain(|backend| !removal_set.contains(backend));
    }

    fn normalize_freeform_list(values: Vec<String>) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for value in values {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                continue;
            }
            let normalized = trimmed.to_string();
            if seen.insert(normalized.clone()) {
                out.push(normalized);
            }
        }
        out
    }

    fn snapshot(cfg: &BrowserConfig) -> Value {
        json!({
            "enabled": cfg.enabled,
            "allowed_domains": cfg.allowed_domains,
            "browser_open": cfg.browser_open,
            "session_name": cfg.session_name,
            "backend": cfg.backend,
            "auto_backend_priority": cfg.auto_backend_priority,
            "agent_browser_command": cfg.agent_browser_command,
            "agent_browser_extra_args": cfg.agent_browser_extra_args,
            "agent_browser_timeout_ms": cfg.agent_browser_timeout_ms,
            "native_headless": cfg.native_headless,
            "native_webdriver_url": cfg.native_webdriver_url,
            "native_chrome_path": cfg.native_chrome_path,
            "computer_use": {
                "endpoint": cfg.computer_use.endpoint,
                "api_key_configured": cfg
                    .computer_use
                    .api_key
                    .as_ref()
                    .is_some_and(|v| !v.trim().is_empty()),
                "timeout_ms": cfg.computer_use.timeout_ms,
                "allow_remote_endpoint": cfg.computer_use.allow_remote_endpoint,
                "window_allowlist": cfg.computer_use.window_allowlist,
                "max_coordinate_x": cfg.computer_use.max_coordinate_x,
                "max_coordinate_y": cfg.computer_use.max_coordinate_y
            }
        })
    }

    fn handle_get(&self) -> anyhow::Result<ToolResult> {
        let cfg = self.load_config_without_env()?;
        Ok(ToolResult {
            success: true,
            output: serde_json::to_string_pretty(&Self::snapshot(&cfg.browser))?,
            error: None,
        })
    }

    fn handle_list_backends(&self) -> anyhow::Result<ToolResult> {
        Ok(ToolResult {
            success: true,
            output: serde_json::to_string_pretty(&json!({
                "supported_backends": ["agent_browser", "rust_native", "computer_use", "auto"],
                "auto_backend_priority_values": ["agent_browser", "rust_native", "computer_use"],
                "browser_open_values": ["disable", "brave", "chrome", "firefox", "edge", "default"],
                "examples": {
                    "enable_auto_backend": {
                        "action": "set",
                        "enabled": true,
                        "backend": "auto",
                        "auto_backend_priority": ["agent_browser", "rust_native", "computer_use"]
                    },
                    "set_rust_native_backend": {
                        "action": "set",
                        "backend": "rust_native",
                        "native_headless": true,
                        "native_webdriver_url": "http://127.0.0.1:9515"
                    },
                    "set_computer_use_backend": {
                        "action": "set",
                        "backend": "computer_use",
                        "computer_use_endpoint": "http://127.0.0.1:8787/v1/actions",
                        "computer_use_allow_remote_endpoint": false
                    }
                }
            }))?,
            error: None,
        })
    }

    async fn handle_set(&self, args: &Value) -> anyhow::Result<ToolResult> {
        let mut cfg = self.load_config_without_env()?;
        let browser = &mut cfg.browser;

        if let Some(enabled) = args.get("enabled") {
            browser.enabled = enabled
                .as_bool()
                .ok_or_else(|| anyhow::anyhow!("'enabled' must be a boolean"))?;
        }

        if let Some(raw) = args.get("allowed_domains") {
            browser.allowed_domains =
                normalize_allowed_domains(Self::parse_string_list(raw, "allowed_domains")?);
        }

        if let Some(raw) = args.get("browser_open") {
            let value = raw
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("'browser_open' must be a string"))?;
            let normalized = Self::normalize_browser_open(value).ok_or_else(|| {
                anyhow::anyhow!(
                    "Invalid browser_open '{}'. Supported: disable, brave, chrome, firefox, edge, default",
                    value
                )
            })?;
            browser.browser_open = normalized.to_string();
        }

        if let Some(raw) = args.get("backend") {
            let value = raw
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("'backend' must be a string"))?;
            let normalized = Self::normalize_backend(value).ok_or_else(|| {
                anyhow::anyhow!(
                    "Invalid backend '{}'. Supported: agent_browser, rust_native, computer_use, auto",
                    value
                )
            })?;
            browser.backend = normalized.to_string();
        }

        if let Some(raw) = args.get("auto_backend_priority") {
            browser.auto_backend_priority = Self::normalize_auto_backend_list(
                Self::parse_string_list(raw, "auto_backend_priority")?,
                "auto_backend_priority",
            )?;
        }

        if let Some(raw) = args.get("add_auto_backend_priority") {
            let additions = Self::normalize_auto_backend_list(
                Self::parse_string_list(raw, "add_auto_backend_priority")?,
                "add_auto_backend_priority",
            )?;
            Self::merge_auto_backend_list(&mut browser.auto_backend_priority, additions);
        }

        if let Some(raw) = args.get("remove_auto_backend_priority") {
            let removals = Self::normalize_auto_backend_list(
                Self::parse_string_list(raw, "remove_auto_backend_priority")?,
                "remove_auto_backend_priority",
            )?;
            Self::remove_auto_backend_list(&mut browser.auto_backend_priority, removals);
        }

        if let Some(raw) = args.get("agent_browser_extra_args") {
            browser.agent_browser_extra_args = Self::normalize_freeform_list(
                Self::parse_string_list(raw, "agent_browser_extra_args")?,
            );
        }

        if let Some(raw) = args.get("agent_browser_timeout_ms") {
            let value = raw
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("'agent_browser_timeout_ms' must be an integer"))?;
            if value == 0 {
                anyhow::bail!("'agent_browser_timeout_ms' must be > 0");
            }
            browser.agent_browser_timeout_ms = value;
        }

        if let Some(raw) = args.get("native_headless") {
            browser.native_headless = raw
                .as_bool()
                .ok_or_else(|| anyhow::anyhow!("'native_headless' must be a boolean"))?;
        }

        if let Some(raw) = args.get("computer_use_timeout_ms") {
            let value = raw
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("'computer_use_timeout_ms' must be an integer"))?;
            if value == 0 {
                anyhow::bail!("'computer_use_timeout_ms' must be > 0");
            }
            browser.computer_use.timeout_ms = value;
        }

        if let Some(raw) = args.get("computer_use_allow_remote_endpoint") {
            browser.computer_use.allow_remote_endpoint = raw.as_bool().ok_or_else(|| {
                anyhow::anyhow!("'computer_use_allow_remote_endpoint' must be a boolean")
            })?;
        }

        if let Some(raw) = args.get("computer_use_window_allowlist") {
            browser.computer_use.window_allowlist = Self::normalize_freeform_list(
                Self::parse_string_list(raw, "computer_use_window_allowlist")?,
            );
        }

        match Self::parse_optional_string_update(args, "session_name")? {
            MaybeSet::Set(value) => browser.session_name = Some(value),
            MaybeSet::Null => browser.session_name = None,
            MaybeSet::Unset => {}
        }

        match Self::parse_optional_string_update(args, "agent_browser_command")? {
            MaybeSet::Set(value) => browser.agent_browser_command = value,
            MaybeSet::Null => browser.agent_browser_command = DEFAULT_AGENT_BROWSER_COMMAND.into(),
            MaybeSet::Unset => {}
        }

        match Self::parse_optional_string_update(args, "native_webdriver_url")? {
            MaybeSet::Set(value) => browser.native_webdriver_url = value,
            MaybeSet::Null => browser.native_webdriver_url = DEFAULT_NATIVE_WEBDRIVER_URL.into(),
            MaybeSet::Unset => {}
        }

        match Self::parse_optional_string_update(args, "native_chrome_path")? {
            MaybeSet::Set(value) => browser.native_chrome_path = Some(value),
            MaybeSet::Null => browser.native_chrome_path = None,
            MaybeSet::Unset => {}
        }

        match Self::parse_optional_string_update(args, "computer_use_endpoint")? {
            MaybeSet::Set(value) => browser.computer_use.endpoint = value,
            MaybeSet::Null => browser.computer_use.endpoint = DEFAULT_COMPUTER_USE_ENDPOINT.into(),
            MaybeSet::Unset => {}
        }

        match Self::parse_optional_string_update(args, "computer_use_api_key")? {
            MaybeSet::Set(value) => browser.computer_use.api_key = Some(value),
            MaybeSet::Null => browser.computer_use.api_key = None,
            MaybeSet::Unset => {}
        }

        match Self::parse_optional_i64_update(args, "computer_use_max_coordinate_x")? {
            MaybeSet::Set(value) => browser.computer_use.max_coordinate_x = Some(value),
            MaybeSet::Null => browser.computer_use.max_coordinate_x = None,
            MaybeSet::Unset => {}
        }

        match Self::parse_optional_i64_update(args, "computer_use_max_coordinate_y")? {
            MaybeSet::Set(value) => browser.computer_use.max_coordinate_y = Some(value),
            MaybeSet::Null => browser.computer_use.max_coordinate_y = None,
            MaybeSet::Unset => {}
        }

        cfg.save().await?;

        Ok(ToolResult {
            success: true,
            output: serde_json::to_string_pretty(&json!({
                "message": "browser configuration updated",
                "browser": Self::snapshot(&cfg.browser)
            }))?,
            error: None,
        })
    }
}

#[async_trait]
impl Tool for BrowserConfigTool {
    fn name(&self) -> &str {
        "browser_config"
    }

    fn description(&self) -> &str {
        "Inspect and update [browser] runtime configuration (backend selection, allowlist, and computer_use controls)."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["get", "set", "list_backends"],
                    "description": "Operation to perform"
                },
                "enabled": {"type": "boolean"},
                "allowed_domains": {
                    "anyOf": [
                        {"type": "string"},
                        {"type": "array", "items": {"type": "string"}}
                    ]
                },
                "browser_open": {"type": "string"},
                "session_name": {"type": ["string", "null"]},
                "backend": {"type": "string"},
                "auto_backend_priority": {
                    "anyOf": [
                        {"type": "string"},
                        {"type": "array", "items": {"type": "string"}}
                    ]
                },
                "add_auto_backend_priority": {
                    "anyOf": [
                        {"type": "string"},
                        {"type": "array", "items": {"type": "string"}}
                    ]
                },
                "remove_auto_backend_priority": {
                    "anyOf": [
                        {"type": "string"},
                        {"type": "array", "items": {"type": "string"}}
                    ]
                },
                "agent_browser_command": {"type": ["string", "null"]},
                "agent_browser_extra_args": {
                    "anyOf": [
                        {"type": "string"},
                        {"type": "array", "items": {"type": "string"}}
                    ]
                },
                "agent_browser_timeout_ms": {"type": "integer", "minimum": 1},
                "native_headless": {"type": "boolean"},
                "native_webdriver_url": {"type": ["string", "null"]},
                "native_chrome_path": {"type": ["string", "null"]},
                "computer_use_endpoint": {"type": ["string", "null"]},
                "computer_use_api_key": {"type": ["string", "null"]},
                "computer_use_timeout_ms": {"type": "integer", "minimum": 1},
                "computer_use_allow_remote_endpoint": {"type": "boolean"},
                "computer_use_window_allowlist": {
                    "anyOf": [
                        {"type": "string"},
                        {"type": "array", "items": {"type": "string"}}
                    ]
                },
                "computer_use_max_coordinate_x": {"type": ["integer", "null"]},
                "computer_use_max_coordinate_y": {"type": ["integer", "null"]}
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let action = args
            .get("action")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("Missing required field: action"))?;

        match action {
            "get" => self.handle_get(),
            "list_backends" => self.handle_list_backends(),
            "set" => {
                if let Some(blocked) = self.require_write_access() {
                    return Ok(blocked);
                }
                self.handle_set(&args).await
            }
            other => anyhow::bail!("Unsupported action '{other}'. Use get|set|list_backends"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{AutonomyLevel, SecurityPolicy};
    use tempfile::TempDir;

    fn test_security() -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            workspace_dir: std::env::temp_dir(),
            ..SecurityPolicy::default()
        })
    }

    async fn test_config(tmp: &TempDir) -> Arc<Config> {
        let config = Config {
            workspace_dir: tmp.path().join("workspace"),
            config_path: tmp.path().join("config.toml"),
            ..Config::default()
        };
        config.save().await.unwrap();
        Arc::new(config)
    }

    #[tokio::test]
    async fn list_backends_includes_supported_values() {
        let tmp = TempDir::new().unwrap();
        let tool = BrowserConfigTool::new(test_config(&tmp).await, test_security());

        let result = tool
            .execute(json!({"action": "list_backends"}))
            .await
            .unwrap();
        assert!(result.success, "{:?}", result.error);
        assert!(result.output.contains("agent_browser"));
        assert!(result.output.contains("rust_native"));
        assert!(result.output.contains("computer_use"));
        assert!(result.output.contains("auto"));
    }

    #[tokio::test]
    async fn set_normalizes_backend_priority_and_browser_open() {
        let tmp = TempDir::new().unwrap();
        let tool = BrowserConfigTool::new(test_config(&tmp).await, test_security());

        let result = tool
            .execute(json!({
                "action": "set",
                "enabled": true,
                "backend": "AUTO",
                "auto_backend_priority": ["native", "agent_browser", "computeruse", "native"],
                "browser_open": "msedge",
                "allowed_domains": ["https://Example.com", "api.example.com"],
                "computer_use_window_allowlist": "Chrome,Terminal"
            }))
            .await
            .unwrap();
        assert!(result.success, "{:?}", result.error);

        let output: Value = serde_json::from_str(&result.output).unwrap();
        let browser = &output["browser"];
        assert_eq!(browser["backend"], json!("auto"));
        assert_eq!(
            browser["auto_backend_priority"],
            json!(["rust_native", "agent_browser", "computer_use"])
        );
        assert_eq!(browser["browser_open"], json!("edge"));
        assert_eq!(
            browser["allowed_domains"],
            json!(["api.example.com", "example.com"])
        );
        assert_eq!(
            browser["computer_use"]["window_allowlist"],
            json!(["Chrome", "Terminal"])
        );
    }

    #[tokio::test]
    async fn set_can_clear_optional_fields_with_null() {
        let tmp = TempDir::new().unwrap();
        let tool = BrowserConfigTool::new(test_config(&tmp).await, test_security());

        let initial = tool
            .execute(json!({
                "action": "set",
                "session_name": "team-browser",
                "native_chrome_path": "/usr/bin/google-chrome",
                "computer_use_api_key": "secret-token",
                "computer_use_max_coordinate_x": 1920
            }))
            .await
            .unwrap();
        assert!(initial.success, "{:?}", initial.error);

        let cleared = tool
            .execute(json!({
                "action": "set",
                "session_name": null,
                "native_chrome_path": null,
                "computer_use_api_key": null,
                "computer_use_max_coordinate_x": null
            }))
            .await
            .unwrap();
        assert!(cleared.success, "{:?}", cleared.error);

        let output: Value = serde_json::from_str(&cleared.output).unwrap();
        let browser = &output["browser"];
        assert!(browser["session_name"].is_null());
        assert!(browser["native_chrome_path"].is_null());
        assert!(browser["computer_use"]["max_coordinate_x"].is_null());
        assert_eq!(browser["computer_use"]["api_key_configured"], json!(false));
    }

    #[tokio::test]
    async fn set_rejects_unknown_backend() {
        let tmp = TempDir::new().unwrap();
        let tool = BrowserConfigTool::new(test_config(&tmp).await, test_security());

        let err = tool
            .execute(json!({
                "action": "set",
                "backend": "unknown_backend"
            }))
            .await
            .expect_err("unknown backend should fail");
        assert!(err.to_string().contains("Invalid backend"));
    }
}
