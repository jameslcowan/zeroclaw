//! Memory factory integration with plugins.
//!
//! This module provides plugin-aware memory factory functions that integrate
//! with the memory module's trait system.

use std::path::Path;
use std::sync::Arc;

use crate::config::{EmbeddingRouteConfig, MemoryConfig, StorageProviderConfig};
use crate::memory::traits::Memory;
use crate::memory::{MarkdownMemory, MemoryBackendKind};
use crate::memory::{
    classify_memory_backend, effective_memory_backend_name,
    create_memory_with_storage_and_routes,
};

use super::loader::PluginLoader;
use super::memory_proxy::PluginMemory;
use super::registry::PluginRegistry;

/// Factory: create memory with optional plugin registry support.
///
/// This is the main entry point for memory creation when the plugin system
/// is enabled. It handles `plugin:<id>` backend names by delegating to the
/// appropriate plugin loader.
///
/// # Arguments
///
/// * `config` - Memory configuration
/// * `embedding_routes` - Embedding route configuration
/// * `storage_provider` - Optional storage provider override
/// * `workspace_dir` - Workspace directory
/// * `api_key` - Optional API key for embedding providers
/// * `plugin_registry` - Optional plugin registry for plugin backends
///
/// # Fallback Behavior
///
/// - Plugin disabled: Falls back to markdown with warning
/// - Plugin not found: Falls back to markdown with error log
/// - Plugin crashes: Falls back to markdown with error log
/// - Feature not compiled: `plugin:` treated as `Unknown` → markdown fallback
pub fn create_memory_with_plugins(
    config: &MemoryConfig,
    embedding_routes: &[EmbeddingRouteConfig],
    storage_provider: Option<&StorageProviderConfig>,
    workspace_dir: &Path,
    api_key: Option<&str>,
    plugin_registry: Option<&PluginRegistry>,
) -> anyhow::Result<Box<dyn Memory>> {
    let backend_name = effective_memory_backend_name(&config.backend, storage_provider);
    let backend_kind = classify_memory_backend(&backend_name);

    // Handle plugin backend
    if matches!(backend_kind, MemoryBackendKind::Plugin) {
        return create_plugin_memory(&backend_name, workspace_dir, plugin_registry);
    }

    // Delegate to standard factory for non-plugin backends
    create_memory_with_storage_and_routes(config, embedding_routes, storage_provider, workspace_dir, api_key)
}

/// Create a plugin-backed memory backend.
fn create_plugin_memory(
    backend_name: &str,
    workspace_dir: &Path,
    plugin_registry: Option<&PluginRegistry>,
) -> anyhow::Result<Box<dyn Memory>> {
    // Extract plugin ID from "plugin:<id>" format
    let plugin_id = backend_name
        .strip_prefix("plugin:")
        .expect("backend_name should start with 'plugin:'");

    // Check if plugin registry is available and enabled
    let Some(registry) = plugin_registry else {
        tracing::warn!(
            plugin_id = %plugin_id,
            "Plugin backend requested but plugin system not initialized, falling back to markdown"
        );
        return Ok(Box::new(MarkdownMemory::new(workspace_dir)));
    };

    if !registry.is_enabled() {
        tracing::warn!(
            plugin_id = %plugin_id,
            "Plugin backend requested but plugin system is disabled, falling back to markdown"
        );
        return Ok(Box::new(MarkdownMemory::new(workspace_dir)));
    }

    // Look up plugin configuration
    let Some(entry) = registry.get(plugin_id) else {
        tracing::error!(
            plugin_id = %plugin_id,
            "Plugin backend '{}' not found in registry, falling back to markdown",
            plugin_id
        );
        return Ok(Box::new(MarkdownMemory::new(workspace_dir)));
    };

    // Check if plugin is enabled
    if !entry.enabled {
        tracing::warn!(
            plugin_id = %plugin_id,
            "Plugin backend '{}' is disabled, falling back to markdown",
            plugin_id
        );
        return Ok(Box::new(MarkdownMemory::new(workspace_dir)));
    }

    // Check if module exists
    if !entry.module_path.exists() {
        tracing::error!(
            plugin_id = %plugin_id,
            module_path = %entry.module_path.display(),
            "Plugin module not found, falling back to markdown"
        );
        return Ok(Box::new(MarkdownMemory::new(workspace_dir)));
    }

    tracing::info!(
        plugin_id = %plugin_id,
        module_path = %entry.module_path.display(),
        "Loading plugin memory backend"
    );

    // Create plugin loader
    let settings = serde_json::to_value(&entry.settings).unwrap_or(serde_json::Value::Null);
    let loader = Arc::new(PluginLoader::new(
        plugin_id.to_string(),
        entry.module_path,
        settings,
    ));

    // Create plugin memory with fallback
    Ok(Box::new(PluginMemory::new(loader, workspace_dir)))
}
