-- pinchtask 项目关系重构迁移
-- 将 task-project 关系从多对多改为一对多：task 加可空 project_id 外键，删 task_projects 关联表

-- 1. 为 tasks 表添加 project_id 列
ALTER TABLE tasks ADD COLUMN project_id TEXT REFERENCES projects(id) ON DELETE SET NULL;

-- 2. 迁移现有数据：每个 task 取第一个关联的 project_id
UPDATE tasks SET project_id = (
    SELECT tp.project_id FROM task_projects tp
    WHERE tp.task_id = tasks.id
    ORDER BY tp.project_id
    LIMIT 1
) WHERE EXISTS (
    SELECT 1 FROM task_projects tp WHERE tp.task_id = tasks.id
);

-- 3. 删除 task_projects 关联表
DROP TABLE IF EXISTS task_projects;

-- 4. 索引：按 project_id 查询关联任务
CREATE INDEX IF NOT EXISTS idx_tasks_project_id ON tasks(project_id);
