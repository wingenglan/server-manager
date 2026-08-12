use crate::domain::server::{SaveServerInput, ServerProfile, ServerRecord};
use crate::errors::{AppError, AppResult};
use chrono::Utc;
use secrecy::{ExposeSecret, SecretString};
use sqlx::{SqlitePool, Transaction};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct ServerRepository {
    pool: SqlitePool,
    credentials: Arc<dyn crate::security::CredentialStore>,
}

impl ServerRepository {
    pub fn new(pool: SqlitePool, credentials: Arc<dyn crate::security::CredentialStore>) -> Self {
        Self { pool, credentials }
    }

    pub async fn list(&self) -> AppResult<Vec<ServerProfile>> {
        let records = sqlx::query_as::<_, ServerProfile>(
            r#"SELECT id, name, description, host, port, username, auth_type, private_key_path,
               sudo_mode, group_id, favorite,
               COALESCE(json_extract(settings_json, '$.connectTimeout'), 10) AS connect_timeout,
               COALESCE(json_extract(settings_json, '$.keepalive'), 30) AS keepalive,
               COALESCE(json_extract(settings_json, '$.encoding'), 'UTF-8') AS encoding,
               last_connected_at, created_at, updated_at
               FROM servers ORDER BY favorite DESC, COALESCE(last_connected_at, created_at) DESC"#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::database)?;
        self.attach_tags(records).await
    }

    pub async fn get(&self, id: &str) -> AppResult<ServerProfile> {
        let profile = sqlx::query_as::<_, ServerProfile>(
            r#"SELECT id, name, description, host, port, username, auth_type, private_key_path,
               sudo_mode, group_id, favorite,
               COALESCE(json_extract(settings_json, '$.connectTimeout'), 10) AS connect_timeout,
               COALESCE(json_extract(settings_json, '$.keepalive'), 30) AS keepalive,
               COALESCE(json_extract(settings_json, '$.encoding'), 'UTF-8') AS encoding,
               last_connected_at, created_at, updated_at FROM servers WHERE id = ?"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::database)?
        .ok_or_else(|| AppError::new("SERVER_NOT_FOUND", "server", "服务器配置不存在"))?;
        self.attach_tags(vec![profile])
            .await
            .map(|mut values| values.remove(0))
    }

    pub async fn record(&self, id: &str) -> AppResult<ServerRecord> {
        sqlx::query_as::<_, ServerRecord>("SELECT * FROM servers WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::new("SERVER_NOT_FOUND", "server", "服务器配置不存在"))
    }

    async fn attach_tags(&self, mut profiles: Vec<ServerProfile>) -> AppResult<Vec<ServerProfile>> {
        for profile in &mut profiles {
            profile.tags = sqlx::query_scalar::<_, String>(
                "SELECT t.name FROM tags t JOIN server_tags st ON st.tag_id=t.id WHERE st.server_id=? ORDER BY t.name",
            )
            .bind(&profile.id)
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::database)?;
        }
        Ok(profiles)
    }

    pub async fn save(&self, input: SaveServerInput) -> AppResult<ServerProfile> {
        input.validate()?;
        let id = input
            .id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let existing = sqlx::query_as::<_, ServerRecord>("SELECT * FROM servers WHERE id=?")
            .bind(&id)
            .fetch_optional(&self.pool)
            .await
            .map_err(AppError::database)?;
        let previous_password_ref = existing
            .as_ref()
            .and_then(|value| value.password_secret_ref.clone());
        let previous_passphrase_ref = existing
            .as_ref()
            .and_then(|value| value.key_passphrase_secret_ref.clone());
        let previous_sudo_ref = existing
            .as_ref()
            .and_then(|value| value.sudo_secret_ref.clone());
        let password_ref = if input.auth_type == "password" {
            self.persist_optional_secret(
                previous_password_ref.clone(),
                input.password.as_ref().map(|value| value.expose_secret()),
                "ssh",
            )?
        } else {
            None
        };
        let passphrase_ref = if input.auth_type == "private_key" {
            self.persist_optional_secret(
                previous_passphrase_ref.clone(),
                input
                    .private_key_passphrase
                    .as_ref()
                    .map(|value| value.expose_secret()),
                "key-passphrase",
            )?
        } else {
            None
        };
        let sudo_ref = if input.sudo_mode == "password" {
            self.persist_optional_secret(
                previous_sudo_ref.clone(),
                input
                    .sudo_password
                    .as_ref()
                    .map(|value| value.expose_secret()),
                "sudo",
            )?
        } else {
            None
        };
        let now = Utc::now();
        let created_at = existing
            .as_ref()
            .map(|value| value.created_at)
            .unwrap_or(now);
        let settings =
            serde_json::json!({"connectTimeout": 10, "keepalive": 30, "encoding": "UTF-8"})
                .to_string();
        let mut transaction = self.pool.begin().await.map_err(AppError::database)?;
        sqlx::query(
            r#"INSERT INTO servers (id,name,description,host,port,username,auth_type,password_secret_ref,
               private_key_path,key_passphrase_secret_ref,sudo_mode,sudo_secret_ref,group_id,favorite,settings_json,created_at,updated_at)
               VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
               ON CONFLICT(id) DO UPDATE SET name=excluded.name,description=excluded.description,host=excluded.host,
               port=excluded.port,username=excluded.username,auth_type=excluded.auth_type,password_secret_ref=excluded.password_secret_ref,
               private_key_path=excluded.private_key_path,key_passphrase_secret_ref=excluded.key_passphrase_secret_ref,
               sudo_mode=excluded.sudo_mode,sudo_secret_ref=excluded.sudo_secret_ref,group_id=excluded.group_id,
               favorite=excluded.favorite,settings_json=excluded.settings_json,updated_at=excluded.updated_at"#,
        )
        .bind(&id).bind(input.name.trim()).bind(input.description.trim()).bind(input.host.trim())
        .bind(i64::from(input.port)).bind(input.username.trim()).bind(&input.auth_type).bind(password_ref.as_deref())
        .bind(input.private_key_path.as_deref().map(str::trim)).bind(passphrase_ref.as_deref()).bind(&input.sudo_mode)
        .bind(sudo_ref.as_deref()).bind(Option::<String>::None).bind(input.favorite).bind(settings)
        .bind(created_at).bind(now)
        .execute(&mut *transaction).await.map_err(AppError::database)?;
        self.replace_tags(&mut transaction, &id, input.tags).await?;
        transaction.commit().await.map_err(AppError::database)?;
        for (previous, current) in [
            (previous_password_ref, password_ref),
            (previous_passphrase_ref, passphrase_ref),
            (previous_sudo_ref, sudo_ref),
        ] {
            if previous.is_some() && previous != current {
                if let Some(reference) = previous {
                    self.credentials.delete(&reference)?;
                }
            }
        }
        self.get(&id).await
    }

    fn persist_optional_secret(
        &self,
        existing: Option<String>,
        value: Option<&str>,
        kind: &str,
    ) -> AppResult<Option<String>> {
        let Some(value) = value.filter(|value| !value.is_empty()) else {
            return Ok(existing);
        };
        let reference = existing.unwrap_or_else(|| format!("{}-{}", kind, Uuid::new_v4()));
        self.credentials
            .put(&reference, SecretString::from(value.to_owned()))?;
        Ok(Some(reference))
    }

    async fn replace_tags(
        &self,
        transaction: &mut Transaction<'_, sqlx::Sqlite>,
        server_id: &str,
        tags: Vec<String>,
    ) -> AppResult<()> {
        sqlx::query("DELETE FROM server_tags WHERE server_id=?")
            .bind(server_id)
            .execute(&mut **transaction)
            .await
            .map_err(AppError::database)?;
        for name in tags
            .into_iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        {
            let tag_id = Uuid::new_v4().to_string();
            sqlx::query(
                "INSERT INTO tags (id,name,created_at) VALUES (?,?,?) ON CONFLICT(name) DO NOTHING",
            )
            .bind(&tag_id)
            .bind(&name)
            .bind(Utc::now())
            .execute(&mut **transaction)
            .await
            .map_err(AppError::database)?;
            sqlx::query(
                "INSERT INTO server_tags (server_id,tag_id) SELECT ?,id FROM tags WHERE name=?",
            )
            .bind(server_id)
            .bind(&name)
            .execute(&mut **transaction)
            .await
            .map_err(AppError::database)?;
        }
        Ok(())
    }

    pub async fn delete(&self, id: &str) -> AppResult<()> {
        let record = self.record(id).await?;
        sqlx::query("DELETE FROM servers WHERE id=?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(AppError::database)?;
        for reference in [
            record.password_secret_ref,
            record.key_passphrase_secret_ref,
            record.sudo_secret_ref,
        ]
        .into_iter()
        .flatten()
        {
            self.credentials.delete(&reference)?;
        }
        Ok(())
    }

    pub async fn known_host(&self, identity: &str) -> AppResult<Option<KnownHost>> {
        sqlx::query_as::<_, KnownHost>("SELECT server_identity,key_type,fingerprint,public_key FROM known_hosts WHERE server_identity=?")
            .bind(identity).fetch_optional(&self.pool).await.map_err(AppError::database)
    }

    pub async fn trust_host(&self, value: &KnownHost) -> AppResult<()> {
        let now = Utc::now();
        sqlx::query("INSERT INTO known_hosts (server_identity,key_type,fingerprint,public_key,first_seen_at,last_seen_at) VALUES (?,?,?,?,?,?)")
            .bind(&value.server_identity).bind(&value.key_type).bind(&value.fingerprint).bind(&value.public_key)
            .bind(now).bind(now).execute(&self.pool).await.map_err(AppError::database)?;
        Ok(())
    }

    pub async fn mark_connected(&self, id: &str) -> AppResult<()> {
        sqlx::query("UPDATE servers SET last_connected_at=?,updated_at=? WHERE id=?")
            .bind(Utc::now())
            .bind(Utc::now())
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(AppError::database)?;
        Ok(())
    }

    pub fn credential(&self, reference: &str) -> AppResult<SecretString> {
        self.credentials.get(reference)
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct KnownHost {
    pub server_identity: String,
    pub key_type: String,
    pub fingerprint: String,
    pub public_key: String,
}
