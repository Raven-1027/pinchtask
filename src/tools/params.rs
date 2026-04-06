//! MCP 工具参数结构体定义。
//!
//! 为每个 MCP 工具定义强类型的参数结构体，
//! 实现 `Deserialize` 用于 JSON 反序列化，`JsonSchema` 用于自动生成 inputSchema。

use std::sync::Arc;

use schemars::generate::SchemaSettings;
use schemars::JsonSchema;
use schemars::Schema;
use serde::Deserialize;

// ---------------------------------------------------------------------------
// 辅助 / 嵌套类型
// ---------------------------------------------------------------------------

/// 初始化任务时的清单条目输入。
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct InitialChecklistItem {
    #[schemars(description = "Short name for the checklist item")]
    pub task: String,
    #[schemars(description = "Detailed description")]
    pub detailed_description: String,
    #[serde(default)]
    #[schemars(description = "Context and plan")]
    pub context_and_plan: Option<String>,
    #[serde(default)]
    #[schemars(description = "Whether the item is already done")]
    pub done: bool,
    #[serde(default)]
    #[schemars(description = "Optional pre-assigned ID (UUID); auto-generated if omitted")]
    pub id: Option<String>,
}

impl Default for InitialChecklistItem {
    fn default() -> Self {
        Self {
            task: String::new(),
            detailed_description: String::new(),
            context_and_plan: None,
            done: false,
            id: None,
        }
    }
}

/// 资源引用输入。
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ResourceInput {
    #[schemars(description = "Resource name")]
    pub name: String,
    #[schemars(description = "Resource URL or file path")]
    pub url: String,
    #[serde(default)]
    #[schemars(description = "Resource description")]
    pub description: Option<String>,
}

/// 任务元数据输入。
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct TaskMetadataInput {
    #[serde(default)]
    #[schemars(description = "Tag list")]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    #[schemars(description = "Priority level: high / medium / low")]
    pub priority: Option<String>,
    #[serde(default)]
    #[schemars(description = "Estimated completion time (ISO timestamp or duration)")]
    pub estimated_completion_time: Option<String>,
}

impl Default for TaskMetadataInput {
    fn default() -> Self {
        Self {
            tags: None,
            priority: None,
            estimated_completion_time: None,
        }
    }
}

// ---------------------------------------------------------------------------
// 工具参数结构体（按 server.rs 中 register_builtin_tools 的顺序）
// ---------------------------------------------------------------------------

/// `initialize_task` 参数。
#[derive(Debug, Deserialize, JsonSchema)]
pub struct InitializeTaskParams {
    #[schemars(description = "A medium-level detailed description about the whole task")]
    pub task_description: String,
    #[serde(default)]
    #[schemars(description = "Information that all tasks in the checklist should include")]
    pub context_for_all_tasks: Option<String>,
    #[serde(default)]
    #[schemars(description = "Optional initial checklist items")]
    pub initial_checklist: Option<Vec<InitialChecklistItem>>,
    #[serde(default)]
    #[schemars(description = "Optional initial notes")]
    pub notes: Option<Vec<String>>,
    #[serde(default)]
    #[schemars(description = "Optional initial resources")]
    pub resources: Option<Vec<ResourceInput>>,
    #[serde(default)]
    #[schemars(description = "Optional metadata for the task")]
    pub metadata: Option<TaskMetadataInput>,
}

/// `update_task` 参数（统一更新多个字段）。
#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateTaskParams {
    #[schemars(description = "The ID of the task to update")]
    pub task_id: String,
    #[serde(default)]
    #[schemars(description = "The new task description")]
    pub task_description: Option<String>,
    #[serde(default)]
    #[schemars(description = "The new context information")]
    pub context_for_all_tasks: Option<String>,
    #[serde(default)]
    #[schemars(description = "Priority level: high / medium / low")]
    pub priority: Option<String>,
    #[serde(default)]
    #[schemars(description = "Comma-separated tags")]
    pub tags: Option<String>,
    #[serde(default)]
    #[schemars(description = "Estimated completion time (ISO timestamp or duration)")]
    pub eta: Option<String>,
}

/// Action type for checklist item operations.
///
/// Serialized as lowercase strings: "add", "update", "reorder", "remove".
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    /// Append a new item to the end of the checklist.
    Add,
    /// Modify an existing item's fields. Only specified fields are changed.
    Update,
    /// Move an item to a new position. After reordering, indices change.
    Reorder,
    /// Delete an item. After removal, subsequent indices shift down by 1.
    Remove,
}

/// `manage_checklist_item` 参数。
///
/// 扁平结构，所有操作共享一个 struct，通过 `action` 字段区分操作类型。
/// `action` 是必填字段，其他字段根据 action 类型有不同含义。
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ManageChecklistItemParams {
    /// The operation to perform. Must be one of: "add", "update", "reorder", "remove".
    #[schemars(
        description = "The operation to perform. Must be one of: \"add\", \"update\", \"reorder\", \"remove\""
    )]
    pub action: Action,

    #[schemars(description = "The ID of the task")]
    pub task_id: String,

    // --- Add 专用 ---
    #[serde(default)]
    #[schemars(description = "A short yet comprehensive name for the item (required for Add)")]
    pub task: Option<String>,
    #[serde(default)]
    #[schemars(
        description = "A longer description about what we want to achieve (required for Add)"
    )]
    pub detailed_description: Option<String>,

    // --- Update / Remove 专用 ---
    #[serde(default)]
    #[schemars(description = "0-based index of the checklist item (required for Update/Remove)")]
    pub index: Option<u64>,

    // --- Reorder 专用 ---
    #[serde(default)]
    #[schemars(description = "Current 0-based index (required for Reorder)")]
    pub from_index: Option<u64>,
    #[serde(default)]
    #[schemars(description = "New 0-based index (required for Reorder)")]
    pub to_index: Option<u64>,

    // --- Update 专用 ---
    /// 三态语义：字段未传入 → `None` → 不修改；传入 `null` → `Some(None)` → 清空；传入字符串 → `Some(Some("..."))` → 更新。
    #[serde(default)]
    #[schemars(
        description = "Related information and a detailed plan (pass null to clear, omit to keep unchanged)"
    )]
    pub context_and_plan: Option<Option<String>>,
    #[serde(default)]
    #[schemars(
        description = "Whether the item is completed (for Update only). true=done, false=undone"
    )]
    pub done: Option<bool>,
}

impl Default for ManageChecklistItemParams {
    fn default() -> Self {
        Self {
            action: Action::Add,
            task_id: String::new(),
            task: None,
            detailed_description: None,
            index: None,
            from_index: None,
            to_index: None,
            context_and_plan: None,
            done: None,
        }
    }
}

/// `add_note` 参数。
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddNoteParams {
    #[schemars(description = "The ID of the task")]
    pub task_id: String,
    #[schemars(description = "The content of the note")]
    pub content: String,
}

/// `add_resource` 参数。
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AddResourceParams {
    #[schemars(description = "The ID of the task")]
    pub task_id: String,
    #[schemars(description = "Name of the resource")]
    pub name: String,
    #[schemars(description = "URL or file path of the resource")]
    pub url: String,
    #[serde(default)]
    #[schemars(description = "Description of the resource")]
    pub description: Option<String>,
}

/// `get_checklist_summary` 参数。
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetChecklistSummaryParams {
    #[schemars(description = "The ID of the task")]
    pub task_id: String,
    #[serde(default)]
    #[schemars(description = "Whether to include detailed descriptions")]
    pub include_descriptions: Option<bool>,
}

/// `clear_task` 参数。
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClearTaskParams {
    #[schemars(description = "The ID of the task to delete")]
    pub task_id: String,
}

/// `list_tasks` 参数（无额外参数）。
#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct ListTasksParams {}

/// `get_current_task_details` 参数（无额外参数）。
#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct GetCurrentTaskDetailsParams {}

// ---------------------------------------------------------------------------
// Schema 生成：内联所有 $ref，输出 MCP 客户端可直接使用的 schema
// ---------------------------------------------------------------------------

/// 为参数类型 `T` 生成完全内联的 JSON Schema。
///
/// rmcp 内部的 `schema_for_type` 使用 `inline_subschemas = false`，
/// 会导致嵌套类型（如 `InitialChecklistItem`、`Action`）被放入 `$defs` 并通过 `$ref` 引用。
/// 多数 MCP 客户端不支持 JSON Schema 引用解析，因此需要在此处：
/// 1. 使用 `inline_subschemas = true` 重新生成 schema
/// 2. 通过 `resolve_refs` 递归内联任何残留的 `$ref`
/// 3. 清理 `$schema`、`$defs`、`title` 等多余字段
///
/// 返回 `Arc<Map<String, Value>>` 以匹配 rmcp 的 `input_schema` 类型要求。
pub fn json_schema_for<T: JsonSchema>() -> Arc<serde_json::Map<String, serde_json::Value>> {
    let mut settings = SchemaSettings::draft2020_12();
    settings.inline_subschemas = true;

    let generator = settings.into_generator();
    let schema: Schema = generator.into_root_schema_for::<T>();
    let mut schema = serde_json::to_value(&schema).expect("schema serialization failed");

    // 1. 递归内联所有 $ref（安全网，处理 inline_subschemas 未覆盖的边界情况）
    resolve_refs(&mut schema);

    // 2. 简化 nullable 类型：将 `type: ["T", "null"]` 转换为 `type: "T"`
    //    许多 MCP 客户端不支持联合类型语法，更好的兼容性策略是保持 type 为基础类型，
    //    同时确保字段不在 required 中（schemars 对 Option<T> 已自动处理）。
    simplify_nullable_types(&mut schema);

    // 3. 清理顶层多余字段
    if let Some(obj) = schema.as_object_mut() {
        obj.remove("$schema");
        obj.remove("$defs");
        obj.remove("title");
    }

    Arc::new(schema.as_object().cloned().unwrap_or_default())
}

/// 递归将所有 `$ref` 引用内联展开，并清理 `definitions`、`allOf` 包裹、`title`。
///
/// 处理三种模式：
/// - `{"$ref": "#/$defs/X"}` 或 `{"$ref": "#/definitions/X"}` → 内联替换为 X 的定义
/// - `{"allOf": [schema]}` → 单元素 allOf 直接展开
/// - 递归处理所有嵌套的对象和数组
fn resolve_refs(schema: &mut serde_json::Value) {
    match schema {
        serde_json::Value::Object(map) => {
            // 模式 1: 纯 $ref → 查找并内联
            if map.len() == 1 {
                if let Some(serde_json::Value::String(ref_uri)) = map.get("$ref") {
                    let def_name = extract_def_name(ref_uri);
                    if let Some(inline) =
                        find_definition(&serde_json::Value::Object(map.clone()), &def_name)
                    {
                        *schema = inline;
                        // 内联后继续处理（可能还有嵌套 $ref）
                        resolve_refs(schema);
                        return;
                    }
                }
            }

            // 模式 2: 单元素 allOf → 展开
            if let Some(serde_json::Value::Array(arr)) = map.get("allOf") {
                if arr.len() == 1 {
                    let inner = arr[0].clone();
                    *schema = inner;
                    resolve_refs(schema);
                    return;
                }
            }

            // 递归处理对象中所有值
            for value in map.values_mut() {
                resolve_refs(value);
            }

            // 清理多余字段
            map.remove("title");
        }
        serde_json::Value::Array(arr) => {
            for item in arr.iter_mut() {
                resolve_refs(item);
            }
        }
        _ => {}
    }
}

/// 从 `$ref` URI 中提取定义名称。
///
/// 支持格式：
/// - `#/$defs/TypeName`
/// - `#/definitions/TypeName`
fn extract_def_name(ref_uri: &str) -> String {
    ref_uri.split('/').last().unwrap_or("").to_string()
}

/// 在 schema 的 `$defs` 或 `definitions` 中查找指定名称的定义。
///
/// 注意：由于我们使用 `inline_subschemas = true`，正常情况下不应有 `$defs`。
/// 此函数仅作为安全网。
fn find_definition(schema: &serde_json::Value, name: &str) -> Option<serde_json::Value> {
    if let Some(obj) = schema.as_object() {
        if let Some(serde_json::Value::Object(defs)) = obj.get("$defs") {
            if let Some(def) = defs.get(name) {
                return Some(def.clone());
            }
        }
        if let Some(serde_json::Value::Object(defs)) = obj.get("definitions") {
            if let Some(def) = defs.get(name) {
                return Some(def.clone());
            }
        }
    }
    None
}

/// 简化 nullable 类型：将 `type: ["T", "null"]` 转换为 `type: "T"`。
///
/// schemars 默认将 `Option<T>` 序列化为联合类型 `["T", "null"]`，
/// 这是 JSON Schema 2020-12 的合法语法，但许多 MCP 客户端不支持。
///
/// 更好的兼容性策略：
/// - 保持 `type` 为基础类型（如 `"string"` 而非 `["string", "null"]`）
/// - 字段不在 `required` 数组中（schemars 对 `Option<T>` 已自动处理）
/// - 移除多余的 `"default": null`（不提供默认值更明确）
///
/// 处理模式：
/// - `"type": ["string", "null"]` → `"type": "string"`
/// - `"type": ["array", "null"]` → `"type": "array"`
/// - `"type": ["integer", "null"]` → `"type": "integer"`
/// - `"type": ["boolean", "null"]` → `"type": "boolean"`
/// - `"type": ["object", "null"]` → `"type": "object"`
fn simplify_nullable_types(schema: &mut serde_json::Value) {
    match schema {
        serde_json::Value::Object(map) => {
            // 检查是否有 type 字段是 ["X", "null"] 格式
            if let Some(serde_json::Value::Array(type_arr)) = map.get("type") {
                // 过滤掉 "null"，只保留实际类型
                let non_null_types: Vec<&serde_json::Value> = type_arr
                    .iter()
                    .filter(|t| t.as_str() != Some("null"))
                    .collect();

                if non_null_types.len() == 1 {
                    // 只有一个非 null 类型，简化为单一类型
                    map.insert("type".to_string(), (*non_null_types[0]).clone());
                    // 移除多余的 default: null
                    if map.get("default").and_then(|v| v.as_str()) == Some("null")
                        || map.get("default").is_some_and(|v| v.is_null())
                    {
                        map.remove("default");
                    }
                }
            }

            // 递归处理所有嵌套值
            for value in map.values_mut() {
                simplify_nullable_types(value);
            }
        }
        serde_json::Value::Array(arr) => {
            for item in arr.iter_mut() {
                simplify_nullable_types(item);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_task_schema_has_no_refs() {
        let schema = json_schema_for::<InitializeTaskParams>();
        let schema_str = serde_json::to_string(&schema).unwrap();
        assert!(
            !schema_str.contains("$ref"),
            "schema should not contain $ref: {schema_str}"
        );
        assert!(
            !schema_str.contains("$defs"),
            "schema should not contain $defs: {schema_str}"
        );
        assert!(
            !schema_str.contains("definitions"),
            "schema should not contain definitions: {schema_str}"
        );
    }

    #[test]
    fn manage_checklist_item_schema_has_no_refs() {
        let schema = json_schema_for::<ManageChecklistItemParams>();
        let schema_str = serde_json::to_string(&schema).unwrap();
        assert!(
            !schema_str.contains("$ref"),
            "schema should not contain $ref: {schema_str}"
        );
        assert!(
            !schema_str.contains("$defs"),
            "schema should not contain $defs: {schema_str}"
        );
    }

    #[test]
    fn new_task_schema_has_inline_checklist_item() {
        let schema = json_schema_for::<InitializeTaskParams>();
        let schema_str = serde_json::to_string(&schema).unwrap();
        // 内联后应包含 InitialChecklistItem 的字段
        assert!(
            schema_str.contains("detailed_description"),
            "should contain inline checklist item fields"
        );
        assert!(
            schema_str.contains("context_and_plan"),
            "should contain inline checklist item fields"
        );
    }

    #[test]
    fn manage_checklist_item_schema_has_inline_action_enum() {
        let schema = json_schema_for::<ManageChecklistItemParams>();
        let schema_str = serde_json::to_string(&schema).unwrap();
        // 内联后应包含 Action 枚举的值
        assert!(
            schema_str.contains("add"),
            "should contain inline action enum values"
        );
        assert!(
            schema_str.contains("update"),
            "should contain inline action enum values"
        );
        assert!(
            schema_str.contains("reorder"),
            "should contain inline action enum values"
        );
        assert!(
            schema_str.contains("remove"),
            "should contain inline action enum values"
        );
    }

    #[test]
    fn schema_is_valid_object() {
        let schema = json_schema_for::<InitializeTaskParams>();
        assert_eq!(schema.get("type").and_then(|v| v.as_str()), Some("object"));
        assert!(schema.contains_key("properties"));
        assert!(schema.contains_key("required"));
    }

    #[test]
    fn extract_def_name_parses_correctly() {
        assert_eq!(extract_def_name("#/$defs/MyType"), "MyType");
        assert_eq!(extract_def_name("#/definitions/MyType"), "MyType");
        assert_eq!(extract_def_name("#/$defs/NestedType"), "NestedType");
    }

    #[test]
    fn schema_has_no_nullable_type_arrays() {
        let schema = json_schema_for::<InitializeTaskParams>();
        let schema_str = serde_json::to_string(&schema).unwrap();
        // 不应出现 ["string", "null"] 等联合类型
        assert!(
            !schema_str.contains(r#""type":["string","null"]"#),
            "should not have nullable string type array: {schema_str}"
        );
        assert!(
            !schema_str.contains(r#""type":["array","null"]"#),
            "should not have nullable array type array: {schema_str}"
        );
        assert!(
            !schema_str.contains(r#""type":["object","null"]"#),
            "should not have nullable object type array: {schema_str}"
        );
    }

    #[test]
    fn optional_fields_not_in_required() {
        let schema = json_schema_for::<InitializeTaskParams>();
        let required = schema
            .get("required")
            .and_then(|v| v.as_array())
            .expect("should have required array");

        // 只有 task_description 是必填的
        assert_eq!(required.len(), 1);
        assert_eq!(required[0].as_str(), Some("task_description"));

        // 确认 Option 字段确实不在 required 中
        let optional_fields = [
            "context_for_all_tasks",
            "initial_checklist",
            "notes",
            "resources",
            "metadata",
        ];
        for field in &optional_fields {
            assert!(
                !required.iter().any(|v| v.as_str() == Some(field)),
                "{field} should not be in required"
            );
        }
    }

    #[test]
    fn nullable_simplification_works() {
        let mut schema = serde_json::json!({
            "type": ["string", "null"],
            "default": null,
            "description": "optional field"
        });
        simplify_nullable_types(&mut schema);

        assert_eq!(schema.get("type").and_then(|v| v.as_str()), Some("string"));
        assert!(!schema.as_object().unwrap().contains_key("default"));
    }

    #[test]
    fn nullable_simplification_preserves_non_nullable() {
        let mut schema = serde_json::json!({
            "type": "string",
            "description": "required field"
        });
        simplify_nullable_types(&mut schema);

        assert_eq!(schema.get("type").and_then(|v| v.as_str()), Some("string"));
    }
}

    #[test]
    fn print_new_task_schema() {
        let schema = json_schema_for::<InitializeTaskParams>();
        eprintln!("{}", serde_json::to_string_pretty(&schema).unwrap());
    }
