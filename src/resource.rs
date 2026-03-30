//! Type-safe MCP resource definitions
//!
//! This module provides a high-level API for defining MCP resources with
//! compile-time type checking and automatic JSON generation for the
//! resources/list, resources/templates/list, and resources/read protocol messages.

use regex::Regex;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Handler for a URI template match: receives the request URI and extracted `{name}` values.
pub type TemplateResourceHandler =
    fn(uri: &str, vars: &HashMap<String, String>) -> Result<ResourceContents, String>;

/// Fallback handler when no static resource or template matches (user parses URI as needed).
pub type GenericResourceReadHandler = fn(&str) -> Result<ResourceContents, String>;

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

// --- URI template matching ({var} placeholders) -------------------------------------------

/// Compiles a uriTemplate into a regex and ordered variable names.
///
/// Each `{name}` becomes a capture group `([^/]+)` (path segment). Literal parts are
/// [`regex::escape`]-d so dots and other regex metacharacters match literally—aligned
/// with typical client-side template matching.
fn compile_uri_template(template: &str) -> Result<(Regex, Vec<String>), ()> {
    if !template.contains('{') {
        let re = Regex::new(&format!("^{}$", regex::escape(template))).map_err(|_| ())?;
        return Ok((re, Vec::new()));
    }

    let mut var_names = Vec::new();
    let mut pattern = String::from("^");
    let mut rest = template;

    while let Some(open) = rest.find('{') {
        let literal = &rest[..open];
        pattern.push_str(&regex::escape(literal));

        let after = &rest[open + 1..];
        let close = after.find('}').ok_or(())?;
        let name = after[..close].trim();
        if name.is_empty() {
            return Err(());
        }
        var_names.push(name.to_string());
        pattern.push_str("([^/]+)");
        rest = &after[close + 1..];
    }

    pattern.push_str(&regex::escape(rest));
    pattern.push('$');

    let re = Regex::new(&pattern).map_err(|_| ())?;
    Ok((re, var_names))
}

/// Match `uri` against a template string with `{var}` placeholders.
///
/// Uses the same idea as common MCP clients: compile a regex with `([^/]+)` per
/// placeholder and [`regex::escape`] for literals. Returns captured names → values, or
/// `None` if the URI does not match or the template is invalid.
///
/// **Note:** This compiles a fresh regex on every call. For repeated matching (e.g.
/// inside `declare_resources!`), the framework uses [`CompiledTemplateMatcher`] which
/// pre-compiles once at initialization. Use this function for ad-hoc / one-off matching.
///
/// Not full RFC 6570; use [`GenericResourceReadHandler`] as `read_fallback` for exotic cases.
pub fn match_uri_against_template(uri: &str, template: &str) -> Option<HashMap<String, String>> {
    let (re, var_names) = compile_uri_template(template).ok()?;
    let caps = re.captures(uri)?;

    if var_names.is_empty() {
        return Some(HashMap::new());
    }

    let mut vars = HashMap::new();
    for (i, name) in var_names.iter().enumerate() {
        let group = caps.get(i + 1)?.as_str();
        vars.insert(name.clone(), group.to_string());
    }
    Some(vars)
}

/// Pre-compiled URI template matcher. Built once at initialization time so that
/// `read_resource` dispatch only runs the regex (no compilation overhead per request).
///
/// Created via [`CompiledTemplateMatcher::new`] and stored in a `OnceLock` by the
/// [`declare_resources!`](crate::declare_resources) macro.
#[derive(Debug)]
pub struct CompiledTemplateMatcher {
    matcher: Regex,
    var_names: Vec<String>,
    /// The original template definition (metadata + handler).
    pub template: ResourceTemplate,
}

impl CompiledTemplateMatcher {
    /// Compile a [`ResourceTemplate`] into a matcher.
    ///
    /// Returns `Err` with a message if the URI template is syntactically invalid.
    pub fn new(template: ResourceTemplate) -> Result<Self, String> {
        let (matcher, var_names) = compile_uri_template(&template.uri_template)
            .map_err(|_| format!("invalid URI template: {}", template.uri_template))?;
        Ok(Self {
            matcher,
            var_names,
            template,
        })
    }

    /// Try to match a URI. Returns extracted `{name}` → value pairs, or `None`.
    pub fn match_uri(&self, uri: &str) -> Option<HashMap<String, String>> {
        let caps = self.matcher.captures(uri)?;
        let mut vars = HashMap::with_capacity(self.var_names.len());
        for (i, name) in self.var_names.iter().enumerate() {
            let group = caps.get(i + 1)?.as_str();
            vars.insert(name.clone(), group.to_string());
        }
        Some(vars)
    }
}

// --- Resource templates (resources/templates/list) ----------------------------------------

/// Parameterized resource definition for `resources/templates/list` and template-based reads.
#[derive(Debug, Clone)]
pub struct ResourceTemplate {
    /// URI template (e.g. `file:///project/{path}`)
    pub uri_template: String,
    pub name: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub mime_type: Option<String>,
    pub handler: TemplateResourceHandler,
}

impl ResourceTemplate {
    pub fn builder(
        uri_template: impl Into<String>,
        handler: TemplateResourceHandler,
    ) -> ResourceTemplateBuilder {
        ResourceTemplateBuilder {
            uri_template: uri_template.into(),
            name: None,
            title: None,
            description: None,
            mime_type: None,
            handler,
        }
    }

    /// MCP `resourceTemplates` list item: `uriTemplate`, `name`, optional fields.
    pub fn to_template_list_item(&self) -> Value {
        let mut obj = serde_json::Map::new();
        obj.insert("uriTemplate".to_string(), json!(self.uri_template));
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

pub struct ResourceTemplateBuilder {
    uri_template: String,
    name: Option<String>,
    title: Option<String>,
    description: Option<String>,
    mime_type: Option<String>,
    handler: TemplateResourceHandler,
}

impl ResourceTemplateBuilder {
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }

    pub fn build(self) -> ResourceTemplate {
        ResourceTemplate {
            uri_template: self.uri_template,
            name: self.name,
            title: self.title,
            description: self.description,
            mime_type: self.mime_type,
            handler: self.handler,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{match_uri_against_template, CompiledTemplateMatcher, ResourceTemplate};
    use std::collections::HashMap;

    fn dummy_handler(_uri: &str, _vars: &HashMap<String, String>) -> Result<super::ResourceContents, String> {
        Ok(vec![])
    }

    #[test]
    fn match_uri_template_trailing_placeholder() {
        let m = match_uri_against_template("file:///docs/readme.md", "file:///docs/{path}").unwrap();
        assert_eq!(m.get("path").map(|s| s.as_str()), Some("readme.md"));
    }

    #[test]
    fn match_uri_template_two_segments() {
        let m = match_uri_against_template(
            "https://example.com/foo",
            "https://{host}/{path}",
        )
        .unwrap();
        assert_eq!(m.get("host").map(|s| s.as_str()), Some("example.com"));
        assert_eq!(m.get("path").map(|s| s.as_str()), Some("foo"));
    }

    #[test]
    fn match_uri_template_multi_segment_path_needs_more_placeholders() {
        assert!(
            match_uri_against_template("https://example.com/foo/bar", "https://{host}/{path}")
                .is_none(),
            "two placeholders => two segments; leftover /bar does not match"
        );
        let m = match_uri_against_template(
            "https://example.com/foo/bar",
            "https://{host}/{a}/{b}",
        )
        .unwrap();
        assert_eq!(m.get("a").map(|s| s.as_str()), Some("foo"));
        assert_eq!(m.get("b").map(|s| s.as_str()), Some("bar"));
    }

    #[test]
    fn compiled_matcher_reuses_regex() {
        let tmpl = ResourceTemplate::builder("gmc:///reading/{token}", dummy_handler)
            .name("readings")
            .build();
        let matcher = CompiledTemplateMatcher::new(tmpl).unwrap();

        let m1 = matcher.match_uri("gmc:///reading/abc123").unwrap();
        assert_eq!(m1.get("token").map(|s| s.as_str()), Some("abc123"));

        let m2 = matcher.match_uri("gmc:///reading/xyz789").unwrap();
        assert_eq!(m2.get("token").map(|s| s.as_str()), Some("xyz789"));

        assert!(matcher.match_uri("gmc:///other/abc").is_none());
    }

    #[test]
    fn compiled_matcher_dots_in_literal_are_exact() {
        let tmpl = ResourceTemplate::builder("https://api.example.com/{id}", dummy_handler)
            .build();
        let matcher = CompiledTemplateMatcher::new(tmpl).unwrap();

        assert!(matcher.match_uri("https://api.example.com/42").is_some());
        assert!(
            matcher.match_uri("https://apiXexampleYcom/42").is_none(),
            "dots in the literal must match literally, not as regex wildcards"
        );
    }

    #[test]
    fn compiled_matcher_invalid_template() {
        let tmpl = ResourceTemplate::builder("bad://{unclosed", dummy_handler).build();
        assert!(CompiledTemplateMatcher::new(tmpl).is_err());
    }
}
