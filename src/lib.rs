//! MCP Plugin API - Interface definitions for plugin development
//!
//! This crate defines the C ABI interface between the MCP framework
//! and plugins. Both the framework and plugins depend on this crate.
//!
//! ## Overview
//!
//! This crate provides two ways to create plugins:
//!
//! ### 1. High-Level API (Recommended)
//!
//! Use the `Tool` builder and `declare_tools!` macro for a clean, type-safe API:
//!
//! ```ignore
//! use mcp_plugin_api::*;
//! use serde_json::{json, Value};
//!
//! fn handle_hello(args: &Value) -> Result<Value, String> {
//!     let name = args["name"].as_str().unwrap_or("World");
//!     Ok(json!({ "message": format!("Hello, {}!", name) }))
//! }
//!
//! declare_tools! {
//!     tools: [
//!         Tool::new("hello", "Say hello")
//!             .param_string("name", "Name to greet", false)
//!             .handler(handle_hello),
//!     ]
//! }
//!
//! declare_plugin! {
//!     list_tools: generated_list_tools,
//!     execute_tool: generated_execute_tool,
//!     free_string: mcp_plugin_api::utils::standard_free_string
//! }
//! ```
//!
//! ### 2. Low-Level API
//!
//! Manually implement the three C functions for maximum control:
//! - `list_tools`: Returns JSON array of available tools
//! - `execute_tool`: Executes a tool by name
//! - `free_string`: Deallocates plugin-allocated memory
//!
//! ## Memory Management
//!
//! The `utils` module provides safe wrappers for memory management:
//! - `return_success`: Return a success result
//! - `return_error`: Return an error result
//! - `standard_free_string`: Standard deallocation function
//!
//! ## Thread Safety
//!
//! The `execute_tool` function will be called concurrently from multiple
//! threads. Implementations must be thread-safe.

use std::os::raw::c_char;

// Re-export serde_json for use in macros
pub use serde_json;

// Re-export once_cell for configuration
pub use once_cell;

// Export sub-modules
pub mod tool;
pub mod utils;

// Don't make macros a public module - macros are exported at crate root
#[macro_use]
mod macros;

// Re-export commonly used items
pub use tool::{ParamType, Tool, ToolBuilder, ToolHandler, ToolParam};

// ============================================================================
// ABI Type Aliases - Single Source of Truth
// ============================================================================

/// Function signature for listing available tools
///
/// Returns a JSON array of tool definitions.
///
/// # Parameters
/// - `result_buf`: Output pointer for JSON array (allocated by plugin)
/// - `result_len`: Output capacity of buffer
///
/// # Returns
/// - 0 on success
/// - Non-zero error code on failure
pub type ListToolsFn = unsafe extern "C" fn(*mut *mut u8, *mut usize) -> i32;

/// Function signature for executing a tool by name
///
/// # Parameters
/// - `tool_name`: Null-terminated C string with tool name
/// - `args_json`: JSON arguments as byte array
/// - `args_len`: Length of args_json
/// - `result_buf`: Output pointer for result (allocated by plugin)
/// - `result_len`: Output capacity of result buffer
///
/// # Returns
/// - 0 on success
/// - Non-zero error code on failure
pub type ExecuteToolFn = unsafe extern "C" fn(
    *const c_char, // tool name
    *const u8,     // args JSON
    usize,         // args length
    *mut *mut u8,  // result buffer (allocated by plugin)
    *mut usize,    // result capacity
) -> i32;

/// Function signature for freeing memory allocated by the plugin
///
/// # Parameters
/// - `ptr`: Pointer to memory to free
/// - `capacity`: Capacity of the allocation (from the original allocation)
pub type FreeStringFn = unsafe extern "C" fn(*mut u8, usize);

/// Function signature for plugin configuration
///
/// # Parameters
/// - `config_json`: JSON configuration as byte array
/// - `config_len`: Length of config_json
///
/// # Returns
/// - 0 on success
/// - Non-zero error code on failure
pub type ConfigureFn = unsafe extern "C" fn(*const u8, usize) -> i32;

/// Function signature for plugin initialization
///
/// Called by the framework at the end of `handle_initialize`, after:
/// - Plugin library is loaded
/// - Configuration is set (via configure function if present)
/// - But before any tools are registered or called
///
/// The plugin should use this to:
/// - Validate configuration
/// - Initialize resources (database connections, caches, etc.)
/// - Perform any expensive setup operations
/// - Report initialization errors
///
/// # Parameters
/// - `error_msg_ptr`: Output pointer for error message (on failure)
/// - `error_msg_len`: Output length of error message (on failure)
///
/// # Returns
/// - 0 on success
/// - Non-zero error code on failure
///
/// If initialization fails, the plugin should allocate an error message,
/// write the pointer and length to the output parameters, and return non-zero.
/// The framework will call `free_string` to deallocate the error message.
pub type InitFn =
    unsafe extern "C" fn(error_msg_ptr: *mut *mut u8, error_msg_len: *mut usize) -> i32;

/// Function signature for getting plugin configuration schema
///
/// This function returns a JSON Schema describing the plugin's configuration structure.
/// It's used by clients to:
/// - Validate configuration before sending
/// - Generate UI for configuration
/// - Document configuration requirements
///
/// The schema should follow JSON Schema Draft 7 format.
///
/// # Parameters
/// - `schema_ptr`: Output pointer for schema JSON string
/// - `schema_len`: Output length of schema JSON string
///
/// # Returns
/// - 0 on success
/// - Non-zero if schema generation fails
///
/// The framework will call `free_string` to deallocate the schema string.
pub type GetConfigSchemaFn = unsafe extern "C" fn(
    schema_ptr: *mut *mut u8,
    schema_len: *mut usize,
) -> i32;

// ============================================================================
// Plugin Declaration
// ============================================================================

/// Plugin declaration exported by each plugin
///
/// This structure must be exported as a static with the name `plugin_declaration`.
/// Use the `declare_plugin!` macro for automatic version management.
#[repr(C)]
pub struct PluginDeclaration {
    /// MCP Plugin API version the plugin was built against (e.g., "0.1.0")
    ///
    /// This is automatically set from the mcp-plugin-api crate version.
    /// The C ABI is stable across Rust compiler versions, so only the API
    /// version matters for compatibility checking.
    pub api_version: *const u8,

    /// Returns list of tools as JSON array
    ///
    /// See [`ListToolsFn`] for details.
    pub list_tools: ListToolsFn,

    /// Execute a tool by name
    ///
    /// See [`ExecuteToolFn`] for details.
    pub execute_tool: ExecuteToolFn,

    /// Function to free memory allocated by the plugin
    ///
    /// See [`FreeStringFn`] for details.
    pub free_string: FreeStringFn,

    /// Optional configuration function called after plugin is loaded
    ///
    /// See [`ConfigureFn`] for details.
    pub configure: Option<ConfigureFn>,

    /// Optional initialization function called after configuration
    ///
    /// See [`InitFn`] for details.
    pub init: Option<InitFn>,

    /// Optional function to get configuration schema
    ///
    /// See [`GetConfigSchemaFn`] for details.
    pub get_config_schema: Option<GetConfigSchemaFn>,
}

// Safety: The static is initialized with constant values and never modified
unsafe impl Sync for PluginDeclaration {}

/// Current MCP Plugin API version (from Cargo.toml at compile time)
pub const API_VERSION: &str = env!("CARGO_PKG_VERSION");

/// API version as a null-terminated C string (for PluginDeclaration)
pub const API_VERSION_CSTR: &[u8] = concat!(env!("CARGO_PKG_VERSION"), "\0").as_bytes();

/// Helper macro to declare a plugin with automatic version management
///
/// # Example
///
/// ```ignore
/// use mcp_plugin_api::*;
///
/// // Minimal (no configuration, no init)
/// declare_plugin! {
///     list_tools: my_list_tools,
///     execute_tool: my_execute_tool,
///     free_string: my_free_string
/// }
///
/// // With configuration
/// declare_plugin! {
///     list_tools: my_list_tools,
///     execute_tool: my_execute_tool,
///     free_string: my_free_string,
///     configure: my_configure
/// }
///
/// // With configuration and init
/// declare_plugin! {
///     list_tools: my_list_tools,
///     execute_tool: my_execute_tool,
///     free_string: my_free_string,
///     configure: my_configure,
///     init: my_init
/// }
/// ```
#[macro_export]
macro_rules! declare_plugin {
    (
        list_tools: $list_fn:expr,
        execute_tool: $execute_fn:expr,
        free_string: $free_fn:expr
        $(, configure: $configure_fn:expr)?
        $(, init: $init_fn:expr)?
        $(, get_config_schema: $schema_fn:expr)?
    ) => {
        #[no_mangle]
        pub static plugin_declaration: $crate::PluginDeclaration = $crate::PluginDeclaration {
            api_version: $crate::API_VERSION_CSTR.as_ptr(),
            list_tools: $list_fn,
            execute_tool: $execute_fn,
            free_string: $free_fn,
            configure: $crate::__declare_plugin_option!($($configure_fn)?),
            init: $crate::__declare_plugin_option!($($init_fn)?),
            get_config_schema: $crate::__declare_plugin_option!($($schema_fn)?),
        };
    };
}

/// Helper macro for optional parameters in declare_plugin!
#[doc(hidden)]
#[macro_export]
macro_rules! __declare_plugin_option {
    ($value:expr) => {
        Some($value)
    };
    () => {
        None
    };
}

/// Declare a plugin initialization function with automatic wrapper generation
///
/// This macro takes a native Rust function and wraps it as an `extern "C"` function
/// that can be used in `declare_plugin!`. The native function should have the signature:
///
/// ```ignore
/// fn my_init() -> Result<(), String>
/// ```
///
/// The macro generates a C ABI wrapper function named `plugin_init` that:
/// - Calls your native init function
/// - Handles the FFI error reporting
/// - Returns appropriate error codes
///
/// # Example
///
/// ```ignore
/// use mcp_plugin_api::*;
/// use once_cell::sync::OnceCell;
///
/// static DB_POOL: OnceCell<DatabasePool> = OnceCell::new();
///
/// // Native Rust init function
/// fn init() -> Result<(), String> {
///     let config = get_config();
///     
///     // Initialize database
///     let pool = connect_to_db(&config.database_url)
///         .map_err(|e| format!("Failed to connect: {}", e))?;
///     
///     // Validate connection
///     pool.ping()
///         .map_err(|e| format!("DB ping failed: {}", e))?;
///     
///     DB_POOL.set(pool)
///         .map_err(|_| "DB already initialized".to_string())?;
///     
///     Ok(())
/// }
///
/// // Generate the C ABI wrapper
/// declare_plugin_init!(init);
///
/// // Use in plugin declaration
/// declare_plugin! {
///     list_tools: generated_list_tools,
///     execute_tool: generated_execute_tool,
///     free_string: utils::standard_free_string,
///     configure: plugin_configure,
///     init: plugin_init  // ← Generated by declare_plugin_init!
/// }
/// ```
#[macro_export]
macro_rules! declare_plugin_init {
    ($native_fn:ident) => {
        /// Auto-generated initialization function for plugin ABI
        ///
        /// This function is called by the framework after configuration
        /// and before any tools are registered or called.
        #[no_mangle]
        pub unsafe extern "C" fn plugin_init(
            error_msg_ptr: *mut *mut ::std::primitive::u8,
            error_msg_len: *mut ::std::primitive::usize,
        ) -> ::std::primitive::i32 {
            match $native_fn() {
                ::std::result::Result::Ok(_) => 0, // Success
                ::std::result::Result::Err(e) => {
                    $crate::utils::return_error(&e, error_msg_ptr, error_msg_len)
                }
            }
        }
    };
}

/// Declare configuration schema export with automatic generation
///
/// This macro generates an `extern "C"` function that exports the plugin's
/// configuration schema in JSON Schema format. It uses the `schemars` crate
/// to automatically generate the schema from your configuration struct.
///
/// The config type must derive `JsonSchema` from the `schemars` crate.
///
/// # Example
///
/// ```ignore
/// use mcp_plugin_api::*;
/// use serde::Deserialize;
/// use schemars::JsonSchema;
///
/// #[derive(Debug, Clone, Deserialize, JsonSchema)]
/// struct PluginConfig {
///     /// PostgreSQL connection URL
///     #[schemars(example = "example_db_url")]
///     database_url: String,
///     
///     /// Maximum database connections
///     #[schemars(range(min = 1, max = 100))]
///     #[serde(default = "default_max_connections")]
///     max_connections: u32,
/// }
///
/// fn example_db_url() -> &'static str {
///     "postgresql://user:pass@localhost:5432/dbname"
/// }
///
/// declare_plugin_config!(PluginConfig);
/// declare_config_schema!(PluginConfig);  // ← Generates plugin_get_config_schema
///
/// declare_plugin! {
///     list_tools: generated_list_tools,
///     execute_tool: generated_execute_tool,
///     free_string: utils::standard_free_string,
///     configure: plugin_configure,
///     get_config_schema: plugin_get_config_schema  // ← Use generated function
/// }
/// ```
#[macro_export]
macro_rules! declare_config_schema {
    ($config_type:ty) => {
        /// Auto-generated function to export configuration schema
        ///
        /// This function is called by the framework (via --get-plugin-schema)
        /// to retrieve the JSON Schema for this plugin's configuration.
        #[no_mangle]
        pub unsafe extern "C" fn plugin_get_config_schema(
            schema_ptr: *mut *mut ::std::primitive::u8,
            schema_len: *mut ::std::primitive::usize,
        ) -> ::std::primitive::i32 {
            use schemars::schema_for;
            
            let schema = schema_for!($config_type);
            let schema_json = match $crate::serde_json::to_string(&schema) {
                ::std::result::Result::Ok(s) => s,
                ::std::result::Result::Err(e) => {
                    ::std::eprintln!("Failed to serialize schema: {}", e);
                    return 1;
                }
            };
            
            // Convert to bytes and return using standard pattern
            let mut vec = schema_json.into_bytes();
            vec.shrink_to_fit();
            
            *schema_len = vec.capacity();
            *schema_ptr = vec.as_mut_ptr();
            let _ = ::std::mem::ManuallyDrop::new(vec);
            
            0 // Success
        }
    };
}

/// Declare plugin configuration with automatic boilerplate generation
///
/// This macro generates:
/// - Static storage for the configuration (`OnceCell`)
/// - `get_config()` function to access the configuration
/// - `try_get_config()` function for optional access
/// - `plugin_configure()` C ABI function for the framework
///
/// # Example
///
/// ```ignore
/// use mcp_plugin_api::*;
/// use serde::Deserialize;
///
/// #[derive(Debug, Clone, Deserialize)]
/// struct PluginConfig {
///     database_url: String,
///     max_connections: u32,
/// }
///
/// // Generate all configuration boilerplate
/// declare_plugin_config!(PluginConfig);
///
/// // Use in handlers
/// fn my_handler(args: &Value) -> Result<Value, String> {
///     let config = get_config();
///     // Use config.database_url, etc.
///     Ok(json!({"status": "ok"}))
/// }
///
/// declare_plugin! {
///     list_tools: generated_list_tools,
///     execute_tool: generated_execute_tool,
///     free_string: mcp_plugin_api::utils::standard_free_string,
///     configure: plugin_configure  // Auto-generated by declare_plugin_config!
/// }
/// ```
#[macro_export]
macro_rules! declare_plugin_config {
    ($config_type:ty) => {
        // Generate static storage
        static __PLUGIN_CONFIG: $crate::once_cell::sync::OnceCell<$config_type> =
            $crate::once_cell::sync::OnceCell::new();

        /// Get plugin configuration
        ///
        /// # Panics
        ///
        /// Panics if the plugin has not been configured yet. The framework calls
        /// `plugin_configure()` during plugin loading, so this should only panic
        /// if called before the plugin is fully loaded.
        pub fn get_config() -> &'static $config_type {
            __PLUGIN_CONFIG
                .get()
                .expect("Plugin not configured - configure() must be called first")
        }

        /// Try to get plugin configuration
        ///
        /// Returns `None` if the plugin has not been configured yet.
        /// Use this if you need to check configuration availability.
        pub fn try_get_config() -> ::std::option::Option<&'static $config_type> {
            __PLUGIN_CONFIG.get()
        }

        /// Auto-generated configuration function
        ///
        /// This function is called by the framework during plugin loading.
        /// It parses the JSON configuration and stores it in a static.
        ///
        /// # Returns
        /// - 0 on success
        /// - 1 on JSON parsing error
        /// - 2 if plugin is already configured
        #[no_mangle]
        pub unsafe extern "C" fn plugin_configure(
            config_json: *const ::std::primitive::u8,
            config_len: ::std::primitive::usize,
        ) -> ::std::primitive::i32 {
            // Parse configuration
            let config_slice = ::std::slice::from_raw_parts(config_json, config_len);
            let config: $config_type = match $crate::serde_json::from_slice(config_slice) {
                ::std::result::Result::Ok(c) => c,
                ::std::result::Result::Err(e) => {
                    ::std::eprintln!("Failed to parse plugin config: {}", e);
                    return 1; // Error code
                }
            };

            // Store globally
            if __PLUGIN_CONFIG.set(config).is_err() {
                ::std::eprintln!("Plugin already configured");
                return 2;
            }

            0 // Success
        }
    };
}
