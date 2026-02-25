//! Plugin interface types for WASM ABI.
//!
//! These types are serializable and used for communication between
//! ZeroClaw and WASM plugins.

use serde::{Deserialize, Serialize};

/// Plugin metadata from `plugin_info` export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Unique plugin identifier (e.g., "redis", "elasticsearch")
    pub id: String,
    /// Human-readable plugin name
    pub name: String,
    /// Plugin version (semver recommended)
    pub version: String,
    /// Type of plugin (determines which subsystem it extends)
    #[serde(rename = "type")]
    pub plugin_type: PluginType,
}

/// Types of plugins supported by ZeroClaw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginType {
    /// Memory backend plugin (implements Memory trait)
    MemoryBackend,
    // Future plugin types:
    // /// LLM provider plugin
    // Provider,
    // /// Messaging channel plugin
    // Channel,
    // /// Tool plugin
    // Tool,
}

// ── Memory Backend Plugin Request/Response Types ───────────────────────────

/// Request for memory store operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStoreRequest {
    /// Memory key
    pub key: String,
    /// Memory content
    pub content: String,
    /// Memory category
    pub category: String,
    /// Optional session ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// Response from memory store operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStoreResponse {
    /// Whether the operation succeeded
    pub success: bool,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Request for memory recall operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecallRequest {
    /// Search query
    pub query: String,
    /// Maximum results to return
    pub limit: usize,
    /// Optional session ID filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// Response from memory recall operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecallResponse {
    /// Matching memory entries
    pub entries: Vec<MemoryEntryData>,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Request for memory get operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryGetRequest {
    /// Memory key to retrieve
    pub key: String,
}

/// Response from memory get operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryGetResponse {
    /// Memory entry if found
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<MemoryEntryData>,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Request for memory list operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryListRequest {
    /// Optional category filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Optional session ID filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// Response from memory list operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryListResponse {
    /// Memory entries
    pub entries: Vec<MemoryEntryData>,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Request for memory forget operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryForgetRequest {
    /// Memory key to delete
    pub key: String,
}

/// Response from memory forget operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryForgetResponse {
    /// Whether the key was found and deleted
    pub deleted: bool,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Request for memory count operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCountRequest;

/// Response from memory count operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCountResponse {
    /// Total number of memories
    pub count: usize,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Request for health check operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryHealthCheckRequest;

/// Response from health check operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryHealthCheckResponse {
    /// Whether the backend is healthy
    pub healthy: bool,
    /// Optional status message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Memory entry data transferred to/from plugins.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntryData {
    /// Unique entry ID
    pub id: String,
    /// Memory key
    pub key: String,
    /// Memory content
    pub content: String,
    /// Memory category
    pub category: String,
    /// ISO 8601 timestamp
    pub timestamp: String,
    /// Optional session ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Optional relevance score
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_info_serialization() {
        let info = PluginInfo {
            id: "redis".into(),
            name: "Redis Memory Backend".into(),
            version: "1.0.0".into(),
            plugin_type: PluginType::MemoryBackend,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"type\":\"memory_backend\""));
        let parsed: PluginInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "redis");
    }

    #[test]
    fn memory_store_request_serialization() {
        let req = MemoryStoreRequest {
            key: "test_key".into(),
            content: "test content".into(),
            category: "core".into(),
            session_id: Some("session-123".into()),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("session_id"));
    }

    #[test]
    fn memory_store_request_omits_none_session() {
        let req = MemoryStoreRequest {
            key: "test_key".into(),
            content: "test content".into(),
            category: "core".into(),
            session_id: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(!json.contains("session_id"));
    }
}
