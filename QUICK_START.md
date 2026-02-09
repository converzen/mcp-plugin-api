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
use std::ffi::CString;
use serde_json::json;

// Declare plugin with automatic version tracking
declare_plugin! {
    register: register_plugin,
    free_string: plugin_free_string
}

// Register your tools
extern "C" fn register_plugin(registrar: *mut PluginRegistrar) -> i32 {
    let tool = ToolDeclaration {
        name: CString::new("my_tool").unwrap().into_raw(),
        description: CString::new("My awesome tool").unwrap().into_raw(),
        parameters_json: CString::new(json!({
            "type": "object",
            "properties": {
                "input": {"type": "string"}
            }
        }).to_string()).unwrap().into_raw(),
        execute: execute_tool,
        free_result: plugin_free_string,
    };
    
    unsafe { ((*registrar).register_tool)(&tool) };
    0
}

// Execute your tool
unsafe extern "C" fn execute_tool(
    args_json: *const u8,
    args_len: usize,
    result_buf: *mut *mut u8,
    result_len: *mut usize,
) -> i32 {
    // Parse input
    let args = std::slice::from_raw_parts(args_json, args_len);
    let args_str = std::str::from_utf8(args).unwrap();
    
    // Do your work
    let result = json!({"output": format!("Processed: {}", args_str)});
    
    // Return result
    let result_vec = result.to_string().into_bytes();
    *result_len = result_vec.capacity();
    *result_buf = result_vec.as_ptr() as *mut u8;
    std::mem::forget(result_vec);
    
    0  // Success
}

// Free memory
unsafe extern "C" fn plugin_free_string(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        let _ = Vec::from_raw_parts(ptr, len, len);
    }
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

✅ **Automatic Versioning**: The `declare_plugin!` macro embeds the API version  
✅ **Thread-Safe**: Your execute function will be called concurrently  
✅ **Memory Management**: Plugin allocates, framework frees via your function  
✅ **Zero Dependencies**: Only needs `mcp-plugin-api`  

## Common Patterns

### Return Success
```rust
let result = json!({"key": "value"});
let mut vec = result.to_string().into_bytes();
vec.shrink_to_fit();
*result_len = vec.capacity();
*result_buf = vec.as_ptr() as *mut u8;
std::mem::forget(vec);
```

### Return Error
```rust
let error = json!({"error": "Something went wrong"});
// ... same as success
return 1;  // Non-zero error code
```

### Thread-Safe State
```rust
use once_cell::sync::Lazy;
use std::sync::Mutex;

static STATE: Lazy<Mutex<MyState>> = Lazy::new(|| {
    Mutex::new(MyState::new())
});

unsafe extern "C" fn execute_tool(...) -> i32 {
    let state = STATE.lock().unwrap();
    // Use state safely
}
```

## Next Steps

- See [PLUGIN_DEVELOPMENT.md](../../PLUGIN_DEVELOPMENT.md) for detailed guide
- Check [pricing plugin](../../plugins/pricing/src/lib.rs) for complete example
- Read [API documentation](README.md) for interface details

## Need Help?

- Check the example plugin: `plugins/pricing/`
- Read the full development guide: `PLUGIN_DEVELOPMENT.md`
- Review the architecture: `ARCHITECTURE.md`

