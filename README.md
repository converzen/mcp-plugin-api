# MCP Plugin API

The interface crate for building MCP (Model Context Protocol) server plugins.

## Overview

This crate defines the C ABI interface between the MCP framework and plugins. It contains only type definitions and no implementation code, making it lightweight and stable.

## Why a Separate Crate?

Having the plugin API in a separate crate provides several benefits:

1. **No Code Duplication**: The interface is defined once and shared
2. **Lightweight**: Plugins only depend on this tiny crate (~5KB)
3. **Clean Dependencies**: No circular dependencies or framework bloat
4. **Versioning**: API can be versioned independently
5. **External Development**: Third parties can develop plugins without accessing framework code

## Dependency Graph

```
┌─────────────────┐
│ mcp-plugin-api  │  ← Interface definitions only
└────────┬────────┘
         │
    ┌────┴────┬──────────┐
    │         │          │
    ▼         ▼          ▼
framework  plugin-A  plugin-B
```

## Usage in Plugins

Add to your plugin's `Cargo.toml`:

```toml
[dependencies]
mcp-plugin-api = { path = "../../mcp-plugin-api" }
# Or from crates.io:
# mcp-plugin-api = "0.1"
```

Then in your plugin:

```rust
use mcp_plugin_api::*;

// Declare plugin with automatic version management
declare_plugin! {
    register: register_plugin,
    free_string: plugin_free_string
}

extern "C" fn register_plugin(registrar: *mut PluginRegistrar) -> i32 {
    // Register your tools...
    0
}

unsafe extern "C" fn plugin_free_string(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        let _ = Vec::from_raw_parts(ptr, len, len);
    }
}
```

The `declare_plugin!` macro automatically embeds the API version from the crate you're building against, ensuring version tracking without manual management.

## Key Types

- **`PluginDeclaration`**: Main plugin entry point
- **`PluginRegistrar`**: Used to register tools during initialization
- **`ToolDeclaration`**: Defines a tool's metadata and execution function

## Memory Safety

The API enforces proper memory management across the plugin boundary:

1. Plugin allocates memory using its allocator
2. Plugin returns pointer and capacity to framework
3. Framework uses the data
4. Framework calls plugin's `free_string` to deallocate
5. Plugin properly deallocates using its allocator

This prevents cross-allocator corruption.

## Thread Safety

All tool execution functions will be called concurrently from multiple threads. Implementations **must be thread-safe**.

## Version Compatibility

The API uses semantic versioning. Breaking changes increment the major version. Plugins built against API v0.1.x are compatible with frameworks using API v0.1.y (where y >= x).

### Automatic Version Tracking

The plugin API version is automatically embedded in your plugin at compile time. When you build a plugin:

1. The `declare_plugin!` macro reads the API version from `mcp-plugin-api`'s Cargo.toml
2. This version is embedded as a constant in your compiled `.so` file
3. The framework reads this version when loading your plugin
4. Major version mismatches generate warnings

This means:
- ✅ No manual version management needed
- ✅ Plugin version always matches the API it was built against
- ✅ Framework can validate compatibility automatically
- ✅ Version is auditable from the plugin binary

**Note:** The Rust compiler version is irrelevant. The C ABI is stable across rustc versions, so only the API version matters for compatibility.

## License

MIT OR Apache-2.0

