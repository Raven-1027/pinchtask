//! MCP 服务器实现。
//!
//! `McpServer` 持有 `TaskStore` 与工具定义列表，实现 MCP 协议生命周期：
//! initialize → tools/list → tools/call。
//! 通过 `StdioTransport` 与客户端通信。

use anyhow::Result;
use serde_json::{json, Value};

use crate::protocol::{
    CallToolParams, CallToolResult, InitializeResult, JsonRpcRequest, JsonRpcResponse,
    ServerInfo, ToolDefinition,
};
use crate::store::TaskStore;
use crate::tools::task as task_tools;
use crate::transport::StdioTransport;

/// MCP 服务器。
pub struct McpServer {
    store: TaskStore,
    transport: StdioTransport,
    tool_definitions: Vec<ToolDefinition>,
    server_info: ServerInfo,
}

impl McpServer {
    /// 创建新的 MCP 服务器实例。
    pub fn new(store: TaskStore) -> Self {
        let transport = StdioTransport::new();
        let server_info = ServerInfo {
            name: "mcp-pinchtask".to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
        };
        let mut server = Self {
            store,
            transport,
            tool_definitions: Vec::new(),
            server_info,
        };
        server.register_builtin_tools();
        server
    }

    /// 注册内置工具（仅注册定义，handler 通过 match dispatch 调用）。
    fn register_builtin_tools(&mut self) {
        // ------------------------------------------------------------------
        // initialize_task
        // ------------------------------------------------------------------
        self.tool_definitions.push(ToolDefinition {
            name: "initialize_task".to_owned(),
            description: "Create a new task with a description, optional checklist items, notes, resources, and metadata.".to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "task_description": {
                        "type": "string",
                        "description": "A medium-level detailed description about the whole task"
                    },
                    "context_for_all_tasks": {
                        "type": "string",
                        "description": "Information that all tasks in the checklist should include"
                    },
                    "initial_checklist": {
                        "type": "array",
                        "description": "Optional initial checklist items",
                        "items": {
                            "type": "object",
                            "properties": {
                                "task": { "type": "string", "description": "Short name for the checklist item" },
                                "detailed_description": { "type": "string", "description": "Detailed description" },
                                "context_and_plan": { "type": "string", "description": "Context and plan" },
                                "done": { "type": "boolean", "description": "Whether the item is already done", "default": false }
                            },
                            "required": ["task", "detailed_description"]
                        }
                    },
                    "notes": {
                        "type": "array",
                        "description": "Optional initial notes",
                        "items": { "type": "string" }
                    },
                    "resources": {
                        "type": "array",
                        "description": "Optional initial resources",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string" },
                                "url": { "type": "string" },
                                "description": { "type": "string" }
                            },
                            "required": ["name", "url"]
                        }
                    },
                    "metadata": {
                        "type": "object",
                        "description": "Optional metadata for the task",
                        "properties": {
                            "tags": {
                                "type": "array",
                                "items": { "type": "string" }
                            },
                            "priority": {
                                "type": "string",
                                "enum": ["high", "medium", "low"]
                            },
                            "estimated_completion_time": {
                                "type": "string",
                                "description": "ISO timestamp or duration"
                            }
                        }
                    }
                },
                "required": ["task_description"]
            }),
        });

        // ------------------------------------------------------------------
        // update_task (统一更新)
        // ------------------------------------------------------------------
        self.tool_definitions.push(ToolDefinition {
            name: "update_task".to_owned(),
            description: "Update multiple task fields at once: description, context, priority, tags, and/or eta. Only specified fields are modified.".to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "The ID of the task to update" },
                    "task_description": { "type": "string", "description": "The new task description" },
                    "context_for_all_tasks": { "type": "string", "description": "The new context information" },
                    "priority": { "type": "string", "description": "Priority level", "enum": ["high", "medium", "low"] },
                    "tags": { "type": "string", "description": "Comma-separated tags" },
                    "eta": { "type": "string", "description": "Estimated completion time (ISO timestamp or duration)" }
                },
                "required": ["task_id"]
            }),
        });

        // ------------------------------------------------------------------
        // update_task_description
        // ------------------------------------------------------------------
        self.tool_definitions.push(ToolDefinition {
            name: "update_task_description".to_owned(),
            description: "Update the overall description of an existing task.".to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "The ID of the task to update" },
                    "task_description": { "type": "string", "description": "The new task description" }
                },
                "required": ["task_id", "task_description"]
            }),
        });

        // ------------------------------------------------------------------
        // update_context
        // ------------------------------------------------------------------
        self.tool_definitions.push(ToolDefinition {
            name: "update_context".to_owned(),
            description: "Update the shared context information for all sub-tasks.".to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "The ID of the task" },
                    "context_for_all_tasks": { "type": "string", "description": "The new context information" }
                },
                "required": ["task_id", "context_for_all_tasks"]
            }),
        });

        // ------------------------------------------------------------------
        // add_checklist_item
        // ------------------------------------------------------------------
        self.tool_definitions.push(ToolDefinition {
            name: "add_checklist_item".to_owned(),
            description: "Add a new item to the task checklist.".to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "The ID of the task" },
                    "task": { "type": "string", "description": "A short yet comprehensive name for the task" },
                    "detailed_description": { "type": "string", "description": "A longer description about what we want to achieve" },
                    "context_and_plan": { "type": "string", "description": "Related information and a detailed plan" }
                },
                "required": ["task_id", "task", "detailed_description"]
            }),
        });

        // ------------------------------------------------------------------
        // update_checklist_item
        // ------------------------------------------------------------------
        self.tool_definitions.push(ToolDefinition {
            name: "update_checklist_item".to_owned(),
            description: "Update an existing checklist item.".to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "The ID of the task" },
                    "index": { "type": "integer", "description": "0-based index of the checklist item to update", "minimum": 0 },
                    "task": { "type": "string", "description": "New short name" },
                    "detailed_description": { "type": "string", "description": "New detailed description" },
                    "context_and_plan": { "type": "string", "description": "New context and plan (pass null to clear)" },
                    "done": { "type": "boolean", "description": "Whether the item is completed" }
                },
                "required": ["task_id", "index"]
            }),
        });

        // ------------------------------------------------------------------
        // mark_task_done
        // ------------------------------------------------------------------
        self.tool_definitions.push(ToolDefinition {
            name: "mark_task_done".to_owned(),
            description: "Mark a specific checklist item as completed.".to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "The ID of the task" },
                    "index": { "type": "integer", "description": "0-based index of the checklist item", "minimum": 0 }
                },
                "required": ["task_id", "index"]
            }),
        });

        // ------------------------------------------------------------------
        // mark_task_undone
        // ------------------------------------------------------------------
        self.tool_definitions.push(ToolDefinition {
            name: "mark_task_undone".to_owned(),
            description: "Mark a specific checklist item as not completed.".to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "The ID of the task" },
                    "index": { "type": "integer", "description": "0-based index of the checklist item", "minimum": 0 }
                },
                "required": ["task_id", "index"]
            }),
        });

        // ------------------------------------------------------------------
        // reorder_checklist_item
        // ------------------------------------------------------------------
        self.tool_definitions.push(ToolDefinition {
            name: "reorder_checklist_item".to_owned(),
            description: "Move a checklist item to a new position.".to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "The ID of the task" },
                    "from_index": { "type": "integer", "description": "Current 0-based index", "minimum": 0 },
                    "to_index": { "type": "integer", "description": "New 0-based index", "minimum": 0 }
                },
                "required": ["task_id", "from_index", "to_index"]
            }),
        });

        // ------------------------------------------------------------------
        // remove_checklist_item
        // ------------------------------------------------------------------
        self.tool_definitions.push(ToolDefinition {
            name: "remove_checklist_item".to_owned(),
            description: "Remove a checklist item from the task.".to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "The ID of the task" },
                    "index": { "type": "integer", "description": "0-based index of the checklist item to remove", "minimum": 0 }
                },
                "required": ["task_id", "index"]
            }),
        });

        // ------------------------------------------------------------------
        // add_note
        // ------------------------------------------------------------------
        self.tool_definitions.push(ToolDefinition {
            name: "add_note".to_owned(),
            description: "Add a note to the task.".to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "The ID of the task" },
                    "content": { "type": "string", "description": "The content of the note" }
                },
                "required": ["task_id", "content"]
            }),
        });

        // ------------------------------------------------------------------
        // add_resource
        // ------------------------------------------------------------------
        self.tool_definitions.push(ToolDefinition {
            name: "add_resource".to_owned(),
            description: "Add a resource reference to the task.".to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "The ID of the task" },
                    "name": { "type": "string", "description": "Name of the resource" },
                    "url": { "type": "string", "description": "URL or file path of the resource" },
                    "description": { "type": "string", "description": "Description of the resource" }
                },
                "required": ["task_id", "name", "url"]
            }),
        });

        // ------------------------------------------------------------------
        // update_metadata
        // ------------------------------------------------------------------
        self.tool_definitions.push(ToolDefinition {
            name: "update_metadata".to_owned(),
            description: "Update the task metadata (tags, priority, estimated completion time).".to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "The ID of the task" },
                    "metadata": {
                        "type": "object",
                        "description": "The metadata object to set",
                        "properties": {
                            "tags": {
                                "type": "array",
                                "items": { "type": "string" }
                            },
                            "priority": {
                                "type": "string",
                                "enum": ["high", "medium", "low"]
                            },
                            "estimated_completion_time": {
                                "type": "string",
                                "description": "ISO timestamp or duration"
                            }
                        }
                    }
                },
                "required": ["task_id", "metadata"]
            }),
        });

        // ------------------------------------------------------------------
        // get_checklist_summary
        // ------------------------------------------------------------------
        self.tool_definitions.push(ToolDefinition {
            name: "get_checklist_summary".to_owned(),
            description: "Get a summary of the task checklist with completion status.".to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "The ID of the task" },
                    "include_descriptions": { "type": "boolean", "description": "Whether to include detailed descriptions", "default": false }
                },
                "required": ["task_id"]
            }),
        });

        // ------------------------------------------------------------------
        // clear_task
        // ------------------------------------------------------------------
        self.tool_definitions.push(ToolDefinition {
            name: "clear_task".to_owned(),
            description: "Delete a task by its ID.".to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "task_id": { "type": "string", "description": "The ID of the task to delete" }
                },
                "required": ["task_id"]
            }),
        });

        // ------------------------------------------------------------------
        // list_tasks (no params)
        // ------------------------------------------------------------------
        self.tool_definitions.push(ToolDefinition {
            name: "list_tasks".to_owned(),
            description: "List all tasks sorted by creation time.".to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        });

        // ------------------------------------------------------------------
        // get_current_task_details (no params)
        // ------------------------------------------------------------------
        self.tool_definitions.push(ToolDefinition {
            name: "get_current_task_details".to_owned(),
            description: "Get details of the first uncompleted task (current task) with full context.".to_owned(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        });
    }

    /// 运行主循环：从 transport 读取请求并分发。
    pub async fn run(mut self) -> Result<()> {
        tracing::info!("MCP server starting stdio transport loop");
        loop {
            let request = match self.transport.read_request().await {
                Ok(Some(req)) => req,
                Ok(None) => {
                    tracing::info!("Client disconnected (EOF)");
                    break;
                }
                Err(e) => {
                    tracing::error!("Failed to read request: {e}");
                    break;
                }
            };

            let response = self.handle_request(request).await;
            if let Err(e) = self.transport.write_response(&response).await {
                tracing::error!("Failed to write response: {e}");
                break;
            }
        }
        tracing::info!("MCP server loop ended");
        Ok(())
    }

    /// 分发请求到对应的处理器。
    pub async fn handle_request(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        let id = request.id.clone();
        match request.method.as_str() {
            "initialize" => self.handle_initialize(id.clone(), request.params),
            "notifications/initialized" => {
                // 客户端确认初始化完成，无需响应通知
                tracing::debug!("Received initialized notification");
                JsonRpcResponse::ok(id, json!({}))
            }
            "tools/list" => self.handle_tools_list(id),
            "tools/call" => self.handle_tools_call(id, request.params).await,
            "ping" => JsonRpcResponse::ok(id, json!({})),
            other => {
                tracing::warn!("Unknown method: {other}");
                JsonRpcResponse::err(id, -32601, format!("Method not found: {other}"))
            }
        }
    }

    /// 处理 initialize 请求。
    fn handle_initialize(&self, id: Option<Value>, _params: Option<Value>) -> JsonRpcResponse {
        let result = InitializeResult {
            protocol_version: "2024-11-05".to_owned(),
            capabilities: json!({
                "tools": {}
            }),
            server_info: self.server_info.clone(),
        };
        JsonRpcResponse::ok(id, serde_json::to_value(result).expect("序列化 InitializeResult 不应失败"))
    }

    /// 处理 tools/list 请求。
    fn handle_tools_list(&self, id: Option<Value>) -> JsonRpcResponse {
        JsonRpcResponse::ok(id, json!({ "tools": &self.tool_definitions }))
    }

    /// 处理 tools/call 请求。
    ///
    /// 使用 match dispatch 直接调用对应的 async handler，无需函数指针。
    async fn handle_tools_call(
        &self,
        id: Option<Value>,
        params: Option<Value>,
    ) -> JsonRpcResponse {
        let params: CallToolParams = match params {
            Some(v) => match serde_json::from_value(v) {
                Ok(p) => p,
                Err(e) => {
                    return JsonRpcResponse::err(
                        id,
                        -32602,
                        format!("Invalid params: {e}"),
                    );
                }
            },
            None => {
                return JsonRpcResponse::err(id, -32602, "Missing params");
            }
        };

        let tool_name = params.name.as_str();
        let args = params.arguments;

        let result: std::result::Result<CallToolResult, String> = match tool_name {
            "initialize_task" => {
                task_tools::initialize_task_handler(&self.store, args).await
            }
            "update_task" => {
                task_tools::update_task_handler(&self.store, args).await
            }
            "update_task_description" => {
                task_tools::update_task_description_handler(&self.store, args).await
            }
            "update_context" => {
                task_tools::update_context_handler(&self.store, args).await
            }
            "add_checklist_item" => {
                task_tools::add_checklist_item_handler(&self.store, args).await
            }
            "update_checklist_item" => {
                task_tools::update_checklist_item_handler(&self.store, args).await
            }
            "mark_task_done" => {
                task_tools::mark_task_done_handler(&self.store, args).await
            }
            "mark_task_undone" => {
                task_tools::mark_task_undone_handler(&self.store, args).await
            }
            "reorder_checklist_item" => {
                task_tools::reorder_checklist_item_handler(&self.store, args).await
            }
            "remove_checklist_item" => {
                task_tools::remove_checklist_item_handler(&self.store, args).await
            }
            "add_note" => {
                task_tools::add_note_handler(&self.store, args).await
            }
            "add_resource" => {
                task_tools::add_resource_handler(&self.store, args).await
            }
            "update_metadata" => {
                task_tools::update_metadata_handler(&self.store, args).await
            }
            "get_checklist_summary" => {
                task_tools::get_checklist_summary_handler(&self.store, args).await
            }
            "clear_task" => {
                task_tools::clear_task_handler(&self.store, args).await
            }
            "list_tasks" => {
                task_tools::list_tasks_handler(&self.store, args).await
            }
            "get_current_task_details" => {
                task_tools::get_current_task_details_handler(&self.store, args).await
            }
            _ => {
                return JsonRpcResponse::err(
                    id,
                    -32601,
                    format!("Unknown tool: {tool_name}"),
                );
            }
        };

        match result {
            Ok(result) => JsonRpcResponse::ok(
                id,
                serde_json::to_value(result).expect("序列化 CallToolResult 不应失败"),
            ),
            Err(e) => JsonRpcResponse::ok(
                id,
                serde_json::to_value(CallToolResult::error_result(e))
                    .expect("序列化 error CallToolResult 不应失败"),
            ),
        }
    }
}

/// 启动 MCP 服务器的入口函数（使用默认数据目录）。
pub async fn run() -> Result<()> {
    let store = TaskStore::new(None).await?;
    let server = McpServer::new(store);
    server.run().await
}
