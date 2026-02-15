//! Tool calling support for agent loop
//!
//! This module handles parsing tool calls from LLM responses and executing them.

use crate::tools::{Tool, ToolResult};
use serde_json::Value;

/// Maximum number of tool call iterations per user message
pub const MAX_TOOL_ITERATIONS: usize = 10;

/// Represents a single tool call parsed from LLM response
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
}

/// Parse tool calls from an LLM response
///
/// Supports various formats:
/// - JSON blocks: ```tool_calls ... ```
/// - Standalone JSON arrays
/// - Individual tool call objects
pub fn parse_tool_calls(response: &str) -> Vec<ToolCall> {
    let mut calls = Vec::new();

    // Try to find JSON block with tool calls (```tool_calls ... ```)
    if let Some(json_start) = response.find("```tool_calls") {
        // Skip past the opening line by finding the newline
        let after_opening = if let Some(nl_pos) = response[json_start..].find('\n') {
            json_start + nl_pos + 1
        } else {
            json_start + 14
        };

        // Find the closing ``` after the JSON content
        if let Some(json_end) = response[after_opening..].find("```") {
            let json_str = &response[after_opening..after_opening + json_end];
            if let Ok(value) = serde_json::from_str::<Value>(json_str.trim()) {
                calls.extend(extract_calls_from_value(&value));
            }
        }
    }

    // Try to find regular JSON block (```json ... ```)
    if calls.is_empty() {
        if let Some(json_start) = response.find("```json") {
            // Skip past the opening line by finding the newline
            let after_opening = if let Some(nl_pos) = response[json_start..].find('\n') {
                json_start + nl_pos + 1
            } else {
                json_start + 7
            };

            // Find the closing ``` after the JSON content
            if let Some(json_end) = response[after_opening..].find("```") {
                let json_str = &response[after_opening..after_opening + json_end];
                if let Ok(value) = serde_json::from_str::<Value>(json_str.trim()) {
                    calls.extend(extract_calls_from_value(&value));
                }
            }
        }
    }

    // Try to parse as direct JSON
    if calls.is_empty() {
        if let Ok(value) = serde_json::from_str::<Value>(response.trim()) {
            calls.extend(extract_calls_from_value(&value));
        }
    }

    // Try to find tool calls in format: <tool_name>({...})
    if calls.is_empty() {
        if let Some(calls_found) = parse_bracketed_tool_calls(response) {
            calls = calls_found;
        }
    }

    calls
}

/// Extract tool calls from a JSON value
fn extract_calls_from_value(value: &Value) -> Vec<ToolCall> {
    let mut calls = Vec::new();

    // Array of tool calls
    if let Some(array) = value.as_array() {
        for item in array {
            if let Some(call) = extract_single_call(item) {
                calls.push(call);
            }
        }
    } else if let Some(call) = extract_single_call(value) {
        calls.push(call);
    }

    calls
}

/// Extract a single tool call from a JSON value
fn extract_single_call(value: &Value) -> Option<ToolCall> {
    // Try OpenAI-style format
    if let Some(function) = value.get("function") {
        let name = function.get("name")?.as_str()?;
        let arguments = function.get("arguments").cloned().unwrap_or(Value::Object(Default::default()));
        return Some(ToolCall {
            name: name.to_string(),
            arguments,
        });
    }

    // Try Claude/Anthropic-style format (tool_use)
    if value.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
        let name = value.get("name")?.as_str()?;
        let input = value.get("input").cloned().unwrap_or(Value::Object(Default::default()));
        return Some(ToolCall {
            name: name.to_string(),
            arguments: input,
        });
    }

    // Try simple format: {"name": "...", "arguments": {...}}
    if let Some(name) = value.get("name").or(value.get("tool")).and_then(|n| n.as_str()) {
        let arguments = value
            .get("arguments")
            .or(value.get("args"))
            .or(value.get("input"))
            .or(value.get("params"))
            .cloned()
            .unwrap_or(Value::Object(Default::default()));
        return Some(ToolCall {
            name: name.to_string(),
            arguments,
        });
    }

    None
}

/// Parse tool calls in format: tool_name({...})
fn parse_bracketed_tool_calls(response: &str) -> Option<Vec<ToolCall>> {
    use regex::Regex;

    let re = Regex::new(r"(\w+)\s*\((\{.*?\})\)|(\w+)\s*\((\[.*?\])\)|(\w+)\s*\(([^)]+)\)").ok()?;

    let mut calls = Vec::new();
    for caps in re.captures_iter(response) {
        let name = caps.get(1).or(caps.get(3)).or(caps.get(4))?.as_str();
        let args_str = caps.get(2).or(caps.get(5)).or(caps.get(6))?.as_str();

        if let Ok(arguments) = serde_json::from_str::<Value>(args_str) {
            calls.push(ToolCall {
                name: name.to_string(),
                arguments,
            });
        }
    }

    if calls.is_empty() {
        None
    } else {
        Some(calls)
    }
}

/// Execute a tool call and return the result
pub async fn execute_tool_call(
    tool_call: &ToolCall,
    tools: &[Box<dyn Tool>],
) -> ToolResult {
    // Find the tool
    let tool = match tools.iter().find(|t| t.name() == tool_call.name) {
        Some(t) => t,
        None => {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Tool '{}' not found", tool_call.name)),
            };
        }
    };

    // Execute the tool
    match tool.execute(tool_call.arguments.clone()).await {
        Ok(result) => result,
        Err(e) => ToolResult {
            success: false,
            output: String::new(),
            error: Some(format!("Tool execution failed: {e}")),
        },
    }
}

/// Build tool result message for feedback to LLM
pub fn format_tool_result(call: &ToolCall, result: &ToolResult) -> String {
    if result.success {
        format!(
            "Tool {} completed successfully:\nOutput: {}",
            call.name, result.output
        )
    } else {
        format!(
            "Tool {} failed:\nError: {}",
            call.name,
            result.error.as_deref().unwrap_or("Unknown error")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tool_calls_openai_format() {
        let response = r#"```tool_calls
[
  {
    "function": {
      "name": "shell",
      "arguments": {"command": "ls"}
    }
  }
]
```"#;

        let calls = parse_tool_calls(response);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "shell");
    }

    #[test]
    fn test_parse_tool_calls_claude_format() {
        let response = r#"```json
[
  {
    "type": "tool_use",
    "name": "file_read",
    "input": {"path": "/tmp/file.txt"}
  }
]
```"#;

        let calls = parse_tool_calls(response);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "file_read");
    }

    #[test]
    fn test_parse_tool_calls_simple_format() {
        let response = r#"{"name": "shell", "arguments": {"command": "echo hello"}}"#;

        let calls = parse_tool_calls(response);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "shell");
    }

    #[test]
    fn test_parse_tool_calls_empty() {
        let response = "This is just regular text without any tool calls.";
        let calls = parse_tool_calls(response);
        assert!(calls.is_empty());
    }

    #[test]
    fn test_format_tool_result_success() {
        let call = ToolCall {
            name: "shell".to_string(),
            arguments: serde_json::json!({"command": "echo test"}),
        };
        let result = ToolResult {
            success: true,
            output: "test\n".to_string(),
            error: None,
        };

        let formatted = format_tool_result(&call, &result);
        assert!(formatted.contains("shell completed successfully"));
        assert!(formatted.contains("test"));
    }

    #[test]
    fn test_format_tool_result_failure() {
        let call = ToolCall {
            name: "shell".to_string(),
            arguments: serde_json::json!({"command": "invalid"}),
        };
        let result = ToolResult {
            success: false,
            output: String::new(),
            error: Some("Command not found".to_string()),
        };

        let formatted = format_tool_result(&call, &result);
        assert!(formatted.contains("shell failed"));
        assert!(formatted.contains("Command not found"));
    }
}
