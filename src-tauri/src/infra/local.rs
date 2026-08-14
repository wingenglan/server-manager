use crate::domain::metrics::SystemOverview;
use crate::domain::shortcuts::{
    default_shortcuts, SaveShortcutInput, ShortcutRecord, ShortcutScope,
};
use crate::errors::{AppError, AppResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

/// Stores local-only shortcut, metric history, and task metadata without remote secrets or output.
#[derive(Clone)]
pub struct LocalRepository {
    pool: SqlitePool,
}

impl LocalRepository {
    /// Creates a local repository backed by the application's already-migrated SQLite pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Seeds the built-in shortcuts exactly once and marks unfinished tasks from a previous run interrupted.
    pub async fn initialize_workspace(&self) -> AppResult<()> {
        self.seed_default_shortcuts().await?;
        let now = Utc::now();
        sqlx::query("UPDATE task_records SET status='interrupted', finished_at=?, updated_at=? WHERE status IN ('queued','running')")
            .bind(now)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(AppError::database)?;
        Ok(())
    }

    /// Lists global shortcuts and server-specific overrides, hiding a global item when the server has the same name.
    pub async fn list_shortcuts(&self, server_id: Option<&str>) -> AppResult<Vec<ShortcutRecord>> {
        let rows = if let Some(server_id) = server_id {
            sqlx::query_as::<_, ShortcutRow>("SELECT id,scope,server_id,name,group_name,command_template,description,tags_json,enabled,builtin,usage_count,created_at,updated_at FROM command_shortcuts WHERE scope='global' OR server_id=? ORDER BY CASE WHEN scope='server' THEN 0 ELSE 1 END, group_name COLLATE NOCASE, usage_count DESC, name COLLATE NOCASE")
                .bind(server_id)
                .fetch_all(&self.pool)
                .await
                .map_err(AppError::database)?
        } else {
            sqlx::query_as::<_, ShortcutRow>("SELECT id,scope,server_id,name,group_name,command_template,description,tags_json,enabled,builtin,usage_count,created_at,updated_at FROM command_shortcuts WHERE scope='global' ORDER BY group_name COLLATE NOCASE, usage_count DESC, name COLLATE NOCASE")
                .fetch_all(&self.pool)
                .await
                .map_err(AppError::database)?
        };
        let mut names = std::collections::HashSet::new();
        let mut shortcuts = Vec::with_capacity(rows.len());
        for row in rows {
            let is_server = row.scope == "server";
            if !is_server && server_id.is_some() && names.contains(&row.name.to_lowercase()) {
                continue;
            }
            if is_server {
                names.insert(row.name.to_lowercase());
            }
            shortcuts.push(row.into_record()?);
        }
        Ok(shortcuts)
    }

    /// Creates or updates a shortcut after validating its scope and template fields.
    pub async fn save_shortcut(&self, input: SaveShortcutInput) -> AppResult<ShortcutRecord> {
        input.validate()?;
        let id = input
            .id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let now = Utc::now();
        let tags = normalize_tags(input.tags);
        let tags_json = serde_json::to_string(&tags).map_err(AppError::database)?;
        let group_name = normalize_group_name(&input.group_name);
        sqlx::query("INSERT INTO command_shortcuts (id,scope,server_id,name,group_name,command_template,description,tags_json,enabled,builtin,usage_count,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET scope=excluded.scope,server_id=excluded.server_id,name=excluded.name,group_name=excluded.group_name,command_template=excluded.command_template,description=excluded.description,tags_json=excluded.tags_json,enabled=excluded.enabled,updated_at=excluded.updated_at")
            .bind(&id)
            .bind(input.scope.as_str())
            .bind(input.server_id.as_deref())
            .bind(input.name.trim())
            .bind(group_name)
            .bind(input.command_template.trim())
            .bind(input.description.trim())
            .bind(tags_json)
            .bind(input.enabled)
            .bind(false)
            .bind(0_i64)
            .bind(now)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(|error| {
                if error.to_string().contains("UNIQUE") {
                    AppError::new("SHORTCUT_DUPLICATE", "shortcut", "同一范围内已存在同名快捷指令")
                } else {
                    AppError::database(error)
                }
            })?;
        self.shortcut_by_id(&id).await
    }

    /// Deletes a shortcut by id; built-in records may be restored later through the defaults action.
    pub async fn delete_shortcut(&self, id: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM command_shortcuts WHERE id=?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(AppError::database)?;
        Ok(())
    }

    /// Restores missing built-in shortcuts without overwriting user edits or disabled choices.
    pub async fn restore_default_shortcuts(&self) -> AppResult<()> {
        self.seed_default_shortcuts().await
    }

    /// Increments local usage metadata after a shortcut is inserted into a terminal.
    pub async fn use_shortcut(&self, id: &str) -> AppResult<()> {
        sqlx::query(
            "UPDATE command_shortcuts SET usage_count=usage_count+1, updated_at=? WHERE id=?",
        )
        .bind(Utc::now())
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(AppError::database)?;
        Ok(())
    }

    /// Appends one Overview sample and prunes data older than 24 hours or beyond 20,000 rows per server.
    pub async fn record_metric(&self, server_id: &str, overview: &SystemOverview) -> AppResult<()> {
        let memory_used = overview
            .memory_total_bytes
            .saturating_sub(overview.memory_available_bytes);
        sqlx::query("INSERT INTO metric_samples (server_id,sampled_at,cpu_usage_percent,memory_used_bytes,memory_total_bytes,load_one,network_rx_bytes_per_second,network_tx_bytes_per_second,disk_usage_percent) VALUES (?,?,?,?,?,?,?,?,?)")
            .bind(server_id)
            .bind(&overview.sampled_at)
            .bind(overview.cpu_usage_percent)
            .bind(i64::try_from(memory_used).unwrap_or(i64::MAX))
            .bind(i64::try_from(overview.memory_total_bytes).unwrap_or(i64::MAX))
            .bind(overview.load[0])
            .bind(i64::try_from(overview.network_rx_bytes_per_second).unwrap_or(i64::MAX))
            .bind(i64::try_from(overview.network_tx_bytes_per_second).unwrap_or(i64::MAX))
            .bind(overview.disks.first().map(|disk| disk.usage_percent))
            .execute(&self.pool)
            .await
            .map_err(AppError::database)?;
        let cutoff = Utc::now() - chrono::Duration::hours(24);
        sqlx::query("DELETE FROM metric_samples WHERE server_id=? AND sampled_at < ?")
            .bind(server_id)
            .bind(cutoff)
            .execute(&self.pool)
            .await
            .map_err(AppError::database)?;
        sqlx::query("DELETE FROM metric_samples WHERE server_id=? AND id NOT IN (SELECT id FROM metric_samples WHERE server_id=? ORDER BY sampled_at DESC LIMIT 20000)")
            .bind(server_id)
            .bind(server_id)
            .execute(&self.pool)
            .await
            .map_err(AppError::database)?;
        Ok(())
    }

    /// Reads bounded metric history after the supplied UTC timestamp for chart rendering.
    pub async fn metric_history(
        &self,
        server_id: &str,
        since: DateTime<Utc>,
        limit: u32,
    ) -> AppResult<Vec<MetricSample>> {
        sqlx::query_as::<_, MetricSample>("SELECT sampled_at,cpu_usage_percent,memory_used_bytes,memory_total_bytes,load_one,network_rx_bytes_per_second,network_tx_bytes_per_second,disk_usage_percent FROM metric_samples WHERE server_id=? AND sampled_at >= ? ORDER BY sampled_at ASC LIMIT ?")
            .bind(server_id)
            .bind(since)
            .bind(i64::from(limit.clamp(1, 500)))
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::database)
    }

    /// Inserts or updates a task metadata record while excluding command output and secrets.
    pub async fn save_task(&self, input: SaveTaskInput) -> AppResult<TaskRecord> {
        let now = Utc::now();
        sqlx::query("INSERT INTO task_records (id,type,server_id,title,status,progress,bytes_transferred,total_bytes,started_at,finished_at,error_code,error_message,cancel_supported,retry_payload_json,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET type=excluded.type,server_id=excluded.server_id,title=excluded.title,status=excluded.status,progress=excluded.progress,bytes_transferred=excluded.bytes_transferred,total_bytes=excluded.total_bytes,started_at=excluded.started_at,finished_at=excluded.finished_at,error_code=excluded.error_code,error_message=excluded.error_message,cancel_supported=excluded.cancel_supported,retry_payload_json=excluded.retry_payload_json,updated_at=excluded.updated_at")
            .bind(&input.id).bind(&input.task_type).bind(input.server_id.as_deref()).bind(&input.title).bind(&input.status).bind(input.progress)
            .bind(i64::try_from(input.bytes_transferred).unwrap_or(i64::MAX)).bind(input.total_bytes.map(|value| i64::try_from(value).unwrap_or(i64::MAX)))
            .bind(input.started_at).bind(input.finished_at).bind(input.error_code.as_deref()).bind(input.error_message.as_deref()).bind(input.cancel_supported).bind(input.retry_payload_json.as_deref()).bind(now).bind(now)
            .execute(&self.pool).await.map_err(AppError::database)?;
        self.task_by_id(&input.id).await
    }

    /// Lists the newest task metadata records for the global task center.
    pub async fn list_tasks(&self, limit: u32) -> AppResult<Vec<TaskRecord>> {
        sqlx::query_as::<_, TaskRecord>("SELECT id,type,server_id,title,status,progress,bytes_transferred,total_bytes,started_at,finished_at,error_code,error_message,cancel_supported,retry_payload_json,created_at,updated_at FROM task_records ORDER BY updated_at DESC LIMIT ?")
            .bind(i64::from(limit.clamp(1, 500))).fetch_all(&self.pool).await.map_err(AppError::database)
    }

    /// Deletes terminal task metadata while keeping active and interrupted records for recovery visibility.
    pub async fn clear_finished_tasks(&self) -> AppResult<()> {
        sqlx::query("DELETE FROM task_records WHERE status IN ('success','failed','cancelled','interrupted')")
            .execute(&self.pool).await.map_err(AppError::database)?;
        Ok(())
    }

    /// Fetches one shortcut record and converts its stored JSON fields into typed data.
    async fn shortcut_by_id(&self, id: &str) -> AppResult<ShortcutRecord> {
        let row = sqlx::query_as::<_, ShortcutRow>("SELECT id,scope,server_id,name,group_name,command_template,description,tags_json,enabled,builtin,usage_count,created_at,updated_at FROM command_shortcuts WHERE id=?")
            .bind(id).fetch_optional(&self.pool).await.map_err(AppError::database)?.ok_or_else(|| AppError::new("SHORTCUT_NOT_FOUND", "shortcut", "快捷指令不存在"))?;
        row.into_record()
    }

    /// Inserts missing built-in shortcuts without modifying existing user choices.
    async fn seed_default_shortcuts(&self) -> AppResult<()> {
        let now = Utc::now();
        for shortcut in default_shortcuts() {
            sqlx::query(
                "INSERT OR IGNORE INTO command_shortcuts (id,scope,server_id,name,group_name,command_template,description,tags_json,enabled,builtin,usage_count,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)",
            )
            .bind(shortcut.id)
            .bind("global")
            .bind(Option::<String>::None)
            .bind(shortcut.name)
            .bind(shortcut.group_name)
            .bind(shortcut.command_template)
            .bind(shortcut.description)
            .bind(serde_json::to_string(shortcut.tags).map_err(AppError::database)?)
            .bind(true)
            .bind(true)
            .bind(0_i64)
            .bind(now)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(AppError::database)?;
            sqlx::query("UPDATE command_shortcuts SET group_name=?, updated_at=? WHERE id=? AND builtin=1 AND group_name=''")
                .bind(shortcut.group_name)
                .bind(now)
                .bind(shortcut.id)
                .execute(&self.pool)
                .await
                .map_err(AppError::database)?;
        }
        Ok(())
    }

    /// Fetches one task record after an upsert for a stable command response.
    async fn task_by_id(&self, id: &str) -> AppResult<TaskRecord> {
        sqlx::query_as::<_, TaskRecord>("SELECT id,type,server_id,title,status,progress,bytes_transferred,total_bytes,started_at,finished_at,error_code,error_message,cancel_supported,retry_payload_json,created_at,updated_at FROM task_records WHERE id=?")
            .bind(id).fetch_optional(&self.pool).await.map_err(AppError::database)?.ok_or_else(|| AppError::new("TASK_NOT_FOUND", "task", "任务记录不存在"))
    }
}

/// Normalizes user-entered shortcut tags before they are serialized into SQLite.
fn normalize_tags(values: Vec<String>) -> Vec<String> {
    let mut tags = Vec::new();
    for value in values {
        let value = value.trim();
        if value.is_empty() || value.chars().count() > 32 {
            continue;
        }
        if !tags
            .iter()
            .any(|current: &String| current.eq_ignore_ascii_case(value))
        {
            tags.push(value.to_string());
        }
        if tags.len() >= 12 {
            break;
        }
    }
    tags
}

/// Trims a shortcut group label while keeping empty labels available for ungrouped commands.
fn normalize_group_name(value: &str) -> String {
    value.trim().to_string()
}

/// Stores the typed SQLite representation of a shortcut before decoding tags and scope.
#[derive(Debug, FromRow)]
struct ShortcutRow {
    id: String,
    scope: String,
    server_id: Option<String>,
    name: String,
    group_name: String,
    command_template: String,
    description: String,
    tags_json: String,
    enabled: bool,
    builtin: bool,
    usage_count: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ShortcutRow {
    /// Converts the database row into the camelCase typed IPC record.
    fn into_record(self) -> AppResult<ShortcutRecord> {
        let scope = match self.scope.as_str() {
            "global" => ShortcutScope::Global,
            "server" => ShortcutScope::Server,
            _ => {
                return Err(AppError::new(
                    "DATABASE_INVALID",
                    "local_storage",
                    "快捷指令范围无效",
                ))
            }
        };
        Ok(ShortcutRecord {
            id: self.id,
            scope,
            server_id: self.server_id,
            name: self.name,
            group_name: self.group_name,
            command_template: self.command_template,
            description: self.description,
            tags: serde_json::from_str(&self.tags_json).unwrap_or_default(),
            enabled: self.enabled,
            builtin: self.builtin,
            usage_count: u64::try_from(self.usage_count).unwrap_or(0),
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

/// Contains one local metric point used by the Overview charts.
#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct MetricSample {
    pub sampled_at: DateTime<Utc>,
    pub cpu_usage_percent: Option<f64>,
    pub memory_used_bytes: i64,
    pub memory_total_bytes: i64,
    pub load_one: f64,
    pub network_rx_bytes_per_second: i64,
    pub network_tx_bytes_per_second: i64,
    pub disk_usage_percent: Option<f64>,
}

/// Carries non-sensitive task metadata from the frontend task store to SQLite.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveTaskInput {
    pub id: String,
    pub task_type: String,
    pub server_id: Option<String>,
    pub title: String,
    pub status: String,
    pub progress: Option<f64>,
    pub bytes_transferred: u64,
    pub total_bytes: Option<u64>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub cancel_supported: bool,
    pub retry_payload_json: Option<String>,
}

/// Represents one persisted task record returned to the task center.
#[derive(Debug, Clone, Serialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct TaskRecord {
    pub id: String,
    #[sqlx(rename = "type")]
    pub task_type: String,
    pub server_id: Option<String>,
    pub title: String,
    pub status: String,
    pub progress: Option<f64>,
    pub bytes_transferred: i64,
    pub total_bytes: Option<i64>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub cancel_supported: bool,
    pub retry_payload_json: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
