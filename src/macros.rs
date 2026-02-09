//! Macros for simplifying plugin declaration

/// Declare tools and auto-generate list_tools and execute_tool functions
///
/// This macro takes a list of Tool definitions and generates:
/// - A static tool registry (HashMap for O(1) lookup)
/// - The `generated_list_tools` function
/// - The `generated_execute_tool` function
///
/// These generated functions can be used directly in the `declare_plugin!` macro.
///
/// # Example
///
/// ```ignore
/// use mcp_plugin_api::*;
/// use serde_json::{json, Value};
///
/// fn handle_hello(args: &Value) -> Result<Value, String> {
///     let name = args["name"].as_str().unwrap_or("World");
///     Ok(json!({ "message": format!("Hello, {}!", name) }))
/// }
///
/// fn handle_goodbye(args: &Value) -> Result<Value, String> {
///     Ok(json!({ "message": "Goodbye!" }))
/// }
///
/// declare_tools! {
///     tools: [
///         Tool::new("hello", "Say hello")
///             .param_string("name", "Name to greet", false)
///             .handler(handle_hello),
///         
///         Tool::new("goodbye", "Say goodbye")
///             .handler(handle_goodbye),
///     ]
/// }
///
/// declare_plugin! {
///     list_tools: generated_list_tools,
///     execute_tool: generated_execute_tool,
///     free_string: mcp_plugin_api::utils::standard_free_string
/// }
/// ```
#[macro_export]
macro_rules! declare_tools {
    (tools: [ $($tool:expr),* $(,)? ]) => {
        // Generate a static HashMap of tools using OnceLock for thread-safe lazy init
        static TOOLS: ::std::sync::OnceLock<::std::collections::HashMap<::std::string::String, $crate::tool::Tool>> 
            = ::std::sync::OnceLock::new();
        
        fn get_tools() -> &'static ::std::collections::HashMap<::std::string::String, $crate::tool::Tool> {
            TOOLS.get_or_init(|| {
                let mut map = ::std::collections::HashMap::new();
                $(
                    let tool = $tool;
                    map.insert(tool.name.clone(), tool);
                )*
                map
            })
        }
        
        /// Auto-generated list_tools function
        ///
        /// Returns a JSON array of all tool definitions.
        #[no_mangle]
        pub unsafe extern "C" fn generated_list_tools(
            result_buf: *mut *mut u8,
            result_len: *mut usize,
        ) -> i32 {
            let tools = get_tools();
            let tools_json: ::std::vec::Vec<$crate::serde_json::Value> = tools
                .values()
                .map(|t| t.to_json_schema())
                .collect();
            
            let json_array = $crate::serde_json::Value::Array(tools_json);
            $crate::utils::return_success(json_array, result_buf, result_len)
        }
        
        /// Auto-generated execute_tool function
        ///
        /// Dispatches to the appropriate tool handler based on the tool name.
        #[no_mangle]
        pub unsafe extern "C" fn generated_execute_tool(
            tool_name: *const ::std::os::raw::c_char,
            args_json: *const u8,
            args_len: usize,
            result_buf: *mut *mut u8,
            result_len: *mut usize,
        ) -> i32 {
            use ::std::ffi::CStr;
            
            // Parse tool name
            let name = match CStr::from_ptr(tool_name).to_str() {
                Ok(s) => s,
                Err(_) => return $crate::utils::return_error(
                    "Invalid tool name encoding",
                    result_buf,
                    result_len
                ),
            };
            
            // Parse arguments
            let args_slice = ::std::slice::from_raw_parts(args_json, args_len);
            let args: $crate::serde_json::Value = match $crate::serde_json::from_slice(args_slice) {
                Ok(v) => v,
                Err(e) => return $crate::utils::return_error(
                    &format!("Invalid JSON arguments: {}", e),
                    result_buf,
                    result_len
                ),
            };
            
            // Find and execute the tool (O(1) HashMap lookup!)
            let tools = get_tools();
            match tools.get(name) {
                Some(tool) => {
                    match (tool.handler)(&args) {
                        Ok(result) => $crate::utils::return_success(
                            result,
                            result_buf,
                            result_len
                        ),
                        Err(e) => $crate::utils::return_error(
                            &e,
                            result_buf,
                            result_len
                        ),
                    }
                }
                None => $crate::utils::return_error(
                    &format!("Unknown tool: {}", name),
                    result_buf,
                    result_len
                ),
            }
        }
    };
}
