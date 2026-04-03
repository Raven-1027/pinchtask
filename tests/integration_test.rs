//! MCP 协议集成测试（rmcp 版本）。
//!
//! 测试 PinchTaskServer 的 ServerHandler 实现、工具注册、以及端到端工具调用。
//! 核心业务逻辑的详细测试见 src/tools/task.rs 中的单元测试。

use rmcp::ServerHandler;
use rmcp::handler::server::wrapper::Parameters;
use tempfile::TempDir;

use mcp_pinchtask::server::PinchTaskServer;
use mcp_pinchtask::store::TaskStore;
use mcp_pinchtask::tools::params::*;

// ---------------------------------------------------------------------------
// 辅助函数
// ---------------------------------------------------------------------------

/// 创建使用临时目录的 PinchTaskServer 实例。
async fn test_server() -> (PinchTaskServer, TempDir) {
    let dir = tempfile::tempdir().expect("创建临时目录失败");
    let store = TaskStore::new(Some(dir.path().to_path_buf()))
        .await
        .expect("创建 TaskStore 失败");
    let server = PinchTaskServer::new(store);
    (server, dir)
}

/// 从 CallToolResult 中提取第一个 content 的 text 字段。
fn extract_text(result: &rmcp::model::CallToolResult) -> Option<String> {
    let json = serde_json::to_value(result).ok()?;
    json.get("content")?
        .get(0)?
        .get("text")?
        .as_str()
        .map(|s| s.to_owned())
}

// ---------------------------------------------------------------------------
// ServerHandler 基础测试
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_info_returns_correct_server_info() {
    let (server, _dir) = test_server().await;
    let info = server.get_info();

    assert_eq!(info.server_info.name, "mcp-pinchtask");
    assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
}

// ---------------------------------------------------------------------------
// 工具注册测试
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_all_17_tools_registered() {
    let (server, _dir) = test_server().await;

    let expected_tools = [
        "initialize_task",
        "update_task",
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

    for name in &expected_tools {
        let tool = server.get_tool(name);
        assert!(tool.is_some(), "工具 '{name}' 应已注册");
        let tool = tool.unwrap();
        assert_eq!(tool.name, *name, "工具名称应匹配");
        assert!(
            tool.description.is_some(),
            "工具 '{name}' 应有 description"
        );
        assert!(
            !tool.description.as_ref().unwrap().is_empty(),
            "工具 '{name}' 的 description 不应为空"
        );
    }

    // 确认不存在的工具返回 None
    assert!(server.get_tool("nonexistent_tool").is_none());
}

#[tokio::test]
async fn test_tool_schemas_are_valid() {
    let (server, _dir) = test_server().await;

    // 每个 tool 的 inputSchema 应该是一个合法的 JSON Schema 对象
    for name in [
        "initialize_task",
        "update_task",
        "add_checklist_item",
        "list_tasks",
    ] {
        let tool = server.get_tool(name).expect("工具应存在");
        let schema = &tool.input_schema;
        let schema_value = serde_json::to_value(schema).expect("inputSchema 应可序列化");
        assert!(
            schema_value.is_object(),
            "工具 '{name}' 的 inputSchema 应为 object"
        );
        assert!(
            schema_value.get("type").is_some(),
            "工具 '{name}' 的 inputSchema 应包含 type 字段"
        );
    }
}

// ---------------------------------------------------------------------------
// 端到端工具调用测试
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_initialize_task_creates_task() {
    let (server, _dir) = test_server().await;

    let result = server
        .initialize_task(Parameters(InitializeTaskParams {
            task_description: "集成测试任务".to_owned(),
            context_for_all_tasks: Some("测试上下文信息".to_owned()),
            initial_checklist: None,
            notes: None,
            resources: None,
            metadata: None,
        }))
        .await
        .expect("initialize_task 不应返回 ErrorData");

    assert_eq!(result.is_error, Some(false), "不应标记为错误");
    assert!(!result.content.is_empty(), "content 不应为空");

    let text = extract_text(&result).expect("应有 text 内容");
    let task: serde_json::Value =
        serde_json::from_str(&text).expect("text 应为合法 JSON");

    assert_eq!(task["task_description"], "集成测试任务");
    assert_eq!(task["context_for_all_tasks"], "测试上下文信息");
    assert!(task["id"].is_string());
    assert!(!task["id"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn test_initialize_task_with_checklist() {
    let (server, _dir) = test_server().await;

    let result = server
        .initialize_task(Parameters(InitializeTaskParams {
            task_description: "带检查项的任务".to_owned(),
            context_for_all_tasks: None,
            initial_checklist: Some(vec![
                InitialChecklistItem {
                    task: "步骤A".to_owned(),
                    detailed_description: "第一项".to_owned(),
                    context_and_plan: None,
                    done: false,
                    id: None,
                },
                InitialChecklistItem {
                    task: "步骤B".to_owned(),
                    detailed_description: "第二项".to_owned(),
                    context_and_plan: Some("计划B".to_owned()),
                    done: true,
                    id: None,
                },
            ]),
            notes: Some(vec!["一条笔记".to_owned()]),
            resources: None,
            metadata: None,
        }))
        .await
        .expect("initialize_task 不应返回 ErrorData");

    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();

    assert_eq!(task["checklist"].as_array().unwrap().len(), 2);
    assert_eq!(task["checklist"][0]["task"], "步骤A");
    assert_eq!(task["checklist"][0]["done"], false);
    assert_eq!(task["checklist"][1]["task"], "步骤B");
    assert_eq!(task["checklist"][1]["done"], true);
    assert_eq!(task["checklist"][1]["context_and_plan"], "计划B");
    assert_eq!(task["notes"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_add_checklist_item_and_mark_done() {
    let (server, _dir) = test_server().await;

    // 创建任务
    let result = server
        .initialize_task(Parameters(InitializeTaskParams {
            task_description: "测试任务".to_owned(),
            context_for_all_tasks: None,
            initial_checklist: None,
            notes: None,
            resources: None,
            metadata: None,
        }))
        .await
        .unwrap();
    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    let task_id = task["id"].as_str().unwrap().to_owned();

    // 添加检查项
    let result = server
        .add_checklist_item(Parameters(AddChecklistItemParams {
            task_id: task_id.clone(),
            task: "步骤一".to_owned(),
            detailed_description: "完成第一步操作".to_owned(),
            context_and_plan: Some("参考文档A".to_owned()),
        }))
        .await
        .unwrap();
    assert_eq!(result.is_error, Some(false));

    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(task["checklist"].as_array().unwrap().len(), 1);
    assert_eq!(task["checklist"][0]["task"], "步骤一");
    assert_eq!(task["checklist"][0]["done"], false);

    // 标记完成
    let result = server
        .mark_task_done(Parameters(MarkTaskDoneParams {
            task_id: task_id.clone(),
            index: 0,
        }))
        .await
        .unwrap();

    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(task["checklist"][0]["done"], true);
}

#[tokio::test]
async fn test_list_tasks_after_creating_two() {
    let (server, _dir) = test_server().await;

    for desc in ["任务A", "任务B"] {
        server
            .initialize_task(Parameters(InitializeTaskParams {
                task_description: desc.to_owned(),
                context_for_all_tasks: None,
                initial_checklist: None,
                notes: None,
                resources: None,
                metadata: None,
            }))
            .await
            .unwrap();
    }

    let result = server
        .list_tasks(Parameters(ListTasksParams {}))
        .await
        .unwrap();

    assert_eq!(result.is_error, Some(false));
    let text = extract_text(&result).unwrap();
    assert!(text.contains("任务A"), "列表应包含 '任务A'");
    assert!(text.contains("任务B"), "列表应包含 '任务B'");
}

#[tokio::test]
async fn test_mark_done_and_get_summary() {
    let (server, _dir) = test_server().await;

    // 创建带两个检查项的任务
    let result = server
        .initialize_task(Parameters(InitializeTaskParams {
            task_description: "进度测试".to_owned(),
            context_for_all_tasks: None,
            initial_checklist: Some(vec![
                InitialChecklistItem {
                    task: "步骤A".to_owned(),
                    detailed_description: "第一项".to_owned(),
                    context_and_plan: None,
                    done: false,
                    id: None,
                },
                InitialChecklistItem {
                    task: "步骤B".to_owned(),
                    detailed_description: "第二项".to_owned(),
                    context_and_plan: None,
                    done: false,
                    id: None,
                },
            ]),
            notes: None,
            resources: None,
            metadata: None,
        }))
        .await
        .unwrap();
    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    let task_id = task["id"].as_str().unwrap().to_owned();

    // 标记第一个完成
    server
        .mark_task_done(Parameters(MarkTaskDoneParams {
            task_id: task_id.clone(),
            index: 0,
        }))
        .await
        .unwrap();

    // 获取摘要
    let result = server
        .get_checklist_summary(Parameters(GetChecklistSummaryParams {
            task_id: task_id.clone(),
            include_descriptions: Some(true),
        }))
        .await
        .unwrap();

    assert_eq!(result.is_error, Some(false));
    let summary = extract_text(&result).unwrap();
    assert!(
        summary.contains("进度: 1/2"),
        "摘要应显示进度 1/2，实际: {summary}"
    );
    assert!(summary.contains("✅"), "已完成的条目应显示 ✅");
    assert!(summary.contains("⬜"), "未完成的条目应显示 ⬜");
}

#[tokio::test]
async fn test_add_note_and_resource() {
    let (server, _dir) = test_server().await;

    let result = server
        .initialize_task(Parameters(InitializeTaskParams {
            task_description: "笔记测试".to_owned(),
            context_for_all_tasks: None,
            initial_checklist: None,
            notes: None,
            resources: None,
            metadata: None,
        }))
        .await
        .unwrap();
    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    let task_id = task["id"].as_str().unwrap().to_owned();

    // 添加笔记
    let result = server
        .add_note(Parameters(AddNoteParams {
            task_id: task_id.clone(),
            content: "一条笔记".to_owned(),
        }))
        .await
        .unwrap();
    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(task["notes"].as_array().unwrap().len(), 1);
    assert_eq!(task["notes"][0], "一条笔记");

    // 添加资源
    let result = server
        .add_resource(Parameters(AddResourceParams {
            task_id: task_id.clone(),
            name: "文档".to_owned(),
            url: "https://example.com".to_owned(),
            description: Some("示例文档".to_owned()),
        }))
        .await
        .unwrap();
    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(task["resources"].as_array().unwrap().len(), 1);
    assert_eq!(task["resources"][0]["name"], "文档");
    assert_eq!(task["resources"][0]["url"], "https://example.com");
}

#[tokio::test]
async fn test_update_task_unified() {
    let (server, _dir) = test_server().await;

    let result = server
        .initialize_task(Parameters(InitializeTaskParams {
            task_description: "原始描述".to_owned(),
            context_for_all_tasks: None,
            initial_checklist: None,
            notes: None,
            resources: None,
            metadata: None,
        }))
        .await
        .unwrap();
    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    let task_id = task["id"].as_str().unwrap().to_owned();

    // 统一更新 description + priority + tags
    let result = server
        .update_task(Parameters(UpdateTaskParams {
            task_id: task_id.clone(),
            task_description: Some("更新后的描述".to_owned()),
            context_for_all_tasks: None,
            priority: Some("high".to_owned()),
            tags: Some("tag1,tag2".to_owned()),
            eta: None,
        }))
        .await
        .unwrap();

    assert_eq!(result.is_error, Some(false));
    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(task["task_description"], "更新后的描述");
    assert_eq!(task["metadata"]["priority"], "high");
    let tags = task["metadata"]["tags"].as_array().unwrap();
    assert_eq!(tags.len(), 2);
    assert!(tags.iter().any(|t| t.as_str() == Some("tag1")));
    assert!(tags.iter().any(|t| t.as_str() == Some("tag2")));
}

#[tokio::test]
async fn test_update_task_with_no_fields_returns_error() {
    let (server, _dir) = test_server().await;

    let result = server
        .initialize_task(Parameters(InitializeTaskParams {
            task_description: "测试".to_owned(),
            context_for_all_tasks: None,
            initial_checklist: None,
            notes: None,
            resources: None,
            metadata: None,
        }))
        .await
        .unwrap();
    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    let task_id = task["id"].as_str().unwrap().to_owned();

    // 不指定任何可修改字段
    let result = server
        .update_task(Parameters(UpdateTaskParams {
            task_id,
            task_description: None,
            context_for_all_tasks: None,
            priority: None,
            tags: None,
            eta: None,
        }))
        .await
        .unwrap();

    assert_eq!(result.is_error, Some(true), "应标记为错误");
}

#[tokio::test]
async fn test_clear_task_deletes_task() {
    let (server, _dir) = test_server().await;

    let result = server
        .initialize_task(Parameters(InitializeTaskParams {
            task_description: "待删除任务".to_owned(),
            context_for_all_tasks: None,
            initial_checklist: None,
            notes: None,
            resources: None,
            metadata: None,
        }))
        .await
        .unwrap();
    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    let task_id = task["id"].as_str().unwrap().to_owned();

    let result = server
        .clear_task(Parameters(ClearTaskParams {
            task_id: task_id.clone(),
        }))
        .await
        .unwrap();

    assert_eq!(result.is_error, Some(false));
    let text = extract_text(&result).unwrap();
    assert!(text.contains("已删除"), "应提示已删除");

    // 后续操作该任务应返回错误
    let result = server
        .mark_task_done(Parameters(MarkTaskDoneParams {
            task_id,
            index: 0,
        }))
        .await
        .unwrap();
    assert_eq!(result.is_error, Some(true), "已删除的任务操作应返回错误");
}

#[tokio::test]
async fn test_reorder_checklist_item() {
    let (server, _dir) = test_server().await;

    let result = server
        .initialize_task(Parameters(InitializeTaskParams {
            task_description: "排序测试".to_owned(),
            context_for_all_tasks: None,
            initial_checklist: Some(vec![
                InitialChecklistItem {
                    task: "第一".to_owned(),
                    detailed_description: "1".to_owned(),
                    context_and_plan: None,
                    done: false,
                    id: None,
                },
                InitialChecklistItem {
                    task: "第二".to_owned(),
                    detailed_description: "2".to_owned(),
                    context_and_plan: None,
                    done: false,
                    id: None,
                },
                InitialChecklistItem {
                    task: "第三".to_owned(),
                    detailed_description: "3".to_owned(),
                    context_and_plan: None,
                    done: false,
                    id: None,
                },
            ]),
            notes: None,
            resources: None,
            metadata: None,
        }))
        .await
        .unwrap();
    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    let task_id = task["id"].as_str().unwrap().to_owned();

    // 把 index 0 移到 index 2
    let result = server
        .reorder_checklist_item(Parameters(ReorderChecklistItemParams {
            task_id: task_id.clone(),
            from_index: 0,
            to_index: 2,
        }))
        .await
        .unwrap();

    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    let checklist = task["checklist"].as_array().unwrap();
    assert_eq!(checklist[0]["task"], "第二");
    assert_eq!(checklist[1]["task"], "第三");
    assert_eq!(checklist[2]["task"], "第一");
}

#[tokio::test]
async fn test_remove_checklist_item() {
    let (server, _dir) = test_server().await;

    let result = server
        .initialize_task(Parameters(InitializeTaskParams {
            task_description: "删除测试".to_owned(),
            context_for_all_tasks: None,
            initial_checklist: Some(vec![
                InitialChecklistItem {
                    task: "保留".to_owned(),
                    detailed_description: "保留".to_owned(),
                    context_and_plan: None,
                    done: false,
                    id: None,
                },
                InitialChecklistItem {
                    task: "删除".to_owned(),
                    detailed_description: "删除".to_owned(),
                    context_and_plan: None,
                    done: false,
                    id: None,
                },
            ]),
            notes: None,
            resources: None,
            metadata: None,
        }))
        .await
        .unwrap();
    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    let task_id = task["id"].as_str().unwrap().to_owned();

    let result = server
        .remove_checklist_item(Parameters(RemoveChecklistItemParams {
            task_id: task_id.clone(),
            index: 1,
        }))
        .await
        .unwrap();

    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(task["checklist"].as_array().unwrap().len(), 1);
    assert_eq!(task["checklist"][0]["task"], "保留");
}

#[tokio::test]
async fn test_get_current_task_details_no_tasks() {
    let (server, _dir) = test_server().await;

    let result = server
        .get_current_task_details(Parameters(GetCurrentTaskDetailsParams {}))
        .await
        .unwrap();

    assert_eq!(result.is_error, Some(true), "无任务时应返回错误");
    let text = extract_text(&result).unwrap();
    assert!(
        text.contains("没有找到"),
        "应提示没有找到未完成任务"
    );
}

#[tokio::test]
async fn test_get_current_task_details_with_uncompleted() {
    let (server, _dir) = test_server().await;

    // 创建一个有未完成子任务的任务
    server
        .initialize_task(Parameters(InitializeTaskParams {
            task_description: "当前任务".to_owned(),
            context_for_all_tasks: Some("共享上下文".to_owned()),
            initial_checklist: Some(vec![InitialChecklistItem {
                task: "待办".to_owned(),
                detailed_description: "待办详情".to_owned(),
                context_and_plan: None,
                done: false,
                id: None,
            }]),
            notes: None,
            resources: None,
            metadata: None,
        }))
        .await
        .unwrap();

    let result = server
        .get_current_task_details(Parameters(GetCurrentTaskDetailsParams {}))
        .await
        .unwrap();

    assert_eq!(result.is_error, Some(false));
    let text = extract_text(&result).unwrap();
    assert!(text.contains("当前任务"), "应包含任务描述");
    assert!(text.contains("共享上下文"), "应包含上下文信息");
}

#[tokio::test]
async fn test_update_metadata() {
    let (server, _dir) = test_server().await;

    let result = server
        .initialize_task(Parameters(InitializeTaskParams {
            task_description: "元数据测试".to_owned(),
            context_for_all_tasks: None,
            initial_checklist: None,
            notes: None,
            resources: None,
            metadata: None,
        }))
        .await
        .unwrap();
    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    let task_id = task["id"].as_str().unwrap().to_owned();

    let result = server
        .update_metadata(Parameters(UpdateMetadataParams {
            task_id,
            metadata: TaskMetadataInput {
                tags: Some(vec!["rust".to_owned(), "mcp".to_owned()]),
                priority: Some("high".to_owned()),
                estimated_completion_time: Some("2025-12-31".to_owned()),
            },
        }))
        .await
        .unwrap();

    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(task["metadata"]["priority"], "high");
    assert_eq!(
        task["metadata"]["estimated_completion_time"],
        "2025-12-31"
    );
    let tags = task["metadata"]["tags"].as_array().unwrap();
    assert_eq!(tags.len(), 2);
}

#[tokio::test]
async fn test_update_context_and_description() {
    let (server, _dir) = test_server().await;

    let result = server
        .initialize_task(Parameters(InitializeTaskParams {
            task_description: "原始".to_owned(),
            context_for_all_tasks: None,
            initial_checklist: None,
            notes: None,
            resources: None,
            metadata: None,
        }))
        .await
        .unwrap();
    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    let task_id = task["id"].as_str().unwrap().to_owned();

    // 更新上下文
    let result = server
        .update_context(Parameters(UpdateContextParams {
            task_id: task_id.clone(),
            context_for_all_tasks: "新上下文".to_owned(),
        }))
        .await
        .unwrap();
    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(task["context_for_all_tasks"], "新上下文");

    // 更新描述
    let result = server
        .update_task_description(Parameters(UpdateTaskDescriptionParams {
            task_id,
            task_description: "新描述".to_owned(),
        }))
        .await
        .unwrap();
    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(task["task_description"], "新描述");
}

#[tokio::test]
async fn test_mark_undone() {
    let (server, _dir) = test_server().await;

    let result = server
        .initialize_task(Parameters(InitializeTaskParams {
            task_description: "撤销测试".to_owned(),
            context_for_all_tasks: None,
            initial_checklist: Some(vec![InitialChecklistItem {
                task: "步骤".to_owned(),
                detailed_description: "详情".to_owned(),
                context_and_plan: None,
                done: true,
                id: None,
            }]),
            notes: None,
            resources: None,
            metadata: None,
        }))
        .await
        .unwrap();
    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    let task_id = task["id"].as_str().unwrap().to_owned();
    assert_eq!(task["checklist"][0]["done"], true);

    let result = server
        .mark_task_undone(Parameters(MarkTaskUndoneParams {
            task_id,
            index: 0,
        }))
        .await
        .unwrap();
    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(task["checklist"][0]["done"], false);
}

#[tokio::test]
async fn test_update_checklist_item_partial() {
    let (server, _dir) = test_server().await;

    let result = server
        .initialize_task(Parameters(InitializeTaskParams {
            task_description: "部分更新测试".to_owned(),
            context_for_all_tasks: None,
            initial_checklist: Some(vec![InitialChecklistItem {
                task: "原始名称".to_owned(),
                detailed_description: "原始描述".to_owned(),
                context_and_plan: Some("原始计划".to_owned()),
                done: false,
                id: None,
            }]),
            notes: None,
            resources: None,
            metadata: None,
        }))
        .await
        .unwrap();
    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    let task_id = task["id"].as_str().unwrap().to_owned();

    // 只更新 task 名称
    let result = server
        .update_checklist_item(Parameters(UpdateChecklistItemParams {
            task_id: task_id.clone(),
            index: 0,
            task: Some("新名称".to_owned()),
            detailed_description: None,
            context_and_plan: None,
            done: None,
        }))
        .await
        .unwrap();

    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(task["checklist"][0]["task"], "新名称");
    assert_eq!(task["checklist"][0]["detailed_description"], "原始描述");
    assert_eq!(
        task["checklist"][0]["context_and_plan"],
        "原始计划"
    );

    // 用 null 清空 context_and_plan
    let result = server
        .update_checklist_item(Parameters(UpdateChecklistItemParams {
            task_id,
            index: 0,
            task: None,
            detailed_description: None,
            context_and_plan: Some(None), // 传入 null
            done: None,
        }))
        .await
        .unwrap();

    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(
        task["checklist"][0]["context_and_plan"].is_null(),
        "context_and_plan 应被清空为 null"
    );
}
