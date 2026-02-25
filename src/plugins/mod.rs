//! Plugin system for ZeroClaw.
//!
//! This module provides a plugin architecture that allows 3rd-party components
//! to extend ZeroClaw without recompilation. The MVP focuses on memory backend
//! plugins, with future extension planned for other subsystems.
//!
//! # Security Model
//!
//! - **Compile-time opt-in**: Requires `--features plugins`
//! - **Runtime opt-in**: `plugins.enabled = true` in config
//! - **Sandboxed**: WASM interpreter with fuel limits (no infinite loops)
//! - **No filesystem by default**: Plugins have no filesystem access
//! - **No network by default**: Plugins have no network access
//! - **Audit logging**: All plugin loads/calls logged
//!
//! # Example Configuration
//!
//! ```toml
//! [plugins]
//! enabled = true
//! dir = "plugins"
//!
//! [plugins.memory_backends.redis]
//! module = "memory-redis.wasm"
//! enabled = true
//!
//! [plugins.memory_backends.redis.settings]
//! url = "redis://localhost:6379"
//!
//! [memory]
//! backend = "plugin:redis"
//! ```

pub mod loader;
pub mod memory_factory;
pub mod memory_proxy;
pub mod registry;
pub mod traits;

pub use loader::PluginLoader;
pub use memory_factory::create_memory_with_plugins;
pub use memory_proxy::PluginMemory;
pub use registry::{PluginEntry, PluginRegistry};
pub use traits::{PluginInfo, PluginType};
