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
        static TOOLS: ::std::sync::OnceLock<::std::collections::HashMap<::std::string::String, $crate::tool::Tool>>
            = ::std::sync::OnceLock::new();
        static TOOLS_LIST_CACHE: ::std::sync::OnceLock<::std::vec::Vec<u8>>
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

        fn get_tools_list_cache() -> &'static [u8] {
            TOOLS_LIST_CACHE.get_or_init(|| {
                let tools = get_tools();
                let tools_json: ::std::vec::Vec<$crate::serde_json::Value> = tools
                    .values()
                    .filter(|t| t.active)
                    .map(|t| t.to_json_schema())
                    .collect();
                let json_array = $crate::serde_json::Value::Array(tools_json);
                json_array.to_string().into_bytes()
            })
        }

        /// Auto-generated list_tools — returns pre-serialized JSON (one memcpy).
        #[no_mangle]
        pub unsafe extern "C" fn generated_list_tools(
            result_buf: *mut *mut u8,
            result_len: *mut usize,
        ) -> i32 {
            $crate::utils::return_prebuilt(get_tools_list_cache(), result_buf, result_len)
        }

        /// Auto-generated execute_tool — O(1) HashMap lookup then handler dispatch.
        #[no_mangle]
        pub unsafe extern "C" fn generated_execute_tool(
            tool_name: *const ::std::os::raw::c_char,
            args_json: *const u8,
            args_len: usize,
            result_buf: *mut *mut u8,
            result_len: *mut usize,
        ) -> i32 {
            use ::std::ffi::CStr;

            let name = match CStr::from_ptr(tool_name).to_str() {
                Ok(s) => s,
                Err(_) => return $crate::utils::return_error(
                    "Invalid tool name encoding",
                    result_buf,
                    result_len
                ),
            };

            let args_slice = ::std::slice::from_raw_parts(args_json, args_len);
            let args: $crate::serde_json::Value = match $crate::serde_json::from_slice(args_slice) {
                Ok(v) => v,
                Err(e) => return $crate::utils::return_error(
                    &format!("Invalid JSON arguments: {}", e),
                    result_buf,
                    result_len
                ),
            };

            let tools = get_tools();
            match tools.get(name) {
                Some(tool) => {
                    if tool.active {
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
                    } else {
                        $crate::utils::return_error(
                            &format!("Inactive tool: {}", name),
                            result_buf,
                            result_len)
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

/// Declare resources and auto-generate list_resources, list_resource_templates, and read_resource
///
/// Dispatches `read_resource` in order: exact static URI, first matching URI template
/// (`{var}` placeholders), then optional `read_fallback`.
///
/// # Example (static resources only)
///
/// ```ignore
/// declare_resources! {
///     resources: [
///         Resource::builder("file:///docs/readme", read_readme)
///             .name("readme.md")
///             .build(),
///     ]
/// }
/// ```
///
/// # Example (templates + fallback)
///
/// ```ignore
/// fn read_file(uri: &str, vars: &HashMap<String, String>) -> Result<ResourceContents, String> {
///     let path = vars.get("path").ok_or("missing path")?;
///     Ok(vec![ResourceContent::text(uri, contents, Some("text/plain".into()))])
/// }
///
/// fn read_any(uri: &str) -> Result<ResourceContents, String> {
///     Err("not found".into())
/// }
///
/// declare_resources! {
///     resources: [],
///     templates: [
///         ResourceTemplate::builder("file:///project/{path}", read_file)
///             .name("project-files")
///             .build(),
///     ],
///     read_fallback: read_any
/// }
/// ```
#[macro_export]
macro_rules! declare_resources {
    (resources: [ $($resource:expr),* $(,)? ]) => {
        $crate::declare_resources!(@impl
            resources: [$($resource),*],
            templates: [],
            read_fallback: []
        );
    };
    (
        resources: [ $($resource:expr),* $(,)? ],
        templates: [ $($template:expr),* $(,)? ]
    ) => {
        $crate::declare_resources!(@impl
            resources: [$($resource),*],
            templates: [$($template),*],
            read_fallback: []
        );
    };
    (
        resources: [ $($resource:expr),* $(,)? ],
        templates: [ $($template:expr),* $(,)? ],
        read_fallback: $fallback:expr
    ) => {
        $crate::declare_resources!(@impl
            resources: [$($resource),*],
            templates: [$($template),*],
            read_fallback: [$fallback]
        );
    };

    (@impl resources: [$($resource:expr),*], templates: [$($template:expr),*], read_fallback: [$($fallback:tt)*]) => {
        static RESOURCES: ::std::sync::OnceLock<::std::collections::HashMap<::std::string::String, $crate::resource::Resource>>
            = ::std::sync::OnceLock::new();
        static COMPILED_TEMPLATE_MATCHERS: ::std::sync::OnceLock<::std::vec::Vec<$crate::resource::CompiledTemplateMatcher>>
            = ::std::sync::OnceLock::new();
        static RESOURCES_LIST_CACHE: ::std::sync::OnceLock<::std::vec::Vec<u8>>
            = ::std::sync::OnceLock::new();
        static TEMPLATES_LIST_CACHE: ::std::sync::OnceLock<::std::vec::Vec<u8>>
            = ::std::sync::OnceLock::new();

        fn get_resources() -> &'static ::std::collections::HashMap<::std::string::String, $crate::resource::Resource> {
            RESOURCES.get_or_init(|| {
                let mut map = ::std::collections::HashMap::new();
                $(
                    let resource = $resource;
                    map.insert(resource.uri.clone(), resource);
                )*
                map
            })
        }

        fn get_template_matchers() -> &'static ::std::vec::Vec<$crate::resource::CompiledTemplateMatcher> {
            COMPILED_TEMPLATE_MATCHERS.get_or_init(|| {
                vec![$($template),*]
                    .into_iter()
                    .map(|t| $crate::resource::CompiledTemplateMatcher::new(t)
                        .expect("invalid URI template in declare_resources!"))
                    .collect()
            })
        }

        fn get_resources_list_cache() -> &'static [u8] {
            RESOURCES_LIST_CACHE.get_or_init(|| {
                let resources = get_resources();
                let items: ::std::vec::Vec<$crate::serde_json::Value> = resources
                    .values()
                    .map(|r| r.to_list_item())
                    .collect();
                $crate::utils::resource_list_response(items, None)
                    .to_string().into_bytes()
            })
        }

        fn get_templates_list_cache() -> &'static [u8] {
            TEMPLATES_LIST_CACHE.get_or_init(|| {
                let matchers = get_template_matchers();
                let items: ::std::vec::Vec<$crate::serde_json::Value> = matchers
                    .iter()
                    .map(|m| m.template.to_template_list_item())
                    .collect();
                $crate::utils::resource_template_list_response(items, None)
                    .to_string().into_bytes()
            })
        }

        fn read_fallback_handler() -> ::std::option::Option<$crate::resource::GenericResourceReadHandler> {
            $crate::__declare_plugin_option!($($fallback)*)
        }

        $crate::declare_resources!(@generated_functions);
    };

    (@generated_functions) => {
        /// Auto-generated list_resources — returns pre-serialized JSON (one memcpy).
        #[no_mangle]
        pub unsafe extern "C" fn generated_list_resources(
            result_buf: *mut *mut u8,
            result_len: *mut usize,
        ) -> i32 {
            $crate::utils::return_prebuilt(get_resources_list_cache(), result_buf, result_len)
        }

        /// Auto-generated list_resource_templates — returns pre-serialized JSON (one memcpy).
        #[no_mangle]
        pub unsafe extern "C" fn generated_list_resource_templates(
            result_buf: *mut *mut u8,
            result_len: *mut usize,
        ) -> i32 {
            $crate::utils::return_prebuilt(get_templates_list_cache(), result_buf, result_len)
        }

        /// Auto-generated read_resource function
        ///
        /// Dispatches: exact static URI, then pre-compiled URI template matchers,
        /// then optional read_fallback. Template regexes are compiled once (via
        /// `OnceLock`) on the first call, not per request.
        #[no_mangle]
        pub unsafe extern "C" fn generated_read_resource(
            uri_ptr: *const u8,
            uri_len: usize,
            result_buf: *mut *mut u8,
            result_len: *mut usize,
        ) -> i32 {
            let uri_slice = ::std::slice::from_raw_parts(uri_ptr, uri_len);
            let uri = match ::std::str::from_utf8(uri_slice) {
                Ok(s) => s,
                Err(_) => return $crate::utils::return_error(
                    "Invalid URI encoding",
                    result_buf,
                    result_len
                ),
            };

            let resources = get_resources();
            if let Some(resource) = resources.get(uri) {
                return match (resource.handler)(uri) {
                    Ok(contents) => {
                        let response = $crate::utils::resource_read_response(&contents);
                        $crate::utils::return_success(response, result_buf, result_len)
                    }
                    Err(e) => $crate::utils::return_error(&e, result_buf, result_len),
                };
            }

            for matcher in get_template_matchers() {
                if let Some(vars) = matcher.match_uri(uri) {
                    return match (matcher.template.handler)(uri, &vars) {
                        Ok(contents) => {
                            let response = $crate::utils::resource_read_response(&contents);
                            $crate::utils::return_success(response, result_buf, result_len)
                        }
                        Err(e) => $crate::utils::return_error(&e, result_buf, result_len),
                    };
                }
            }

            if let Some(fallback) = read_fallback_handler() {
                return match fallback(uri) {
                    Ok(contents) => {
                        let response = $crate::utils::resource_read_response(&contents);
                        $crate::utils::return_success(response, result_buf, result_len)
                    }
                    Err(e) => $crate::utils::return_error(&e, result_buf, result_len),
                };
            }

            $crate::utils::return_error(
                &format!("Unknown resource: {}", uri),
                result_buf,
                result_len
            )
        }
    };
}
