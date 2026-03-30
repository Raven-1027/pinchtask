//! MCP 协议流程集成测试。
//!
//! 测试完整的 JSON-RPC 请求/响应生命周期：
//! initialize → tools/list → tools/call (创建任务、添加检查项、标记完成、获取摘要)。

use serde_json::{json, Value};
use tempfile::TempDir;

use mcp_pinchtask::protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
use mcp_pinchtask::server::McpServer;
use mcp_pinchtask::store::TaskStore;

// ---------------------------------------------------------------------------
// 辅助函数
// ---------------------------------------------------------------------------

/// 创建使用临时目录的 McpServer 实例。
///
/// 返回 (server, _tempdir)。调用者必须保持 `_tempdir` 存活。
fn test_server() -> (McpServer, TempDir) {
    let dir = tempfile::tempdir().expect("创建临时目录失败");
    let store = TaskStore::new(Some(dir.path().to_path_buf())).expect("创建 TaskStore 失败");
    let server = McpServer::new(store);
    (server, dir)
}

/// 构造一个 JSON-RPC 2.0 请求。
fn make_request(method: &str, id: i64, params: Option<Value>) -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(json!(id)),
        method: method.to_owned(),
        params,
    }
}

/// 从响应中提取 result 字段。
fn result_of(resp: &JsonRpcResponse) -> &Value {
    resp.result.as_ref().expect("响应缺少 result 字段")
}

/// 从响应中提取 error 字段的引用。
fn error_of(resp: &JsonRpcResponse) -> &JsonRpcError {
    resp.error.as_ref().expect("响应缺少 error 字段")
}

// ---------------------------------------------------------------------------
// 测试用例
// ---------------------------------------------------------------------------

#[test]
fn test_initialize_request_response() {
    let (mut server, _dir) = test_server();

    let req = make_request(
        "initialize",
        1,
        Some(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "test-client", "version": "0.1.0" }
        })),
    );

    let resp = server.handle_request(req);

    assert!(resp.error.is_none(), "initialize 不应返回错误");
    let result = result_of(&resp);

    // 验证协议版本
    assert_eq!(result["protocolVersion"], "2024-11-05");

    // 验证 capabilities 包含 tools
    assert!(result["capabilities"]["tools"].is_object());

    // 验证 server_info
    assert_eq!(result["serverInfo"]["name"], "mcp-pinchtask");
    assert_eq!(result["serverInfo"]["version"], env!("CARGO_PKG_VERSION"));
}

#[test]
fn test_tools_list_returns_all_16_tools() {
    let (mut server, _dir) = test_server();

    let req = make_request("tools/list", 2, None);
    let resp = server.handle_request(req);

    assert!(resp.error.is_none());
    let result = result_of(&resp);
    let tools = result["tools"].as_array().expect("tools 应为数组");

    assert_eq!(tools.len(), 16, "应注册 16 个工具");

    // 验证所有工具都有 name, description, inputSchema
    for tool in tools {
        assert!(tool["name"].is_string(), "工具应包含 name 字段");
        assert!(tool["description"].is_string(), "工具应包含 description 字段");
        assert!(tool["inputSchema"].is_object(), "工具应包含 inputSchema 字段");
    }

    // 验证关键工具存在
    let tool_names: Vec<&str> = tools
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();

    let expected_tools = [
        "initialize_task",
        "update_task_description",
        "update_context",
        "add_checklist_item",
        "update_checklist_item",
        "mark_task_done",
        "mark_task_undone",
        "reorder_checklist_item",
        "remove_checklist_item",
        "add_note",
        "add_resource",
        "update_metadata",
        "get_checklist_summary",
        "clear_task",
        "list_tasks",
        "get_current_task_details",
    ];

    for expected in &expected_tools {
        assert!(
            tool_names.contains(expected),
            "工具列表中应包含 '{expected}'"
        );
    }
}

#[test]
fn test_tools_call_initialize_task() {
    let (mut server, _dir) = test_server();

    let req = make_request(
        "tools/call",
        3,
        Some(json!({
            "name": "initialize_task",
            "arguments": {
                "task_description": "集成测试任务",
                "context_for_all_tasks": "测试上下文信息"
            }
        })),
    );

    let resp = server.handle_request(req);
    assert!(resp.error.is_none(), "initialize_task 不应返回错误");

    let result = result_of(&resp);
    // isError 是可选字段，可能不存在或为 false
    assert!(result.get("isError").is_none_or(|v| !v.as_bool().unwrap_or(true)));

    let content = result["content"].as_array().expect("content 应为数组");
    assert!(!content.is_empty(), "content 不应为空");
    assert_eq!(content[0]["type"], "text");

    // 解析返回的 task JSON
    let task_text = content[0]["text"].as_str().expect("text 应为字符串");
    let task: Value = serde_json::from_str(task_text).expect("应返回合法的 task JSON");
    assert_eq!(task["task_description"], "集成测试任务");
    assert_eq!(task["context_for_all_tasks"], "测试上下文信息");
    assert!(task["id"].is_string());
    assert!(!task["id"].as_str().unwrap().is_empty());
}

#[test]
fn test_tools_call_list_tasks() {
    let (mut server, _dir) = test_server();

    // 先创建两个任务
    for desc in ["任务A", "任务B"] {
        let req = make_request(
            "tools/call",
            10,
            Some(json!({
                "name": "initialize_task",
                "arguments": { "task_description": desc }
            })),
        );
        let _ = server.handle_request(req);
    }

    // 列出任务
    let req = make_request(
        "tools/call",
        11,
        Some(json!({
            "name": "list_tasks",
            "arguments": {}
        })),
    );
    let resp = server.handle_request(req);
    assert!(resp.error.is_none());

    let result = result_of(&resp);
    let content = result["content"].as_array().expect("content 应为数组");
    let text = content[0]["text"].as_str().unwrap();

    assert!(text.contains("任务A"), "列表应包含 '任务A'");
    assert!(text.contains("任务B"), "列表应包含 '任务B'");
}

#[test]
fn test_tools_call_add_checklist_item() {
    let (mut server, _dir) = test_server();

    // 创建任务
    let req = make_request(
        "tools/call",
        20,
        Some(json!({
            "name": "initialize_task",
            "arguments": { "task_description": "带检查项的任务" }
        })),
    );
    let resp = server.handle_request(req);
    let task: Value = serde_json::from_str(
        resp.result.unwrap()["content"][0]["text"].as_str().unwrap(),
    )
    .unwrap();
    let task_id = task["id"].as_str().unwrap();

    // 添加检查项
    let req = make_request(
        "tools/call",
        21,
        Some(json!({
            "name": "add_checklist_item",
            "arguments": {
                "task_id": task_id,
                "task": "步骤一",
                "detailed_description": "完成第一步操作",
                "context_and_plan": "参考文档A"
            }
        })),
    );
    let resp = server.handle_request(req);
    assert!(resp.error.is_none());

    let result = result_of(&resp);
    let task_text = result["content"][0]["text"].as_str().unwrap();
    let task: Value = serde_json::from_str(task_text).unwrap();
    assert_eq!(task["checklist"].as_array().unwrap().len(), 1);
    assert_eq!(task["checklist"][0]["task"], "步骤一");
    assert_eq!(task["checklist"][0]["done"], false);
}

#[test]
fn test_tools_call_mark_task_done_and_summary() {
    let (mut server, _dir) = test_server();

    // 创建带检查项的任务
    let req = make_request(
        "tools/call",
        30,
        Some(json!({
            "name": "initialize_task",
            "arguments": {
                "task_description": "进度测试",
                "initial_checklist": [
                    { "task": "步骤A", "detailed_description": "第一项" },
                    { "task": "步骤B", "detailed_description": "第二项" }
                ]
            }
        })),
    );
    let resp = server.handle_request(req);
    let task: Value = serde_json::from_str(
        resp.result.unwrap()["content"][0]["text"].as_str().unwrap(),
    )
    .unwrap();
    let task_id = task["id"].as_str().unwrap();

    // 标记第一个为完成
    let req = make_request(
        "tools/call",
        31,
        Some(json!({
            "name": "mark_task_done",
            "arguments": {
                "task_id": task_id,
                "index": 0
            }
        })),
    );
    let resp = server.handle_request(req);
    assert!(resp.error.is_none());

    let result_val = resp.result.unwrap();
    let task_text = result_val["content"][0]["text"].as_str().unwrap();
    let task: Value = serde_json::from_str(task_text).unwrap();
    assert_eq!(task["checklist"][0]["done"], true);
    assert_eq!(task["checklist"][1]["done"], false);

    // 获取摘要
    let req = make_request(
        "tools/call",
        32,
        Some(json!({
            "name": "get_checklist_summary",
            "arguments": {
                "task_id": task_id,
                "include_descriptions": true
            }
        })),
    );
    let resp = server.handle_request(req);
    assert!(resp.error.is_none());

    let result_val = resp.result.unwrap();
    let summary = result_val["content"][0]["text"].as_str().unwrap();
    assert!(summary.contains("进度: 1/2"), "摘要应显示进度 1/2");
    assert!(summary.contains("✅"), "已完成的条目应显示 ✅");
    assert!(summary.contains("⬜"), "未完成的条目应显示 ⬜");
}

#[test]
fn test_ping_returns_empty_result() {
    let (mut server, _dir) = test_server();

    let req = make_request("ping", 99, None);
    let resp = server.handle_request(req);

    assert!(resp.error.is_none());
    assert_eq!(result_of(&resp), &json!({}));
}

#[test]
fn test_unknown_method_returns_error() {
    let (mut server, _dir) = test_server();

    let req = make_request("nonexistent_method", 100, None);
    let resp = server.handle_request(req);

    assert!(resp.result.is_none(), "未知方法不应返回 result");
    let err = error_of(&resp);
    assert_eq!(err.code, -32601);
    assert!(err.message.contains("Method not found"));
}

#[test]
fn test_unknown_tool_returns_error() {
    let (mut server, _dir) = test_server();

    let req = make_request(
        "tools/call",
        101,
        Some(json!({
            "name": "nonexistent_tool",
            "arguments": {}
        })),
    );
    let resp = server.handle_request(req);

    assert!(resp.result.is_none());
    let err = error_of(&resp);
    assert_eq!(err.code, -32601);
    assert!(err.message.contains("Unknown tool"));
}
