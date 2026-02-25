//! Plugin registry for discovery and lookup.
//!
//! The registry manages plugin metadata and provides lookup by ID.
//! It supports the `plugin:<id>` backend naming convention.

use crate::config::PluginsConfig;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::path::PathBuf;

use super::traits::{PluginInfo, PluginType};

/// Entry for a registered plugin.
#[derive(Debug, Clone)]
pub struct PluginEntry {
    /// Plugin metadata
    pub info: PluginInfo,
    /// Path to the WASM module
    pub module_path: PathBuf,
    /// Whether this plugin is enabled
    pub enabled: bool,
    /// Plugin-specific settings
    pub settings: HashMap<String, serde_json::Value>,
}

/// Registry for discovered and loaded plugins.
///
/// Thread-safe for concurrent access from multiple agents/sessions.
pub struct PluginRegistry {
    /// Registered plugins keyed by ID
    plugins: RwLock<HashMap<String, PluginEntry>>,
    /// Directory containing plugin WASM modules
    plugins_dir: PathBuf,
    /// Whether the plugin system is enabled
    enabled: bool,
}

impl PluginRegistry {
    /// Create a new plugin registry from configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - Plugin system configuration
    /// * `workspace_dir` - ZeroClaw workspace directory for resolving relative paths
    pub fn new(config: &PluginsConfig, workspace_dir: &std::path::Path) -> Self {
        let plugins_dir = if PathBuf::from(&config.dir).is_absolute() {
            PathBuf::from(&config.dir)
        } else {
            workspace_dir.join(&config.dir)
        };

        let mut registry = Self {
            plugins: RwLock::new(HashMap::new()),
            plugins_dir,
            enabled: config.enabled,
        };

        if config.enabled {
            registry.discover_plugins(config);
        }

        registry
    }

    /// Discover and register plugins from configuration.
    fn discover_plugins(&self, config: &PluginsConfig) {
        let mut plugins = self.plugins.write();

        for (id, plugin_config) in &config.memory_backends {
            if !plugin_config.enabled {
                tracing::debug!("Plugin '{id}' is disabled, skipping");
                continue;
            }

            let module_path = if PathBuf::from(&plugin_config.module).is_absolute() {
                PathBuf::from(&plugin_config.module)
            } else {
                self.plugins_dir.join(&plugin_config.module)
            };

            // Create a placeholder entry; actual info will be loaded on first use
            let entry = PluginEntry {
                info: PluginInfo {
                    id: id.clone(),
                    name: id.clone(),
                    version: "unknown".into(),
                    plugin_type: PluginType::MemoryBackend,
                },
                module_path,
                enabled: plugin_config.enabled,
                settings: plugin_config.settings.clone(),
            };

            tracing::info!(
                plugin_id = %id,
                module_path = %entry.module_path.display(),
                "Registered memory backend plugin"
            );

            plugins.insert(id.clone(), entry);
        }
    }

    /// Get a plugin entry by ID.
    pub fn get(&self, id: &str) -> Option<PluginEntry> {
        self.plugins.read().get(id).cloned()
    }

    /// Check if a backend name refers to a plugin.
    ///
    /// Returns `true` if the backend starts with "plugin:".
    pub fn is_plugin_backend(&self, backend: &str) -> bool {
        backend.starts_with("plugin:")
    }

    /// Parse the plugin ID from a backend name.
    ///
    /// Returns `Some(id)` if the backend is "plugin:<id>", `None` otherwise.
    pub fn parse_plugin_id(&self, backend: &str) -> Option<String> {
        backend.strip_prefix("plugin:").map(str::to_string)
    }

    /// Check if the plugin system is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get the plugins directory.
    pub fn plugins_dir(&self) -> &std::path::Path {
        &self.plugins_dir
    }

    /// List all registered plugin IDs.
    pub fn plugin_ids(&self) -> Vec<String> {
        self.plugins.read().keys().cloned().collect()
    }

    /// Update plugin info after loading.
    pub fn update_info(&self, id: &str, info: PluginInfo) {
        if let Some(entry) = self.plugins.write().get_mut(id) {
            entry.info = info;
        }
    }
}

impl std::fmt::Debug for PluginRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginRegistry")
            .field("enabled", &self.enabled)
            .field("plugins_dir", &self.plugins_dir)
            .field("plugin_count", &self.plugins.read().len())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_config() -> PluginsConfig {
        PluginsConfig {
            enabled: true,
            dir: "plugins".into(),
            memory_backends: {
                let mut map = HashMap::new();
                map.insert(
                    "redis".into(),
                    crate::config::MemoryPluginConfig {
                        module: "memory-redis.wasm".into(),
                        settings: HashMap::new(),
                        enabled: true,
                    },
                );
                map
            },
        }
    }

    #[test]
    fn registry_parses_plugin_id() {
        let config = PluginsConfig {
            enabled: false,
            dir: "plugins".into(),
            memory_backends: HashMap::new(),
        };
        let registry = PluginRegistry::new(&config, std::path::Path::new("/tmp"));
        assert_eq!(registry.parse_plugin_id("plugin:redis"), Some("redis".into()));
        assert_eq!(registry.parse_plugin_id("sqlite"), None);
    }

    #[test]
    fn registry_detects_plugin_backend() {
        let config = PluginsConfig {
            enabled: false,
            dir: "plugins".into(),
            memory_backends: HashMap::new(),
        };
        let registry = PluginRegistry::new(&config, std::path::Path::new("/tmp"));
        assert!(registry.is_plugin_backend("plugin:redis"));
        assert!(!registry.is_plugin_backend("sqlite"));
    }

    #[test]
    fn registry_discovers_plugins_when_enabled() {
        let config = make_config();
        let registry = PluginRegistry::new(&config, std::path::Path::new("/tmp"));
        assert!(registry.is_enabled());
        assert!(registry.get("redis").is_some());
    }

    #[test]
    fn registry_skips_plugins_when_disabled() {
        let mut config = make_config();
        config.enabled = false;
        let registry = PluginRegistry::new(&config, std::path::Path::new("/tmp"));
        assert!(!registry.is_enabled());
        assert!(registry.get("redis").is_none());
    }
}
