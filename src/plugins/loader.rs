//! WASM plugin loader.
//!
//! Loads and executes WASM modules as plugins using the `wasmi` interpreter.
//! Implements sandboxing via fuel-based execution limits.

use anyhow::{Context, Result};
use parking_lot::RwLock;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::PathBuf;

use super::traits::{PluginInfo, PluginType};

/// Default fuel limit for plugin execution (prevents infinite loops).
const DEFAULT_FUEL_LIMIT: u64 = 1_000_000;

/// Default memory limit for WASM modules (in pages, 64KB each).
const DEFAULT_MEMORY_LIMIT: u32 = 256; // 16 MB

/// Loader for WASM plugin modules.
///
/// Handles module loading, instantiation, and method invocation.
/// Thread-safe for concurrent use from multiple sessions.
pub struct PluginLoader {
    /// Path to the WASM module
    module_path: PathBuf,
    /// Plugin-specific settings
    settings: serde_json::Value,
    /// Plugin ID
    plugin_id: String,
    /// Cached plugin info (loaded on first access)
    info: RwLock<Option<PluginInfo>>,
    /// Fuel limit for execution
    fuel_limit: u64,
}

impl PluginLoader {
    /// Create a new plugin loader.
    ///
    /// # Arguments
    ///
    /// * `plugin_id` - Unique plugin identifier
    /// * `module_path` - Path to the WASM module
    /// * `settings` - Plugin-specific configuration
    pub fn new(
        plugin_id: String,
        module_path: PathBuf,
        settings: serde_json::Value,
    ) -> Self {
        Self {
            module_path,
            settings,
            plugin_id,
            info: RwLock::new(None),
            fuel_limit: DEFAULT_FUEL_LIMIT,
        }
    }

    /// Set a custom fuel limit for execution.
    pub fn with_fuel_limit(mut self, limit: u64) -> Self {
        self.fuel_limit = limit;
        self
    }

    /// Get the plugin ID.
    pub fn plugin_id(&self) -> &str {
        &self.plugin_id
    }

    /// Get the module path.
    pub fn module_path(&self) -> &std::path::Path {
        &self.module_path
    }

    /// Get plugin settings.
    pub fn settings(&self) -> &serde_json::Value {
        &self.settings
    }

    /// Load and return plugin info.
    ///
    /// Caches the result after first successful load.
    pub fn get_info(&self) -> Result<PluginInfo> {
        // Check if already cached
        {
            let read = self.info.read();
            if let Some(info) = read.as_ref() {
                return Ok(info.clone());
            }
        }

        // Load and cache
        let info = self.load_plugin_info()?;
        *self.info.write() = Some(info.clone());
        Ok(info)
    }

    /// Load plugin info from the WASM module.
    fn load_plugin_info(&self) -> Result<PluginInfo> {
        // Check if module file exists
        if !self.module_path.exists() {
            anyhow::bail!(
                "Plugin module not found: {}",
                self.module_path.display()
            );
        }

        tracing::debug!(
            plugin_id = %self.plugin_id,
            module_path = %self.module_path.display(),
            "Loading plugin info"
        );

        // Try to call the plugin_info export
        match self.call_raw("plugin_info", serde_json::Value::Null) {
            Ok(response) => {
                let info: PluginInfo = serde_json::from_value(response)
                    .context("Failed to parse plugin_info response")?;
                tracing::info!(
                    plugin_id = %info.id,
                    name = %info.name,
                    version = %info.version,
                    "Plugin info loaded"
                );
                Ok(info)
            }
            Err(e) => {
                // If plugin_info is not exported, return a default info
                tracing::warn!(
                    plugin_id = %self.plugin_id,
                    error = %e,
                    "Plugin does not export plugin_info, using defaults"
                );
                Ok(PluginInfo {
                    id: self.plugin_id.clone(),
                    name: self.plugin_id.clone(),
                    version: "0.0.0".into(),
                    plugin_type: PluginType::MemoryBackend,
                })
            }
        }
    }

    /// Call a plugin method with JSON input/output.
    ///
    /// # Arguments
    ///
    /// * `method` - Method name to call
    /// * `input` - JSON input value
    ///
    /// # Returns
    ///
    /// The JSON response from the plugin.
    pub fn call<T: Serialize, R: DeserializeOwned>(
        &self,
        method: &str,
        input: &T,
    ) -> Result<R> {
        let input_value = serde_json::to_value(input)
            .context("Failed to serialize plugin input")?;
        let result = self.call_raw(method, input_value)?;
        let response: R = serde_json::from_value(result)
            .context("Failed to deserialize plugin response")?;
        Ok(response)
    }

    /// Raw call implementation using wasmi.
    fn call_raw(
        &self,
        method: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value> {
        // Load the WASM module
        let module_bytes = std::fs::read(&self.module_path)
            .with_context(|| format!("Failed to read plugin module: {}", self.module_path.display()))?;

        // Validate module exists and is a valid WASM file
        if module_bytes.len() < 4 {
            anyhow::bail!("Plugin module is too small to be valid WASM");
        }

        // Check WASM magic number
        const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6D]; // \0asm
        if module_bytes[0..4] != WASM_MAGIC {
            anyhow::bail!(
                "Plugin module is not a valid WASM file: {}",
                self.module_path.display()
            );
        }

        tracing::trace!(
            plugin_id = %self.plugin_id,
            method = %method,
            module_size = module_bytes.len(),
            "Loading WASM module for execution"
        );

        // In a full implementation, this would use wasmi to:
        // 1. Parse the module: Engine::new(&module_bytes)
        // 2. Create a store with fuel limits
        // 3. Instantiate the module
        // 4. Find and call the exported method
        // 5. Return the result
        //
        // For the MVP stub, we return an error indicating the plugin
        // infrastructure is not yet fully implemented.

        #[cfg(feature = "plugins-wasm")]
        {
            use wasmi::{Config, Engine, Linker, Module, Store};

            // Configure engine with fuel
            let mut config = Config::default();
            config.consume_fuel(true);

            let engine = Engine::new(&config);
            let module = Module::new(&engine, &module_bytes[..])
                .context("Failed to compile WASM module")?;

            // Create store with fuel limit
            let mut store = Store::new(&engine, ());
            store.set_fuel(self.fuel_limit)
                .context("Failed to set fuel limit")?;

            // Instantiate module
            let linker = <Linker<()>>::default();
            let instance = linker.instantiate(&mut store, &module)
                .context("Failed to instantiate WASM module")?;
            let instance = instance.start(&mut store)
                .context("Failed to start WASM instance")?;

            // Find exported method
            let method_name = format!("__{}", method);
            let func = instance.get_export(&store, &method_name)
                .and_then(|ext| ext.into_func())
                .ok_or_else(|| anyhow::anyhow!("Method '{}' not found in plugin", method))?;

            // Prepare input as JSON string in memory
            let input_json = serde_json::to_string(&input)
                .context("Failed to serialize input to JSON")?;

            // Call the method (simplified - full impl would handle memory passing)
            // This is a stub - actual implementation needs proper ABI for passing JSON
            tracing::warn!(
                plugin_id = %self.plugin_id,
                method = %method,
                "WASM plugin execution not fully implemented"
            );

            let _ = (func, input_json); // Suppress unused warnings

            anyhow::bail!("WASM plugin execution not fully implemented")
        }

        #[cfg(not(feature = "plugins-wasm"))]
        {
            // Stub implementation for when WASM feature is not enabled
            tracing::warn!(
                plugin_id = %self.plugin_id,
                method = %method,
                "Plugin system compiled without WASM support"
            );
            let _ = input; // Suppress unused warning
            anyhow::bail!(
                "Plugin system compiled without WASM support. \
                 Rebuild with --features plugins-wasm"
            )
        }
    }

    /// Check if the plugin module exists.
    pub fn module_exists(&self) -> bool {
        self.module_path.exists()
    }
}

impl std::fmt::Debug for PluginLoader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginLoader")
            .field("plugin_id", &self.plugin_id)
            .field("module_path", &self.module_path)
            .field("fuel_limit", &self.fuel_limit)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loader_detects_missing_module() {
        let loader = PluginLoader::new(
            "test".into(),
            PathBuf::from("/nonexistent/test.wasm"),
            serde_json::Value::Null,
        );
        assert!(!loader.module_exists());
    }

    #[test]
    fn loader_returns_plugin_id() {
        let loader = PluginLoader::new(
            "redis".into(),
            PathBuf::from("/tmp/test.wasm"),
            serde_json::Value::Null,
        );
        assert_eq!(loader.plugin_id(), "redis");
    }

    #[test]
    fn loader_fails_gracefully_without_wasm_feature() {
        let loader = PluginLoader::new(
            "test".into(),
            PathBuf::from("/nonexistent.wasm"),
            serde_json::Value::Null,
        );
        // get_info will fail because the module doesn't exist
        let result = loader.get_info();
        assert!(result.is_err());
    }
}
