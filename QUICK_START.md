# Quick Start: MCP Plugin API

Create MCP server plugins in 5 minutes.

## Step 1: Create Plugin Crate

```bash
cargo new --lib my_plugin
cd my_plugin
```

## Step 2: Configure Cargo.toml

```toml
[package]
name = "my_plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]  # Creates .so file

[dependencies]
mcp-plugin-api = { path = "../../mcp-plugin-api" }  # Or from crates.io
serde_json = "1"
```

## Step 3: Implement Plugin (src/lib.rs)

```rust
use mcp_plugin_api::*;
use serde_json::{json, Value};

fn handle_my_tool(args: &Value) -> Result<Value, String> {
    let input = args["input"].as_str().unwrap_or("default");
    Ok(json!({"output": format!("Processed: {}", input)}))
}

declare_tools! {
    tools: [
        Tool::builder("my_tool", "My awesome tool", true)
            .param_string("input", "Input value", false)
            .handler(handle_my_tool),
    ]
}

declare_plugin! {
    list_tools: generated_list_tools,
    execute_tool: generated_execute_tool,
    free_string: mcp_plugin_api::utils::standard_free_string
}
```

## Step 4: Build

```bash
cargo build --release
# Creates: target/release/libmy_plugin.so
```

## Step 5: Use with Framework

Add to `config.toml`:
```toml
[plugins]
directory = "./target/release"
enabled = ["my_plugin"]
```

Run the server:
```bash
./target/release/mcp-server-stdio
```

## Key Points

- **Automatic Versioning**: The `declare_plugin!` macro embeds the API version
- **Thread-Safe**: Your execute function will be called concurrently
- **Memory Management**: Plugin allocates, framework frees via your function
- **Zero Dependencies**: Only needs `mcp-plugin-api`

## Adding Resources (Optional)

Plugins can expose MCP resources in addition to tools:

```rust
use mcp_plugin_api::*;

fn read_readme(uri: &str) -> Result<ResourceContents, String> {
    Ok(vec![ResourceContent::text(
        uri,
        "# Hello\n\nPlugin documentation here.",
        Some("text/markdown".to_string()),
    )])
}

declare_resources! {
    resources: [
        Resource::builder("file:///my-plugin/readme", read_readme)
            .name("readme.md")
            .description("Plugin documentation")
            .mime_type("text/markdown")
            .build(),
    ]
}

declare_plugin! {
    list_tools: generated_list_tools,
    execute_tool: generated_execute_tool,
    free_string: mcp_plugin_api::utils::standard_free_string,
    list_resources: generated_list_resources,
    read_resource: generated_read_resource
}
```

## Common Patterns

### Return Success (from tool handler)
```rust
Ok(json!({"key": "value"}))
```

### Return Error (from tool handler)
```rust
Err("Something went wrong".to_string())
```

### Thread-Safe State
```rust
use once_cell::sync::Lazy;
use std::sync::Mutex;

static STATE: Lazy<Mutex<MyState>> = Lazy::new(|| {
    Mutex::new(MyState::new())
});

fn handle_tool(args: &Value) -> Result<Value, String> {
    let state = STATE.lock().unwrap();
    // Use state safely
    Ok(json!({"status": "ok"}))
}
```

## Next Steps

- See [PLUGIN_DEVELOPMENT.md](../../PLUGIN_DEVELOPMENT.md) for detailed guide
- Read [API documentation](README.md) for interface details
