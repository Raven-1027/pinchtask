-- pinchtask 项目管理功能迁移
-- 新增 projects 表存储项目元数据，task_projects 关联表实现任务与项目的多对多关系

CREATE TABLE IF NOT EXISTS projects (
    id              TEXT PRIMARY KEY,
    name            TEXT NOT NULL,
    description     TEXT,
    created_at      TEXT NOT NULL,          -- ISO 8601
    updated_at      TEXT NOT NULL           -- ISO 8601
);

CREATE TABLE IF NOT EXISTS task_projects (
    task_id         TEXT NOT NULL,
    project_id      TEXT NOT NULL,
    PRIMARY KEY (task_id, project_id),
    FOREIGN KEY (task_id)    REFERENCES tasks(id)    ON DELETE CASCADE,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

-- 索引：按项目查询关联任务
CREATE INDEX IF NOT EXISTS idx_task_projects_project_id ON task_projects(project_id);

-- 索引：按任务查询关联项目（复合主键 task_id 在前，已有覆盖，但为显式表达意图单独建立）
CREATE INDEX IF NOT EXISTS idx_task_projects_task_id ON task_projects(task_id);

-- 索引：按创建时间排序列出项目
CREATE INDEX IF NOT EXISTS idx_projects_created_at ON projects(created_at);
