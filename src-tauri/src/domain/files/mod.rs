use crate::domain::ssh::SshConnectionManager;
use crate::errors::{AppError, AppResult};
use crate::security::shell_escape;
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::FileAttributes;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const EDITOR_LIMIT: u64 = 10 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteFileEntry {
    pub name: String,
    pub path: String,
    pub kind: FileKind,
    pub size: u64,
    pub permissions: String,
    pub owner: String,
    pub group: String,
    pub modified_at: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FileKind {
    Directory,
    File,
    Symlink,
    Other,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryListing {
    pub path: String,
    pub entries: Vec<RemoteFileEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteTextFile {
    pub path: String,
    pub content: String,
    pub size: u64,
    pub modified_at: Option<u32>,
    pub permissions: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveTextInput {
    pub server_id: String,
    pub path: String,
    pub content: String,
    pub expected_size: u64,
    pub expected_modified_at: Option<u32>,
    #[serde(default)]
    pub force: bool,
}

pub async fn list(
    ssh: &SshConnectionManager,
    server_id: &str,
    path: &str,
) -> AppResult<DirectoryListing> {
    validate_path(path)?;
    let sftp = ssh.open_sftp(server_id).await?;
    let canonical = sftp
        .canonicalize(path)
        .await
        .map_err(|error| map_sftp(error, server_id, "无法解析远程路径"))?;
    let entries = sftp
        .read_dir(canonical.clone())
        .await
        .map_err(|error| map_sftp(error, server_id, "无法读取远程目录"))?
        .map(|entry| {
            let metadata = entry.metadata();
            RemoteFileEntry {
                name: entry.file_name(),
                path: entry.path(),
                kind: kind(&metadata),
                size: metadata.len(),
                permissions: format!("{:04o}", metadata.permissions.unwrap_or(0) & 0o7777),
                owner: metadata
                    .user
                    .clone()
                    .or_else(|| metadata.uid.map(|value| value.to_string()))
                    .unwrap_or_else(|| "—".into()),
                group: metadata
                    .group
                    .clone()
                    .or_else(|| metadata.gid.map(|value| value.to_string()))
                    .unwrap_or_else(|| "—".into()),
                modified_at: metadata.mtime,
            }
        })
        .collect();
    let _ = sftp.close().await;
    Ok(DirectoryListing {
        path: canonical,
        entries,
    })
}

pub async fn read_text(
    ssh: &SshConnectionManager,
    server_id: &str,
    path: &str,
) -> AppResult<RemoteTextFile> {
    validate_path(path)?;
    let sftp = ssh.open_sftp(server_id).await?;
    let metadata = sftp
        .metadata(path)
        .await
        .map_err(|error| map_sftp(error, server_id, "无法读取文件信息"))?;
    if !metadata.is_regular() {
        return Err(
            AppError::new("SFTP_FAILED", "sftp", "所选对象不是普通文件").for_server(server_id)
        );
    }
    if metadata.len() > EDITOR_LIMIT {
        return Err(AppError::new(
            "FILE_TOO_LARGE",
            "file",
            "文件超过 10 MB，请使用下载或大文件查看器",
        )
        .for_server(server_id));
    }
    let mut file = sftp
        .open(path)
        .await
        .map_err(|error| map_sftp(error, server_id, "无法打开远程文件"))?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.read_to_end(&mut bytes)
        .await
        .map_err(|error| map_sftp(error, server_id, "远程文件读取失败"))?;
    let content = String::from_utf8(bytes).map_err(|_| {
        AppError::new("FILE_NOT_TEXT", "file", "文件不是有效 UTF-8 文本").for_server(server_id)
    })?;
    let _ = sftp.close().await;
    Ok(RemoteTextFile {
        path: path.to_string(),
        content,
        size: metadata.len(),
        modified_at: metadata.mtime,
        permissions: metadata.permissions,
    })
}

pub async fn save_text(
    ssh: &SshConnectionManager,
    input: SaveTextInput,
) -> AppResult<RemoteTextFile> {
    validate_path(&input.path)?;
    if input.content.len() as u64 > EDITOR_LIMIT {
        return Err(
            AppError::new("FILE_TOO_LARGE", "file", "编辑内容超过 10 MB")
                .for_server(&input.server_id),
        );
    }
    let sftp = ssh.open_sftp(&input.server_id).await?;
    let current = sftp
        .metadata(&input.path)
        .await
        .map_err(|error| map_sftp(error, &input.server_id, "无法检查远程文件状态"))?;
    if !input.force
        && (current.len() != input.expected_size || current.mtime != input.expected_modified_at)
    {
        return Err(AppError::new(
            "FILE_CONFLICT",
            "file",
            "远程文件已被其他程序修改，已阻止覆盖",
        )
        .for_server(&input.server_id)
        .suggestion("重新载入文件、比较差异，或明确选择强制覆盖"));
    }
    let temporary = temporary_sibling(&input.path);
    let mut file = sftp
        .create(&temporary)
        .await
        .map_err(|error| map_sftp(error, &input.server_id, "无法创建同目录临时文件"))?;
    file.write_all(input.content.as_bytes())
        .await
        .map_err(|error| map_sftp(error, &input.server_id, "远程文件写入失败"))?;
    file.flush()
        .await
        .map_err(|error| map_sftp(error, &input.server_id, "远程文件刷新失败"))?;
    file.sync_all()
        .await
        .map_err(|error| map_sftp(error, &input.server_id, "远程文件同步失败"))?;
    let mut attributes = FileAttributes::empty();
    attributes.permissions = current.permissions;
    sftp.set_metadata(&temporary, attributes)
        .await
        .map_err(|error| map_sftp(error, &input.server_id, "无法保留远程文件权限"))?;
    drop(file);
    let command = format!(
        "mv -f -- {} {}",
        shell_escape(&temporary),
        shell_escape(&input.path)
    );
    let result = ssh
        .execute(&input.server_id, &command, Duration::from_secs(30))
        .await?;
    if result.exit_code != 0 {
        let _ = sftp.remove_file(&temporary).await;
        return Err(
            AppError::new("REMOTE_COMMAND_FAILED", "file", "原子替换远程文件失败")
                .details(result.stderr)
                .for_server(&input.server_id),
        );
    }
    let _ = sftp.close().await;
    read_text(ssh, &input.server_id, &input.path).await
}

pub async fn save_text_privileged(
    ssh: &SshConnectionManager,
    input: SaveTextInput,
) -> AppResult<RemoteTextFile> {
    validate_path(&input.path)?;
    if input.content.len() as u64 > EDITOR_LIMIT {
        return Err(
            AppError::new("FILE_TOO_LARGE", "file", "编辑内容超过 10 MB")
                .for_server(&input.server_id),
        );
    }
    let sftp = ssh.open_sftp(&input.server_id).await?;
    let current = sftp
        .metadata(&input.path)
        .await
        .map_err(|error| map_sftp(error, &input.server_id, "无法检查远程文件状态"))?;
    if !input.force
        && (current.len() != input.expected_size || current.mtime != input.expected_modified_at)
    {
        return Err(AppError::new(
            "FILE_CONFLICT",
            "file",
            "远程文件已被其他程序修改，已阻止覆盖",
        )
        .for_server(&input.server_id)
        .suggestion("重新载入文件、比较差异，或明确选择强制覆盖"));
    }
    let temporary = format!("/tmp/.relay-{}.tmp", uuid::Uuid::new_v4());
    let mut file = sftp
        .create(&temporary)
        .await
        .map_err(|error| map_sftp(error, &input.server_id, "无法创建 sudo 保存临时文件"))?;
    file.write_all(input.content.as_bytes())
        .await
        .map_err(|error| map_sftp(error, &input.server_id, "sudo 临时文件写入失败"))?;
    file.flush()
        .await
        .map_err(|error| map_sftp(error, &input.server_id, "sudo 临时文件刷新失败"))?;
    file.sync_all()
        .await
        .map_err(|error| map_sftp(error, &input.server_id, "sudo 临时文件同步失败"))?;
    drop(file);
    let backup = format!(
        "{}.relay-backup-{}",
        input.path,
        chrono::Utc::now().format("%Y%m%d%H%M%S")
    );
    let mode = current.permissions.unwrap_or(0o644) & 0o7777;
    let command = format!(
        "set -e; cp -a -- {target} {backup}; install -m {mode:o} -- {temporary} {target}; rm -f -- {temporary}",
        target = shell_escape(&input.path),
        backup = shell_escape(&backup),
        temporary = shell_escape(&temporary),
    );
    let result = ssh
        .execute_privileged(&input.server_id, &command, Duration::from_secs(30))
        .await?;
    if result.exit_code != 0 {
        let _ = sftp.remove_file(&temporary).await;
        return Err(AppError::new(
            "PERMISSION_DENIED",
            "privilege",
            "使用 sudo 保存远程文件失败",
        )
        .details(result.stderr)
        .for_server(&input.server_id));
    }
    let _ = sftp.close().await;
    let saved = read_text(ssh, &input.server_id, &input.path).await?;
    if saved.content != input.content {
        return Err(
            AppError::new("FILE_VERIFY_FAILED", "file", "sudo 保存后内容验证失败")
                .for_server(&input.server_id),
        );
    }
    Ok(saved)
}

pub async fn create(
    ssh: &SshConnectionManager,
    server_id: &str,
    path: &str,
    directory: bool,
) -> AppResult<()> {
    validate_path(path)?;
    let sftp = ssh.open_sftp(server_id).await?;
    if directory {
        sftp.create_dir(path)
            .await
            .map_err(|error| map_sftp(error, server_id, "新建文件夹失败"))?;
    } else {
        let mut file = sftp
            .create(path)
            .await
            .map_err(|error| map_sftp(error, server_id, "新建文件失败"))?;
        file.flush()
            .await
            .map_err(|error| map_sftp(error, server_id, "新建文件刷新失败"))?;
    }
    verify_exists(&sftp, server_id, path, true).await?;
    let _ = sftp.close().await;
    Ok(())
}

pub async fn rename(
    ssh: &SshConnectionManager,
    server_id: &str,
    old_path: &str,
    new_path: &str,
) -> AppResult<()> {
    validate_path(old_path)?;
    validate_path(new_path)?;
    let sftp = ssh.open_sftp(server_id).await?;
    if sftp
        .try_exists(new_path)
        .await
        .map_err(|error| map_sftp(error, server_id, "无法检查目标路径"))?
    {
        return Err(AppError::new("FILE_CONFLICT", "file", "目标路径已存在").for_server(server_id));
    }
    sftp.rename(old_path, new_path)
        .await
        .map_err(|error| map_sftp(error, server_id, "重命名失败"))?;
    verify_exists(&sftp, server_id, new_path, true).await?;
    verify_exists(&sftp, server_id, old_path, false).await?;
    let _ = sftp.close().await;
    Ok(())
}

pub async fn remove(
    ssh: &SshConnectionManager,
    server_id: &str,
    path: &str,
    recursive: bool,
) -> AppResult<()> {
    validate_path(path)?;
    if recursive && is_protected_path(path) {
        return Err(AppError::new(
            "DANGEROUS_PATH",
            "security",
            "禁止通过普通文件界面递归删除系统根目录",
        )
        .for_server(server_id)
        .fatal());
    }
    let sftp = ssh.open_sftp(server_id).await?;
    let metadata = sftp
        .symlink_metadata(path)
        .await
        .map_err(|error| map_sftp(error, server_id, "无法读取待删除对象"))?;
    if metadata.is_dir() {
        if recursive {
            remove_tree(&sftp, server_id, path).await?;
        } else {
            sftp.remove_dir(path)
                .await
                .map_err(|error| map_sftp(error, server_id, "文件夹删除失败"))?;
        }
    } else {
        sftp.remove_file(path)
            .await
            .map_err(|error| map_sftp(error, server_id, "文件删除失败"))?;
    }
    verify_exists(&sftp, server_id, path, false).await?;
    let _ = sftp.close().await;
    Ok(())
}

async fn remove_tree(sftp: &SftpSession, server_id: &str, root: &str) -> AppResult<()> {
    let mut pending = vec![root.to_string()];
    let mut directories = Vec::new();
    while let Some(path) = pending.pop() {
        directories.push(path.clone());
        let entries = sftp
            .read_dir(path)
            .await
            .map_err(|error| map_sftp(error, server_id, "递归读取文件夹失败"))?;
        for entry in entries {
            if entry.metadata().is_dir() {
                pending.push(entry.path());
            } else {
                sftp.remove_file(entry.path())
                    .await
                    .map_err(|error| map_sftp(error, server_id, "递归删除文件失败"))?;
            }
        }
    }
    for directory in directories.into_iter().rev() {
        sftp.remove_dir(directory)
            .await
            .map_err(|error| map_sftp(error, server_id, "递归删除文件夹失败"))?;
    }
    Ok(())
}

async fn verify_exists(
    sftp: &SftpSession,
    server_id: &str,
    path: &str,
    expected: bool,
) -> AppResult<()> {
    let exists = sftp
        .try_exists(path)
        .await
        .map_err(|error| map_sftp(error, server_id, "无法验证文件操作结果"))?;
    if exists != expected {
        return Err(
            AppError::new("FILE_VERIFY_FAILED", "file", "远程文件操作结果验证失败")
                .for_server(server_id),
        );
    }
    Ok(())
}

fn temporary_sibling(path: &str) -> String {
    let (parent, name) = path.rsplit_once('/').unwrap_or((".", path));
    format!("{parent}/.{name}.relay-{}.tmp", uuid::Uuid::new_v4())
}

fn kind(metadata: &FileAttributes) -> FileKind {
    if metadata.is_dir() {
        FileKind::Directory
    } else if metadata.is_symlink() {
        FileKind::Symlink
    } else if metadata.is_regular() {
        FileKind::File
    } else {
        FileKind::Other
    }
}

fn validate_path(path: &str) -> AppResult<()> {
    if path.is_empty() || path.contains('\0') || path.contains('\n') || path.contains('\r') {
        return Err(AppError::new("INVALID_PATH", "validation", "远程路径无效"));
    }
    Ok(())
}

fn is_protected_path(path: &str) -> bool {
    matches!(
        path.trim_end_matches('/'),
        "" | "/"
            | "/bin"
            | "/boot"
            | "/dev"
            | "/etc"
            | "/home"
            | "/lib"
            | "/lib64"
            | "/opt"
            | "/proc"
            | "/root"
            | "/run"
            | "/sbin"
            | "/srv"
            | "/sys"
            | "/tmp"
            | "/usr"
            | "/var"
    )
}

fn map_sftp(error: impl std::fmt::Display, server_id: &str, message: &str) -> AppError {
    AppError::new("SFTP_FAILED", "sftp", message)
        .details(error)
        .for_server(server_id)
}

#[cfg(test)]
mod tests {
    use super::{is_protected_path, temporary_sibling, validate_path};

    #[test]
    fn protects_system_roots_but_not_children() {
        assert!(is_protected_path("/etc"));
        assert!(is_protected_path("/var/"));
        assert!(!is_protected_path("/etc/nginx"));
        assert!(!is_protected_path("/var/log/app"));
    }

    #[test]
    fn rejects_control_characters_in_paths() {
        assert!(validate_path("/tmp/ok").is_ok());
        assert!(validate_path("/tmp/bad\nname").is_err());
        assert!(validate_path("/tmp/bad\0name").is_err());
    }

    #[test]
    fn temporary_file_stays_beside_target() {
        let value = temporary_sibling("/etc/nginx/nginx.conf");
        assert!(value.starts_with("/etc/nginx/.nginx.conf.relay-"));
        assert!(value.ends_with(".tmp"));
    }
}
