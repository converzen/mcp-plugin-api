# MCP Plugin API

The interface crate for building MCP (Model Context Protocol) server plugins.

## Overview

This crate defines the C ABI interface between the MCP framework and plugins. It supports both **Tools** and **Resources** capabilities. Plugins can expose tools (callable functions), resources (URI-addressable content), or both.

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
use serde_json::{json, Value};

fn handle_hello(args: &Value) -> Result<Value, String> {
    Ok(json!({ "message": "Hello!" }))
}

declare_tools! {
    tools: [
        Tool::builder("hello", "Say hello", true).handler(handle_hello),
    ]
}

declare_plugin! {
    list_tools: generated_list_tools,
    execute_tool: generated_execute_tool,
    free_string: mcp_plugin_api::utils::standard_free_string
}
```

Plugins can expose **static resources** (`resources/list`, `resources/read` for fixed URIs), **resource templates** (`resources/templates/list` with `{variable}` placeholders in URIs), and a **read fallback** for custom routing. Wire optional ABI slots in `declare_plugin!`: `list_resources`, `list_resource_templates`, `read_resource`.

The `declare_plugin!` macro automatically embeds the API version from the crate you're building against, ensuring version tracking without manual management.

## Key Types

- **`PluginDeclaration`**: Main plugin entry point
- **`Tool`** / **`ToolBuilder`**: High-level tool definitions with `declare_tools!`
- **`Resource`** / **`ResourceBuilder`**: Fixed-URI resources for `declare_resources!`
- **`ResourceTemplate`** / **`ResourceTemplateBuilder`**: URI templates (`file:///x/{path}`) for `resources/templates/list`
- **`TemplateResourceHandler`**, **`GenericResourceReadHandler`**: Template and fallback read handlers
- **`ResourceContent`**: `resources/read` body items (text or binary)
- **`ListResourceTemplatesFn`**: C ABI for `resources/templates/list` JSON

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

