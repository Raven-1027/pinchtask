-- pinchtask 初始数据库 Schema
-- 采用混合范式化设计：主表 + checklist_items/notes/resources 独立表，metadata 为 JSON 列

CREATE TABLE IF NOT EXISTS tasks (
    id              TEXT PRIMARY KEY,
    task_description    TEXT NOT NULL,
    context_for_all_tasks   TEXT,
    metadata        TEXT,                   -- JSON: {"tags":[...],"priority":"...","estimated_completion_time":"..."}
    created_at      TEXT NOT NULL,          -- ISO 8601
    updated_at      TEXT NOT NULL           -- ISO 8601
);

CREATE TABLE IF NOT EXISTS checklist_items (
    id              TEXT PRIMARY KEY,
    task_id         TEXT NOT NULL,
    sort_order      INTEGER NOT NULL,
    task            TEXT NOT NULL,
    detailed_description  TEXT NOT NULL DEFAULT '',
    context_and_plan      TEXT,
    done            INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS notes (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id         TEXT NOT NULL,
    sort_order      INTEGER NOT NULL,
    content         TEXT NOT NULL,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS resources (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id         TEXT NOT NULL,
    name            TEXT NOT NULL,
    url             TEXT NOT NULL,
    description     TEXT,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE
);

-- 索引：按任务快速查询关联数据
CREATE INDEX IF NOT EXISTS idx_checklist_items_task_id ON checklist_items(task_id, sort_order);
CREATE INDEX IF NOT EXISTS idx_notes_task_id ON notes(task_id, sort_order);
CREATE INDEX IF NOT EXISTS idx_resources_task_id ON resources(task_id);

-- 索引：按创建时间排序列出任务
CREATE INDEX IF NOT EXISTS idx_tasks_created_at ON tasks(created_at);
