use sqlx::{sqlite::SqlitePoolOptions, Row, SqlitePool};
use tauri::{AppHandle, Manager};

const MIGRATION_V1: &str = "
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    model_config_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    pinned INTEGER NOT NULL DEFAULT 0,
    archived INTEGER NOT NULL DEFAULT 0,
    system_prompt TEXT
);

CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL DEFAULT '',
    version INTEGER NOT NULL DEFAULT 1,
    parent_id TEXT,
    created_at TEXT NOT NULL,
    token_count INTEGER NOT NULL DEFAULT 0,
    error TEXT,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS model_configs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    base_url TEXT,
    api_key_name TEXT,
    api_key TEXT,
    max_tokens INTEGER NOT NULL DEFAULT 4096,
    context_window INTEGER NOT NULL DEFAULT 128000,
    temperature REAL NOT NULL DEFAULT 0.7,
    top_p REAL NOT NULL DEFAULT 1.0,
    temperature_enabled INTEGER NOT NULL DEFAULT 1,
    top_p_enabled INTEGER NOT NULL DEFAULT 1,
    system_prompt TEXT,
    reasoning_effort TEXT,
    passback_reasoning INTEGER NOT NULL DEFAULT 0,
    retry_count INTEGER NOT NULL DEFAULT 3,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS api_logs (
    id TEXT PRIMARY KEY,
    session_id TEXT,
    model TEXT,
    provider TEXT,
    request_body TEXT,
    response_body TEXT,
    status_code INTEGER,
    latency_ms INTEGER,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS attachments (
    id TEXT PRIMARY KEY,
    message_id TEXT NOT NULL,
    file_name TEXT NOT NULL,
    file_path TEXT,
    mime_type TEXT,
    size INTEGER,
    created_at TEXT NOT NULL,
    FOREIGN KEY (message_id) REFERENCES messages(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS search_configs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    model TEXT NOT NULL,
    base_url TEXT,
    api_key_name TEXT,
    api_key TEXT,
    max_tokens INTEGER NOT NULL DEFAULT 4096,
    context_window INTEGER NOT NULL DEFAULT 128000,
    temperature REAL NOT NULL DEFAULT 0.7,
    top_p REAL NOT NULL DEFAULT 1.0,
    temperature_enabled INTEGER NOT NULL DEFAULT 1,
    top_p_enabled INTEGER NOT NULL DEFAULT 1,
    reasoning_effort TEXT,
    prompt_template TEXT NOT NULL DEFAULT '请搜索以下内容并总结: {query}',
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL
);
";

/// 检测列是否存在
async fn has_column(pool: &SqlitePool, table: &str, column: &str) -> bool {
    let sql = format!("PRAGMA table_info({})", table);
    let rows = sqlx::query(&sql).fetch_all(pool).await.unwrap_or_default();
    rows.iter().any(|r| {
        let name: String = r.get("name");
        name == column
    })
}

/// 执行表结构升级
async fn migrate_schema(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    // 兼容旧版本 sessions 表
    if !has_column(pool, "sessions", "system_prompt").await {
        sqlx::query("ALTER TABLE sessions ADD COLUMN system_prompt TEXT")
            .execute(pool)
            .await?;
    }
    // 搜索配置表结构变化(从 model_config_id 模式迁移到独立配置)
    if has_column(pool, "search_configs", "model_config_id").await {
        // 旧表更名 + 建新表 + 复制公共列, 用事务保证原子性
        let mut tx = pool.begin().await?;
        sqlx::query("ALTER TABLE search_configs RENAME TO search_configs_old")
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "CREATE TABLE search_configs (\
                id TEXT PRIMARY KEY,\
                name TEXT NOT NULL,\
                model TEXT NOT NULL,\
                base_url TEXT,\
                api_key_name TEXT,\
                max_tokens INTEGER NOT NULL DEFAULT 4096,\
                context_window INTEGER NOT NULL DEFAULT 128000,\
                temperature REAL NOT NULL DEFAULT 0.7,\
                top_p REAL NOT NULL DEFAULT 1.0,\
                temperature_enabled INTEGER NOT NULL DEFAULT 1,\
                top_p_enabled INTEGER NOT NULL DEFAULT 1,\
                reasoning_effort TEXT,\
                prompt_template TEXT NOT NULL DEFAULT '请搜索以下内容并总结: {query}',\
                enabled INTEGER NOT NULL DEFAULT 1,\
                created_at TEXT NOT NULL\
            )"
        )
        .execute(&mut *tx)
        .await?;
        // 探测旧表实际拥有的列, 仅复制交集
        let old_cols_rows = sqlx::query("PRAGMA table_info(search_configs_old)")
            .fetch_all(&mut *tx)
            .await?;
        let old_cols: std::collections::HashSet<String> = old_cols_rows
            .iter()
            .map(|r| r.get::<String, _>("name"))
            .collect();
        // 新表所有列(顺序固定, 缺失列由 DEFAULT 兜底)
        let candidate_cols = [
            "id", "name", "model", "base_url", "api_key_name",
            "max_tokens", "temperature", "top_p", "reasoning_effort",
            "prompt_template", "enabled", "created_at",
        ];
        let common: Vec<&str> = candidate_cols
            .iter()
            .copied()
            .filter(|c| old_cols.contains(*c))
            .collect();
        if !common.is_empty() {
            let col_list = common.join(", ");
            let copy_sql = format!(
                "INSERT INTO search_configs ({}) SELECT {} FROM search_configs_old",
                col_list, col_list
            );
            sqlx::query(&copy_sql).execute(&mut *tx).await?;
        }
        sqlx::query("DROP TABLE search_configs_old")
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
    }
    // 主模型添加推理强度字段
    if !has_column(pool, "model_configs", "reasoning_effort").await {
        sqlx::query("ALTER TABLE model_configs ADD COLUMN reasoning_effort TEXT")
            .execute(pool)
            .await?;
    }
    // 主模型添加上下文窗口和参数开关
    if !has_column(pool, "model_configs", "context_window").await {
        sqlx::query("ALTER TABLE model_configs ADD COLUMN context_window INTEGER NOT NULL DEFAULT 128000")
            .execute(pool)
            .await?;
    }
    if !has_column(pool, "model_configs", "temperature_enabled").await {
        sqlx::query("ALTER TABLE model_configs ADD COLUMN temperature_enabled INTEGER NOT NULL DEFAULT 1")
            .execute(pool)
            .await?;
    }
    if !has_column(pool, "model_configs", "top_p_enabled").await {
        sqlx::query("ALTER TABLE model_configs ADD COLUMN top_p_enabled INTEGER NOT NULL DEFAULT 1")
            .execute(pool)
            .await?;
    }
    // 搜索配置同步加列
    if !has_column(pool, "search_configs", "context_window").await {
        sqlx::query("ALTER TABLE search_configs ADD COLUMN context_window INTEGER NOT NULL DEFAULT 128000")
            .execute(pool)
            .await?;
    }
    if !has_column(pool, "search_configs", "temperature_enabled").await {
        sqlx::query("ALTER TABLE search_configs ADD COLUMN temperature_enabled INTEGER NOT NULL DEFAULT 1")
            .execute(pool)
            .await?;
    }
    if !has_column(pool, "search_configs", "top_p_enabled").await {
        sqlx::query("ALTER TABLE search_configs ADD COLUMN top_p_enabled INTEGER NOT NULL DEFAULT 1")
            .execute(pool)
            .await?;
    }
    // 主模型添加 API Key 明文备份列
    if !has_column(pool, "model_configs", "api_key").await {
        sqlx::query("ALTER TABLE model_configs ADD COLUMN api_key TEXT")
            .execute(pool)
            .await?;
    }
    // 搜索配置添加 API Key 明文备份列
    if !has_column(pool, "search_configs", "api_key").await {
        sqlx::query("ALTER TABLE search_configs ADD COLUMN api_key TEXT")
            .execute(pool)
            .await?;
    }
    // 消息添加思维链字段
    if !has_column(pool, "messages", "thinking").await {
        sqlx::query("ALTER TABLE messages ADD COLUMN thinking TEXT")
            .execute(pool)
            .await?;
    }
    // 主模型添加思维回传开关
    if !has_column(pool, "model_configs", "passback_reasoning").await {
        sqlx::query("ALTER TABLE model_configs ADD COLUMN passback_reasoning INTEGER NOT NULL DEFAULT 0")
            .execute(pool)
            .await?;
    }
    // 主模型添加重试次数
    if !has_column(pool, "model_configs", "retry_count").await {
        sqlx::query("ALTER TABLE model_configs ADD COLUMN retry_count INTEGER NOT NULL DEFAULT 3")
            .execute(pool)
            .await?;
    }
    Ok(())
}

/// 初始化数据库连接池并执行迁移
pub async fn init(app: &AppHandle) -> Result<SqlitePool, Box<dyn std::error::Error>> {
    let app_dir = app.path().app_config_dir()?;
    std::fs::create_dir_all(&app_dir)?;

    let db_path = app_dir.join("chatllm.db");
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;

    sqlx::query("PRAGMA journal_mode=WAL;")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA foreign_keys=ON;")
        .execute(&pool)
        .await?;

    // 初次建表
    for statement in MIGRATION_V1.split(';') {
        let trimmed = statement.trim();
        if !trimmed.is_empty() {
            sqlx::query(trimmed).execute(&pool).await?;
        }
    }

    // 旧库结构升级
    migrate_schema(&pool).await?;

    Ok(pool)
}
