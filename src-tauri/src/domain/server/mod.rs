use chrono::{DateTime, Utc};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ServerProfile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_type: String,
    pub private_key_path: Option<String>,
    pub sudo_mode: String,
    pub group_id: Option<String>,
    #[sqlx(skip)]
    pub tags: Vec<String>,
    pub favorite: bool,
    pub connect_timeout: i64,
    pub keepalive: i64,
    pub encoding: String,
    pub last_connected_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct ServerGroup {
    pub id: String,
    pub name: String,
    pub sort_order: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicServerProfile {
    pub name: String,
    pub description: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_type: String,
    pub private_key_path: Option<String>,
    pub sudo_mode: String,
    pub group_id: Option<String>,
    pub tags: Vec<String>,
    pub favorite: bool,
    pub connect_timeout: i64,
    pub keepalive: i64,
    pub encoding: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicServerImport {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_type: String,
    pub private_key_path: Option<String>,
    #[serde(default = "default_sudo_mode")]
    pub sudo_mode: String,
    pub group_id: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout: i64,
    #[serde(default = "default_keepalive")]
    pub keepalive: i64,
    #[serde(default = "default_encoding")]
    pub encoding: String,
}

/// 为旧版公共配置导入提供稳定的连接超时默认值。
fn default_connect_timeout() -> i64 {
    10
}

/// 为旧版公共配置导入提供稳定的 keepalive 默认值。
fn default_keepalive() -> i64 {
    30
}

/// 为旧版公共配置导入提供 UTF-8 默认编码。
fn default_encoding() -> String {
    "UTF-8".into()
}

/// 为不含 sudo 配置的旧版公共导入提供安全默认值。
fn default_sudo_mode() -> String {
    "none".into()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicServerExport {
    pub format: String,
    pub version: u32,
    pub encrypted: bool,
    pub servers: Vec<PublicServerProfile>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveServerInput {
    pub id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_type: String,
    pub password: Option<SecretString>,
    pub private_key_path: Option<String>,
    pub private_key_passphrase: Option<SecretString>,
    pub sudo_mode: String,
    pub sudo_password: Option<SecretString>,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub connect_timeout: Option<u64>,
    #[serde(default)]
    pub keepalive: Option<u64>,
    #[serde(default)]
    pub encoding: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub favorite: bool,
}

impl SaveServerInput {
    pub fn validate(&self) -> crate::errors::AppResult<()> {
        if self.name.trim().is_empty()
            || self.host.trim().is_empty()
            || self.username.trim().is_empty()
        {
            return Err(crate::errors::AppError::new(
                "VALIDATION_FAILED",
                "validation",
                "名称、主机和用户名不能为空",
            ));
        }
        if !matches!(
            self.auth_type.as_str(),
            "password" | "private_key" | "ssh_agent"
        ) {
            return Err(crate::errors::AppError::new(
                "VALIDATION_FAILED",
                "validation",
                "不支持的 SSH 认证方式",
            ));
        }
        if !matches!(
            self.sudo_mode.as_str(),
            "none" | "passwordless" | "password"
        ) {
            return Err(crate::errors::AppError::new(
                "VALIDATION_FAILED",
                "validation",
                "不支持的 sudo 模式",
            ));
        }
        if self.auth_type == "password"
            && self.id.is_none()
            && self
                .password
                .as_ref()
                .map(|value| value.expose_secret().is_empty())
                .unwrap_or(true)
        {
            return Err(crate::errors::AppError::new(
                "VALIDATION_FAILED",
                "validation",
                "密码认证需要 SSH 密码",
            ));
        }
        if self.auth_type == "private_key"
            && self
                .private_key_path
                .as_deref()
                .unwrap_or_default()
                .is_empty()
        {
            return Err(crate::errors::AppError::new(
                "VALIDATION_FAILED",
                "validation",
                "私钥认证需要私钥路径",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
#[allow(dead_code)]
pub struct ServerRecord {
    pub id: String,
    pub name: String,
    pub description: String,
    pub host: String,
    pub port: i64,
    pub username: String,
    pub auth_type: String,
    pub password_secret_ref: Option<String>,
    pub private_key_path: Option<String>,
    pub key_passphrase_secret_ref: Option<String>,
    pub sudo_mode: String,
    pub sudo_secret_ref: Option<String>,
    pub group_id: Option<String>,
    pub favorite: bool,
    pub settings_json: String,
    pub last_connected_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ServerRecord {
    pub fn connect_timeout(&self) -> u64 {
        serde_json::from_str::<serde_json::Value>(&self.settings_json)
            .ok()
            .and_then(|value| value.get("connectTimeout").and_then(|value| value.as_u64()))
            .unwrap_or(10)
    }

    pub fn keepalive(&self) -> u64 {
        serde_json::from_str::<serde_json::Value>(&self.settings_json)
            .ok()
            .and_then(|value| value.get("keepalive").and_then(|value| value.as_u64()))
            .unwrap_or(30)
    }
}
