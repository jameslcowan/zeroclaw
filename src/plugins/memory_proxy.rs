//! Memory trait adapter for plugins.
//!
//! Implements the `Memory` trait by delegating to a WASM plugin.

use async_trait::async_trait;
use std::sync::Arc;

use crate::memory::traits::{Memory, MemoryCategory, MemoryEntry};
use crate::memory::MarkdownMemory;

use super::loader::PluginLoader;
use super::traits::{
    MemoryCountRequest, MemoryCountResponse, MemoryForgetRequest, MemoryForgetResponse,
    MemoryGetRequest, MemoryGetResponse, MemoryHealthCheckRequest, MemoryHealthCheckResponse,
    MemoryListRequest, MemoryListResponse, MemoryRecallRequest, MemoryRecallResponse,
    MemoryStoreRequest, MemoryStoreResponse,
};

/// Memory backend that delegates to a WASM plugin.
///
/// Falls back to markdown memory if the plugin fails.
pub struct PluginMemory {
    /// Plugin loader for WASM calls
    plugin: Arc<PluginLoader>,
    /// Plugin ID (used as backend name)
    plugin_id: String,
    /// Fallback markdown memory for graceful degradation
    fallback: Arc<MarkdownMemory>,
    /// Whether the plugin is currently healthy
    healthy: std::sync::atomic::AtomicBool,
}

impl PluginMemory {
    /// Create a new plugin-backed memory.
    ///
    /// # Arguments
    ///
    /// * `plugin` - The plugin loader
    /// * `workspace_dir` - Workspace directory for fallback storage
    pub fn new(plugin: Arc<PluginLoader>, workspace_dir: &std::path::Path) -> Self {
        let plugin_id = plugin.plugin_id().to_string();
        let fallback = Arc::new(MarkdownMemory::new(workspace_dir));

        Self {
            plugin,
            plugin_id,
            fallback,
            healthy: std::sync::atomic::AtomicBool::new(true),
        }
    }

    /// Convert category to string for plugin serialization.
    fn category_to_string(category: &MemoryCategory) -> String {
        match category {
            MemoryCategory::Core => "core".into(),
            MemoryCategory::Daily => "daily".into(),
            MemoryCategory::Conversation => "conversation".into(),
            MemoryCategory::Custom(name) => name.clone(),
        }
    }

    /// Parse category from string.
    fn string_to_category(s: &str) -> MemoryCategory {
        match s {
            "core" => MemoryCategory::Core,
            "daily" => MemoryCategory::Daily,
            "conversation" => MemoryCategory::Conversation,
            other => MemoryCategory::Custom(other.into()),
        }
    }

    /// Call plugin with fallback on error.
    fn call_or_fallback<T, R>(
        &self,
        operation: &str,
        request: &T,
    ) -> Result<R, PluginError>
    where
        T: serde::Serialize + std::fmt::Debug,
        R: serde::de::DeserializeOwned,
    {
        tracing::trace!(
            plugin_id = %self.plugin_id,
            operation = %operation,
            "Calling plugin"
        );

        match self.plugin.call(operation, request) {
            Ok(response) => {
                self.healthy.store(true, std::sync::atomic::Ordering::Relaxed);
                Ok(response)
            }
            Err(e) => {
                self.healthy.store(false, std::sync::atomic::Ordering::Relaxed);
                tracing::warn!(
                    plugin_id = %self.plugin_id,
                    operation = %operation,
                    error = %e,
                    "Plugin call failed, using fallback"
                );
                Err(PluginError(e))
            }
        }
    }
}

/// Error wrapper for plugin failures.
struct PluginError(anyhow::Error);

#[async_trait]
impl Memory for PluginMemory {
    fn name(&self) -> &str {
        &self.plugin_id
    }

    async fn store(
        &self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        session_id: Option<&str>,
    ) -> anyhow::Result<()> {
        let request = MemoryStoreRequest {
            key: key.into(),
            content: content.into(),
            category: Self::category_to_string(&category),
            session_id: session_id.map(str::to_string),
        };

        match self.call_or_fallback("memory_store", &request) {
            Ok(response) => {
                let response: MemoryStoreResponse = response;
                if !response.success {
                    if let Some(error) = response.error {
                        anyhow::bail!("Plugin store failed: {}", error);
                    }
                }
                Ok(())
            }
            Err(_) => {
                // Fallback to markdown
                self.fallback.store(key, content, category, session_id).await
            }
        }
    }

    async fn recall(
        &self,
        query: &str,
        limit: usize,
        session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        let request = MemoryRecallRequest {
            query: query.into(),
            limit,
            session_id: session_id.map(str::to_string),
        };

        match self.call_or_fallback("memory_recall", &request) {
            Ok(response) => {
                let response: MemoryRecallResponse = response;
                if let Some(error) = response.error {
                    tracing::warn!("Plugin recall error: {}", error);
                }

                Ok(response
                    .entries
                    .into_iter()
                    .map(|e| MemoryEntry {
                        id: e.id,
                        key: e.key,
                        content: e.content,
                        category: Self::string_to_category(&e.category),
                        timestamp: e.timestamp,
                        session_id: e.session_id,
                        score: e.score,
                    })
                    .collect())
            }
            Err(_) => {
                // Fallback to markdown
                self.fallback.recall(query, limit, session_id).await
            }
        }
    }

    async fn get(&self, key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        let request = MemoryGetRequest { key: key.into() };

        match self.call_or_fallback("memory_get", &request) {
            Ok(response) => {
                let response: MemoryGetResponse = response;
                if let Some(error) = response.error {
                    tracing::warn!("Plugin get error: {}", error);
                }

                Ok(response.entry.map(|e| MemoryEntry {
                    id: e.id,
                    key: e.key,
                    content: e.content,
                    category: Self::string_to_category(&e.category),
                    timestamp: e.timestamp,
                    session_id: e.session_id,
                    score: e.score,
                }))
            }
            Err(_) => {
                // Fallback to markdown
                self.fallback.get(key).await
            }
        }
    }

    async fn list(
        &self,
        category: Option<&MemoryCategory>,
        session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        let request = MemoryListRequest {
            category: category.map(|c| Self::category_to_string(c)),
            session_id: session_id.map(str::to_string),
        };

        match self.call_or_fallback("memory_list", &request) {
            Ok(response) => {
                let response: MemoryListResponse = response;
                if let Some(error) = response.error {
                    tracing::warn!("Plugin list error: {}", error);
                }

                Ok(response
                    .entries
                    .into_iter()
                    .map(|e| MemoryEntry {
                        id: e.id,
                        key: e.key,
                        content: e.content,
                        category: Self::string_to_category(&e.category),
                        timestamp: e.timestamp,
                        session_id: e.session_id,
                        score: e.score,
                    })
                    .collect())
            }
            Err(_) => {
                // Fallback to markdown
                self.fallback.list(category, session_id).await
            }
        }
    }

    async fn forget(&self, key: &str) -> anyhow::Result<bool> {
        let request = MemoryForgetRequest { key: key.into() };

        match self.call_or_fallback("memory_forget", &request) {
            Ok(response) => {
                let response: MemoryForgetResponse = response;
                if let Some(error) = response.error {
                    tracing::warn!("Plugin forget error: {}", error);
                }
                Ok(response.deleted)
            }
            Err(_) => {
                // Fallback to markdown
                self.fallback.forget(key).await
            }
        }
    }

    async fn count(&self) -> anyhow::Result<usize> {
        let request = MemoryCountRequest;

        match self.call_or_fallback("memory_count", &request) {
            Ok(response) => {
                let response: MemoryCountResponse = response;
                if let Some(error) = response.error {
                    tracing::warn!("Plugin count error: {}", error);
                }
                Ok(response.count)
            }
            Err(_) => {
                // Fallback to markdown
                self.fallback.count().await
            }
        }
    }

    async fn health_check(&self) -> bool {
        let request = MemoryHealthCheckRequest;

        match self.plugin.call::<_, MemoryHealthCheckResponse>("memory_health_check", &request) {
            Ok(response) => {
                let healthy = response.healthy;
                self.healthy.store(healthy, std::sync::atomic::Ordering::Relaxed);
                healthy
            }
            Err(e) => {
                tracing::warn!(
                    plugin_id = %self.plugin_id,
                    error = %e,
                    "Plugin health check failed"
                );
                self.healthy.store(false, std::sync::atomic::Ordering::Relaxed);
                // Fall back to markdown health
                self.fallback.health_check().await
            }
        }
    }
}

impl std::fmt::Debug for PluginMemory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginMemory")
            .field("plugin_id", &self.plugin_id)
            .field("healthy", &self.healthy.load(std::sync::atomic::Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn plugin_memory_name_returns_plugin_id() {
        let tmp = TempDir::new().unwrap();
        let loader = Arc::new(PluginLoader::new(
            "redis".into(),
            PathBuf::from("/tmp/test.wasm"),
            serde_json::Value::Null,
        ));
        let memory = PluginMemory::new(loader, tmp.path());
        assert_eq!(memory.name(), "redis");
    }

    #[test]
    fn category_conversion_roundtrip() {
        assert_eq!(
            PluginMemory::string_to_category(&PluginMemory::category_to_string(
                &MemoryCategory::Core
            )),
            MemoryCategory::Core
        );
        assert_eq!(
            PluginMemory::string_to_category(&PluginMemory::category_to_string(
                &MemoryCategory::Custom("custom".into())
            )),
            MemoryCategory::Custom("custom".into())
        );
    }
}
