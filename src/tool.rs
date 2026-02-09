//! Type-safe tool definitions
//!
//! This module provides a high-level API for defining tools with
//! compile-time type checking and automatic JSON schema generation.

use serde_json::{json, Value};

/// A parameter definition for a tool
#[derive(Debug, Clone)]
pub struct ToolParam {
    pub name: String,
    pub description: String,
    pub param_type: ParamType,
    pub required: bool,
}

/// Parameter type enumeration
#[derive(Debug, Clone)]
pub enum ParamType {
    String,
    Integer,
    Number,
    Boolean,
    Object,
    Array,
}

impl ParamType {
    /// Convert to JSON Schema type string
    pub fn to_json_type(&self) -> &'static str {
        match self {
            ParamType::String => "string",
            ParamType::Integer => "integer",
            ParamType::Number => "number",
            ParamType::Boolean => "boolean",
            ParamType::Object => "object",
            ParamType::Array => "array",
        }
    }
}

/// Tool handler function type
///
/// A tool handler takes JSON arguments and returns either a JSON result
/// or an error message.
pub type ToolHandler = fn(&Value) -> Result<Value, String>;

/// A tool definition
///
/// This represents a single tool with its metadata and handler function.
pub struct Tool {
    pub name: String,
    pub description: String,
    pub params: Vec<ToolParam>,
    pub handler: ToolHandler,
}

impl Tool {
    /// Create a new tool definition with a builder
    ///
    /// # Example
    ///
    /// ```ignore
    /// Tool::new("get_price", "Get the price of a product")
    ///     .param_i64("product_id", "The product ID", true)
    ///     .handler(handle_get_price)
    /// ```
    pub fn builder(name: &str, description: &str) -> ToolBuilder {
        ToolBuilder {
            name: name.to_string(),
            description: description.to_string(),
            params: Vec::new(),
        }
    }
    
    /// Convert tool definition to JSON Schema format
    ///
    /// Returns a JSON object compatible with MCP protocol:
    /// ```json
    /// {
    ///   "name": "tool_name",
    ///   "description": "Tool description",
    ///   "inputSchema": {
    ///     "type": "object",
    ///     "properties": { ... },
    ///     "required": [ ... ]
    ///   }
    /// }
    /// ```
    pub fn to_json_schema(&self) -> Value {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();
        
        for param in &self.params {
            properties.insert(
                param.name.clone(),
                json!({
                    "type": param.param_type.to_json_type(),
                    "description": param.description
                })
            );
            
            if param.required {
                required.push(param.name.clone());
            }
        }
        
        json!({
            "name": self.name,
            "description": self.description,
            "inputSchema": {
                "type": "object",
                "properties": properties,
                "required": required
            }
        })
    }
}

/// Builder for creating tools with a fluent API
pub struct ToolBuilder {
    name: String,
    description: String,
    params: Vec<ToolParam>,
}

impl ToolBuilder {
    /// Add a string parameter
    ///
    /// # Arguments
    /// * `name` - Parameter name
    /// * `description` - Parameter description
    /// * `required` - Whether the parameter is required
    pub fn param_string(mut self, name: &str, description: &str, required: bool) -> Self {
        self.params.push(ToolParam {
            name: name.to_string(),
            description: description.to_string(),
            param_type: ParamType::String,
            required,
        });
        self
    }
    
    /// Add an integer parameter (i64)
    pub fn param_i64(mut self, name: &str, description: &str, required: bool) -> Self {
        self.params.push(ToolParam {
            name: name.to_string(),
            description: description.to_string(),
            param_type: ParamType::Integer,
            required,
        });
        self
    }
    
    /// Add a number parameter (f64)
    pub fn param_f64(mut self, name: &str, description: &str, required: bool) -> Self {
        self.params.push(ToolParam {
            name: name.to_string(),
            description: description.to_string(),
            param_type: ParamType::Number,
            required,
        });
        self
    }
    
    /// Add a boolean parameter
    pub fn param_bool(mut self, name: &str, description: &str, required: bool) -> Self {
        self.params.push(ToolParam {
            name: name.to_string(),
            description: description.to_string(),
            param_type: ParamType::Boolean,
            required,
        });
        self
    }
    
    /// Add an object parameter
    pub fn param_object(mut self, name: &str, description: &str, required: bool) -> Self {
        self.params.push(ToolParam {
            name: name.to_string(),
            description: description.to_string(),
            param_type: ParamType::Object,
            required,
        });
        self
    }
    
    /// Add an array parameter
    pub fn param_array(mut self, name: &str, description: &str, required: bool) -> Self {
        self.params.push(ToolParam {
            name: name.to_string(),
            description: description.to_string(),
            param_type: ParamType::Array,
            required,
        });
        self
    }
    
    /// Set the handler function and finalize the tool
    ///
    /// This consumes the builder and returns the completed Tool.
    pub fn handler(self, handler: ToolHandler) -> Tool {
        Tool {
            name: self.name,
            description: self.description,
            params: self.params,
            handler,
        }
    }
}

