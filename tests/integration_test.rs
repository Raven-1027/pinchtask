//! MCP 协议集成测试（rmcp 版本）。
//!
//! 测试 PinchTaskServer 的 ServerHandler 实现、工具注册、以及端到端工具调用。
//! 核心业务逻辑的详细测试见 src/tools/task.rs 中的单元测试。

use rmcp::handler::server::wrapper::Parameters;
use rmcp::ServerHandler;
use tempfile::TempDir;

use pinchtask::server::PinchTaskServer;
use pinchtask::store::TaskStore;
use pinchtask::tools::params::*;

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

    assert_eq!(info.server_info.name, "pinchtask");
    assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
}

// ---------------------------------------------------------------------------
// 工具注册测试
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_all_9_tools_registered() {
    let (server, _dir) = test_server().await;

    let expected_tools = [
        "new_task",
        "update_task",
        "manage_checklist_item",
        "add_note",
        "add_resource",
        "get_checklist_summary",
        "clear_task",
        "list_tasks",
        "manage_project",
    ];

    for name in &expected_tools {
        let tool = server.get_tool(name);
        assert!(tool.is_some(), "工具 '{name}' 应已注册");
        let tool = tool.unwrap();
        assert_eq!(tool.name, *name, "工具名称应匹配");
        assert!(tool.description.is_some(), "工具 '{name}' 应有 description");
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
        "new_task",
        "update_task",
        "manage_checklist_item",
        "list_tasks",
    ] {
        let tool = server.get_tool(name).expect("工具应存在");
        let schema = &tool.input_schema;
        let schema_value = serde_json::to_value(schema).expect("inputSchema 应可序列化");
        assert!(
            schema_value.is_object(),
            "工具 '{name}' 的 inputSchema 应为 object"
        );
        // manage_checklist_item 使用 tagged enum，schema 为 oneOf 结构，无顶层 type 字段
        if name != "manage_checklist_item" {
            assert!(
                schema_value.get("type").is_some(),
                "工具 '{name}' 的 inputSchema 应包含 type 字段"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// 端到端工具调用测试
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_initialize_task_creates_task() {
    let (server, _dir) = test_server().await;

    let result = server
        .new_task(Parameters(InitializeTaskParams {
            task_description: "集成测试任务".to_owned(),
            context_for_all_tasks: Some("测试上下文信息".to_owned()),
            initial_checklist: None,
            notes: None,
            resources: None,
            metadata: None,
            project_id: None,
        }))
        .await
        .expect("initialize_task 不应返回 ErrorData");

    assert_eq!(result.is_error, Some(false), "不应标记为错误");
    assert!(!result.content.is_empty(), "content 不应为空");

    let text = extract_text(&result).expect("应有 text 内容");
    let task: serde_json::Value = serde_json::from_str(&text).expect("text 应为合法 JSON");

    assert_eq!(task["task_description"], "集成测试任务");
    assert_eq!(task["context_for_all_tasks"], "测试上下文信息");
    assert!(task["id"].is_string());
    assert!(!task["id"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn test_initialize_task_with_checklist() {
    let (server, _dir) = test_server().await;

    let result = server
        .new_task(Parameters(InitializeTaskParams {
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
            project_id: None,
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
        .new_task(Parameters(InitializeTaskParams {
            task_description: "测试任务".to_owned(),
            context_for_all_tasks: None,
            initial_checklist: None,
            notes: None,
            resources: None,
            metadata: None,
            project_id: None,
        }))
        .await
        .unwrap();
    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    let task_id = task["id"].as_str().unwrap().to_owned();

    // 添加检查项
    let result = server
        .manage_checklist_item(Parameters(ManageChecklistItemParams {
            action: Action::Add,
            task_id: task_id.clone(),
            task: Some("步骤一".to_owned()),
            detailed_description: Some("完成第一步操作".to_owned()),
            context_and_plan: Some(Some("参考文档A".to_owned())),
            ..Default::default()
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
        .manage_checklist_item(Parameters(ManageChecklistItemParams {
            action: Action::Update,
            task_id: task_id.clone(),
            index: Some(0),
            done: Some(true),
            ..Default::default()
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
            .new_task(Parameters(InitializeTaskParams {
                task_description: desc.to_owned(),
                context_for_all_tasks: None,
                initial_checklist: None,
                notes: None,
                resources: None,
                metadata: None,
                project_id: None,
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
        .new_task(Parameters(InitializeTaskParams {
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
            project_id: None,
        }))
        .await
        .unwrap();
    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    let task_id = task["id"].as_str().unwrap().to_owned();

    // 标记第一个完成
    server
        .manage_checklist_item(Parameters(ManageChecklistItemParams {
            action: Action::Update,
            task_id: task_id.clone(),
            index: Some(0),
            done: Some(true),
            ..Default::default()
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
        .new_task(Parameters(InitializeTaskParams {
            task_description: "笔记测试".to_owned(),
            context_for_all_tasks: None,
            initial_checklist: None,
            notes: None,
            resources: None,
            metadata: None,
            project_id: None,
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
        .new_task(Parameters(InitializeTaskParams {
            task_description: "原始描述".to_owned(),
            context_for_all_tasks: None,
            initial_checklist: None,
            notes: None,
            resources: None,
            metadata: None,
            project_id: None,
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
        .new_task(Parameters(InitializeTaskParams {
            task_description: "测试".to_owned(),
            context_for_all_tasks: None,
            initial_checklist: None,
            notes: None,
            resources: None,
            metadata: None,
            project_id: None,
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
        .new_task(Parameters(InitializeTaskParams {
            task_description: "待删除任务".to_owned(),
            context_for_all_tasks: None,
            initial_checklist: None,
            notes: None,
            resources: None,
            metadata: None,
            project_id: None,
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
        .manage_checklist_item(Parameters(ManageChecklistItemParams {
            action: Action::Update,
            task_id,
            index: Some(0),
            done: Some(true),
            ..Default::default()
        }))
        .await
        .unwrap();
    assert_eq!(result.is_error, Some(true), "已删除的任务操作应返回错误");
}

#[tokio::test]
async fn test_reorder_checklist_item() {
    let (server, _dir) = test_server().await;

    let result = server
        .new_task(Parameters(InitializeTaskParams {
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
            project_id: None,
        }))
        .await
        .unwrap();
    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    let task_id = task["id"].as_str().unwrap().to_owned();

    // 把 index 0 移到 index 2
    let result = server
        .manage_checklist_item(Parameters(ManageChecklistItemParams {
            action: Action::Reorder,
            task_id: task_id.clone(),
            from_index: Some(0),
            to_index: Some(2),
            ..Default::default()
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
        .new_task(Parameters(InitializeTaskParams {
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
            project_id: None,
        }))
        .await
        .unwrap();
    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    let task_id = task["id"].as_str().unwrap().to_owned();

    let result = server
        .manage_checklist_item(Parameters(ManageChecklistItemParams {
            action: Action::Remove,
            task_id: task_id.clone(),
            index: Some(1),
            ..Default::default()
        }))
        .await
        .unwrap();

    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(task["checklist"].as_array().unwrap().len(), 1);
    assert_eq!(task["checklist"][0]["task"], "保留");
}

#[tokio::test]
async fn test_update_metadata() {
    let (server, _dir) = test_server().await;

    let result = server
        .new_task(Parameters(InitializeTaskParams {
            task_description: "元数据测试".to_owned(),
            context_for_all_tasks: None,
            initial_checklist: None,
            notes: None,
            resources: None,
            metadata: None,
            project_id: None,
        }))
        .await
        .unwrap();
    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    let task_id = task["id"].as_str().unwrap().to_owned();

    let result = server
        .update_task(Parameters(UpdateTaskParams {
            task_id,
            task_description: None,
            context_for_all_tasks: None,
            priority: Some("high".to_owned()),
            tags: Some("rust,mcp".to_owned()),
            eta: Some("2025-12-31".to_owned()),
        }))
        .await
        .unwrap();

    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(task["metadata"]["priority"], "high");
    assert_eq!(task["metadata"]["estimated_completion_time"], "2025-12-31");
    let tags = task["metadata"]["tags"].as_array().unwrap();
    assert_eq!(tags.len(), 2);
}

#[tokio::test]
async fn test_update_context_and_description() {
    let (server, _dir) = test_server().await;

    let result = server
        .new_task(Parameters(InitializeTaskParams {
            task_description: "原始".to_owned(),
            context_for_all_tasks: None,
            initial_checklist: None,
            notes: None,
            resources: None,
            metadata: None,
            project_id: None,
        }))
        .await
        .unwrap();
    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    let task_id = task["id"].as_str().unwrap().to_owned();

    // 更新上下文
    let result = server
        .update_task(Parameters(UpdateTaskParams {
            task_id: task_id.clone(),
            task_description: None,
            context_for_all_tasks: Some("新上下文".to_owned()),
            priority: None,
            tags: None,
            eta: None,
        }))
        .await
        .unwrap();
    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(task["context_for_all_tasks"], "新上下文");

    // 更新描述
    let result = server
        .update_task(Parameters(UpdateTaskParams {
            task_id,
            task_description: Some("新描述".to_owned()),
            context_for_all_tasks: None,
            priority: None,
            tags: None,
            eta: None,
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
        .new_task(Parameters(InitializeTaskParams {
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
            project_id: None,
        }))
        .await
        .unwrap();
    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    let task_id = task["id"].as_str().unwrap().to_owned();
    assert_eq!(task["checklist"][0]["done"], true);

    let result = server
        .manage_checklist_item(Parameters(ManageChecklistItemParams {
            action: Action::Update,
            task_id,
            index: Some(0),
            done: Some(false),
            ..Default::default()
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
        .new_task(Parameters(InitializeTaskParams {
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
            project_id: None,
        }))
        .await
        .unwrap();
    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    let task_id = task["id"].as_str().unwrap().to_owned();

    // 只更新 task 名称
    let result = server
        .manage_checklist_item(Parameters(ManageChecklistItemParams {
            action: Action::Update,
            task_id: task_id.clone(),
            index: Some(0),
            task: Some("新名称".to_owned()),
            ..Default::default()
        }))
        .await
        .unwrap();

    let text = extract_text(&result).unwrap();
    let task: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(task["checklist"][0]["task"], "新名称");
    assert_eq!(task["checklist"][0]["detailed_description"], "原始描述");
    assert_eq!(task["checklist"][0]["context_and_plan"], "原始计划");

    // 用 null 清空 context_and_plan
    let result = server
        .manage_checklist_item(Parameters(ManageChecklistItemParams {
            action: Action::Update,
            task_id,
            index: Some(0),
            context_and_plan: Some(None), // 传入 null
            ..Default::default()
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
