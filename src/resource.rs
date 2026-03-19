//! Type-safe MCP resource definitions
//!
//! This module provides a high-level API for defining MCP resources with
//! compile-time type checking and automatic JSON generation for the
//! resources/list and resources/read protocol messages.

use serde_json::{json, Value};

/// Content of a resource, either text or binary (base64-encoded).
///
/// Matches the MCP resources/read response format:
/// - Text content: `{ "uri", "mimeType?", "text" }`
/// - Binary content: `{ "uri", "mimeType?", "blob" }` (base64)
#[derive(Debug, Clone)]
pub struct ResourceContent {
    /// Resource URI
    pub uri: String,
    /// Optional MIME type
    pub mime_type: Option<String>,
    /// Text content (use for UTF-8 text)
    pub text: Option<String>,
    /// Base64-encoded binary content (use for binary data)
    pub blob: Option<String>,
}

impl ResourceContent {
    /// Create text content
    pub fn text(uri: impl Into<String>, content: impl Into<String>, mime_type: Option<String>) -> Self {
        Self {
            uri: uri.into(),
            mime_type,
            text: Some(content.into()),
            blob: None,
        }
    }

    /// Create binary content (base64-encoded)
    pub fn blob(uri: impl Into<String>, base64_data: impl Into<String>, mime_type: Option<String>) -> Self {
        Self {
            uri: uri.into(),
            mime_type,
            text: None,
            blob: Some(base64_data.into()),
        }
    }

    /// Convert to MCP content block JSON
    pub fn to_json(&self) -> Value {
        let mut obj = serde_json::Map::new();
        obj.insert("uri".to_string(), json!(self.uri));
        if let Some(ref mt) = self.mime_type {
            obj.insert("mimeType".to_string(), json!(mt));
        }
        if let Some(ref t) = self.text {
            obj.insert("text".to_string(), json!(t));
        }
        if let Some(ref b) = self.blob {
            obj.insert("blob".to_string(), json!(b));
        }
        Value::Object(obj)
    }
}

/// Collection of resource contents (e.g. multi-part resource)
pub type ResourceContents = Vec<ResourceContent>;

/// Resource read handler function type
///
/// Given a URI, returns the resource contents or an error.
pub type ResourceHandler = fn(&str) -> Result<ResourceContents, String>;

/// A resource definition for the resources/list response
///
/// Represents a single resource with its metadata. The handler is called
/// when the client requests the resource via resources/read.
#[derive(Debug, Clone)]
pub struct Resource {
    /// Unique resource URI
    pub uri: String,
    /// Resource name (e.g. filename)
    pub name: Option<String>,
    /// Human-readable title for display
    pub title: Option<String>,
    /// Description of the resource
    pub description: Option<String>,
    /// Optional MIME type
    pub mime_type: Option<String>,
    /// Handler to read resource content when requested
    pub handler: ResourceHandler,
}

impl Resource {
    /// Create a new resource with a builder
    ///
    /// # Example
    ///
    /// ```ignore
    /// Resource::builder("file:///docs/readme", read_readme)
    ///     .name("readme.md")
    ///     .description("Project documentation")
    ///     .mime_type("text/markdown")
    /// ```
    pub fn builder(uri: impl Into<String>, handler: ResourceHandler) -> ResourceBuilder {
        ResourceBuilder {
            uri: uri.into(),
            name: None,
            title: None,
            description: None,
            mime_type: None,
            handler,
        }
    }

    /// Convert to MCP resources/list item format
    ///
    /// Returns a JSON object compatible with MCP protocol:
    /// ```json
    /// {
    ///   "uri": "file:///...",
    ///   "name": "main.rs",
    ///   "title": "Optional title",
    ///   "description": "Optional description",
    ///   "mimeType": "text/x-rust"
    /// }
    /// ```
    pub fn to_list_item(&self) -> Value {
        let mut obj = serde_json::Map::new();
        obj.insert("uri".to_string(), json!(self.uri));
        if let Some(ref n) = self.name {
            obj.insert("name".to_string(), json!(n));
        }
        if let Some(ref t) = self.title {
            obj.insert("title".to_string(), json!(t));
        }
        if let Some(ref d) = self.description {
            obj.insert("description".to_string(), json!(d));
        }
        if let Some(ref mt) = self.mime_type {
            obj.insert("mimeType".to_string(), json!(mt));
        }
        Value::Object(obj)
    }
}

/// Builder for creating resources with a fluent API
pub struct ResourceBuilder {
    uri: String,
    name: Option<String>,
    title: Option<String>,
    description: Option<String>,
    mime_type: Option<String>,
    handler: ResourceHandler,
}

impl ResourceBuilder {
    /// Set the resource name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the human-readable title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the MIME type
    pub fn mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    /// Finalize and build the Resource
    pub fn build(self) -> Resource {
        Resource {
            uri: self.uri,
            name: self.name,
            title: self.title,
            description: self.description,
            mime_type: self.mime_type,
            handler: self.handler,
        }
    }
}
