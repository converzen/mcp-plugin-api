//! Memory management utilities for plugins
//!
//! This module provides safe wrappers around the unsafe FFI memory management
//! operations required by the plugin API.

use serde_json::Value;
use std::mem::ManuallyDrop;

/// Return a success result to the framework
///
/// This handles all the unsafe memory management details:
/// - Converts the value to JSON
/// - Allocates a buffer
/// - Shrinks to minimize memory usage
/// - Returns the pointer and capacity to the framework
///
/// # Safety
///
/// The caller must ensure that:
/// - `result_buf` points to valid, properly aligned memory for writing a pointer
/// - `result_len` points to valid, properly aligned memory for writing a usize
/// - These pointers remain valid for the duration of the call
/// - The pointers are not aliased (no other mutable references exist)
///
/// # Example
///
/// ```ignore
/// unsafe {
///     let result = json!({"status": "ok"});
///     return return_success(result, result_buf, result_len);
/// }
/// ```
pub unsafe fn return_success(data: Value, result_buf: *mut *mut u8, result_len: *mut usize) -> i32 {
    prepare_result(data, result_buf, result_len);

    0 // Success code
}

/// Return an error result to the framework
///
/// This wraps the error message in a JSON object and returns it
/// with an error code.
///
/// # Safety
///
/// The caller must ensure that:
/// - `result_buf` points to valid, properly aligned memory for writing a pointer
/// - `result_len` points to valid, properly aligned memory for writing a usize
/// - These pointers remain valid for the duration of the call
/// - The pointers are not aliased (no other mutable references exist)
///
/// # Example
///
/// ```ignore
/// unsafe {
///     return return_error("Product not found", result_buf, result_len);
/// }
/// ```
pub unsafe fn return_error(error: &str, result_buf: *mut *mut u8, result_len: *mut usize) -> i32 {
    let error_json = serde_json::json!({
        "error": error
    });

    prepare_result(error_json, result_buf, result_len);

    1 // Error code
}

/// Prepare a result for return to the framework
///
/// Internal helper function that handles the common memory management
/// for both success and error results.
///
/// # Safety
///
/// The caller must ensure that:
/// - `result_buf` points to valid, properly aligned memory for writing a pointer
/// - `result_len` points to valid, properly aligned memory for writing a usize
/// - These pointers remain valid for the duration of the call
/// - The pointers are not aliased (no other mutable references exist)
pub unsafe fn prepare_result(data: Value, result_buf: *mut *mut u8, result_len: *mut usize) {
    let json_string = data.to_string();
    let mut vec = json_string.into_bytes();
    vec.shrink_to_fit();

    *result_len = vec.capacity();
    *result_buf = vec.as_mut_ptr();
    let _ = ManuallyDrop::new(vec);
}

/// Standard free_string implementation
///
/// This can be used directly in the `declare_plugin!` macro.
/// It safely deallocates memory that was allocated by the plugin
/// and passed to the framework.
///
/// # Safety
///
/// The pointer and capacity must match the values returned by
/// `return_success` or `return_error`.
///
/// # Example
///
/// ```ignore
/// declare_plugin! {
///     list_tools: generated_list_tools,
///     execute_tool: generated_execute_tool,
///     free_string: mcp_plugin_api::utils::standard_free_string
/// }
/// ```
pub unsafe extern "C" fn standard_free_string(ptr: *mut u8, capacity: usize) {
    if !ptr.is_null() && capacity > 0 {
        // Reconstruct the Vec with the same capacity that was returned
        let _ = Vec::from_raw_parts(ptr, capacity, capacity);
        // Vec is dropped here, freeing the memory
    }
}

// ============================================================================
// Content Helpers - MCP-compliant content construction
// ============================================================================

/// Helper to create a text content response
///
/// Creates a standard MCP text content response:
/// ```json
/// {
///   "content": [{
///     "type": "text",
///     "text": "your text here"
///   }]
/// }
/// ```
///
/// # Example
///
/// ```ignore
/// fn handle_get_price(args: &Value) -> Result<Value, String> {
///     let price = 29.99;
///     Ok(text_content(format!("Price: ${:.2}", price)))
/// }
/// ```
pub fn text_content(text: impl Into<String>) -> Value {
    serde_json::json!({
        "content": [{
            "type": "text",
            "text": text.into()
        }]
    })
}

/// Helper to create a JSON content response
///
/// Creates a standard MCP JSON content response with structured data:
/// ```json
/// {
///   "content": [{
///     "type": "json",
///     "json": { ... }
///   }]
/// }
/// ```
///
/// **Use case**: Structured data for programmatic clients
///
/// # Example
///
/// ```ignore
/// fn handle_get_product(args: &Value) -> Result<Value, String> {
///     let product = get_product_from_db()?;
///     Ok(json_content(serde_json::to_value(product)?))
/// }
/// ```
pub fn json_content(json: Value) -> Value {
    serde_json::json!({
        "content": [{
            "type": "json",
            "json": json
        }]
    })
}

/// Helper to create an HTML content response
///
/// Creates a standard MCP HTML content response:
/// ```json
/// {
///   "content": [{
///     "type": "html",
///     "html": "<div>...</div>"
///   }]
/// }
/// ```
///
/// **Use case**: Rich HTML content for UIs
///
/// # Example
///
/// ```ignore
/// fn handle_get_formatted(args: &Value) -> Result<Value, String> {
///     let html = format!("<div><h1>{}</h1><p>{}</p></div>", title, body);
///     Ok(html_content(html))
/// }
/// ```
pub fn html_content(html: impl Into<String>) -> Value {
    serde_json::json!({
        "content": [{
            "type": "html",
            "html": html.into()
        }]
    })
}

/// Helper to create a Markdown content response
///
/// Creates a standard MCP Markdown content response:
/// ```json
/// {
///   "content": [{
///     "type": "markdown",
///     "markdown": "# Title\n\nContent..."
///   }]
/// }
/// ```
///
/// **Use case**: Formatted text for chat clients
///
/// # Example
///
/// ```ignore
/// fn handle_get_readme(args: &Value) -> Result<Value, String> {
///     let markdown = format!("# {}\n\n{}", title, content);
///     Ok(markdown_content(markdown))
/// }
/// ```
pub fn markdown_content(markdown: impl Into<String>) -> Value {
    serde_json::json!({
        "content": [{
            "type": "markdown",
            "markdown": markdown.into()
        }]
    })
}

/// Helper to create an image content response with URL
///
/// Creates a standard MCP image content response with image URL:
/// ```json
/// {
///   "content": [{
///     "type": "image",
///     "imageUrl": "https://example.com/image.png",
///     "mimeType": "image/png"
///   }]
/// }
/// ```
///
/// **Use case**: Return image by URL reference
///
/// # Example
///
/// ```ignore
/// fn handle_get_product_image(args: &Value) -> Result<Value, String> {
///     let url = format!("https://cdn.example.com/products/{}.jpg", product_id);
///     Ok(image_url_content(url, Some("image/jpeg".to_string())))
/// }
/// ```
pub fn image_url_content(url: impl Into<String>, mime_type: Option<String>) -> Value {
    let mut img = serde_json::json!({
        "type": "image",
        "imageUrl": url.into()
    });
    
    if let Some(mt) = mime_type {
        img["mimeType"] = serde_json::json!(mt);
    }
    
    serde_json::json!({
        "content": [img]
    })
}

/// Helper to create an image content response with base64 data
///
/// Creates a standard MCP image content response with embedded data:
/// ```json
/// {
///   "content": [{
///     "type": "image",
///     "imageData": "base64-encoded-data",
///     "mimeType": "image/png"
///   }]
/// }
/// ```
///
/// **Use case**: Return embedded image data
///
/// # Example
///
/// ```ignore
/// fn handle_get_chart(args: &Value) -> Result<Value, String> {
///     let chart_bytes = generate_chart()?;
///     let base64_data = base64::encode(chart_bytes);
///     Ok(image_data_content(base64_data, Some("image/png".to_string())))
/// }
/// ```
pub fn image_data_content(data: impl Into<String>, mime_type: Option<String>) -> Value {
    let mut img = serde_json::json!({
        "type": "image",
        "imageData": data.into()
    });
    
    if let Some(mt) = mime_type {
        img["mimeType"] = serde_json::json!(mt);
    }
    
    serde_json::json!({
        "content": [img]
    })
}

/// Helper to create an image content response (legacy)
///
/// **DEPRECATED**: Use `image_url_content` or `image_data_content` instead.
///
/// This function is kept for backward compatibility but uses the old field name.
#[deprecated(since = "0.2.0", note = "Use image_url_content or image_data_content instead")]
pub fn image_content(data: impl Into<String>, mime_type: impl Into<String>) -> Value {
    image_data_content(data, Some(mime_type.into()))
}

/// Helper to create a resource content response
///
/// Creates a standard MCP resource content response:
/// ```json
/// {
///   "content": [{
///     "type": "resource",
///     "uri": "https://example.com/resource",
///     "mimeType": "text/html",  // optional
///     "text": "content"         // optional
///   }]
/// }
/// ```
///
/// # Example
///
/// ```ignore
/// fn handle_get_resource(args: &Value) -> Result<Value, String> {
///     Ok(resource_content(
///         "https://example.com/docs",
///         Some("text/html".to_string()),
///         None
///     ))
/// }
/// ```
pub fn resource_content(
    uri: impl Into<String>,
    mime_type: Option<String>,
    text: Option<String>,
) -> Value {
    let mut res = serde_json::json!({
        "type": "resource",
        "uri": uri.into()
    });

    if let Some(mt) = mime_type {
        res["mimeType"] = serde_json::json!(mt);
    }
    if let Some(t) = text {
        res["text"] = serde_json::json!(t);
    }

    serde_json::json!({
        "content": [res]
    })
}

/// Helper to create a multi-content response
///
/// Creates a response with multiple content items (text, images, resources):
/// ```json
/// {
///   "content": [
///     {"type": "text", "text": "..."},
///     {"type": "image", "data": "...", "mimeType": "..."},
///     {"type": "resource", "uri": "..."}
///   ]
/// }
/// ```
///
/// # Example
///
/// ```ignore
/// fn handle_get_product(args: &Value) -> Result<Value, String> {
///     Ok(multi_content(vec![
///         serde_json::json!({"type": "text", "text": "Product info"}),
///         serde_json::json!({
///             "type": "image",
///             "data": base64_image,
///             "mimeType": "image/jpeg"
///         })
///     ]))
/// }
/// ```
pub fn multi_content(items: Vec<Value>) -> Value {
    serde_json::json!({
        "content": items
    })
}
