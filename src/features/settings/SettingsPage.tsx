import { ClipboardList, Download, Info, Languages, LockKeyhole, Moon, RotateCcw, RefreshCw, Upload } from "lucide-react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { type ChangeEvent, useEffect, useRef, useState } from "react";
import { Button } from "../../components/ui/Button";
import { api } from "../../lib/api";
import { errorMessage } from "../../lib/errors";
import { applyLocale, readLocale, saveLocale, type Locale } from "../../lib/i18n";
import { pushNotice } from "../../lib/noticeStore";
import type { PublicServerImport } from "../../types/server";

/** 展示本地偏好和不含 secret 的服务器配置导入/导出操作。 */
export function SettingsPage() {
  const inputRef = useRef<HTMLInputElement>(null);
  const fullBackupInputRef = useRef<HTMLInputElement>(null);
  const queryClient = useQueryClient();
  const [busy, setBusy] = useState(false);
  const [backupPassword, setBackupPassword] = useState("");
  const [theme, setTheme] = useState<"system" | "dark" | "light">(() => (localStorage.getItem("relay.theme") as "system" | "dark" | "light" | null) ?? "system");
  const [locale, setLocale] = useState<Locale>(() => readLocale());
  const [restoreWorkspace, setRestoreWorkspace] = useState(() => localStorage.getItem("relay.restoreWorkspace") !== "false");

  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: light)");
    const apply = () => { document.documentElement.dataset.theme = theme === "system" ? (media.matches ? "light" : "dark") : theme; };
    apply();
    localStorage.setItem("relay.theme", theme);
    if (theme === "system") media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [theme]);
  useEffect(() => { localStorage.setItem("relay.restoreWorkspace", String(restoreWorkspace)); }, [restoreWorkspace]);
  useEffect(() => { saveLocale(locale); applyLocale(locale); }, [locale]);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => { if (message) pushNotice("success", message); }, [message]);
  useEffect(() => { if (error) pushNotice("error", error); }, [error]);
  const audit = useQuery({ queryKey: ["audit-events"], queryFn: () => api.listAuditEvents(30) });

  /** 读取公共配置并下载 JSON；响应内容不包含 Keychain secret。 */
  const exportServers = async () => {
    setBusy(true); setError(null); setMessage(null);
    try {
      const payload = await api.exportServers();
      const blob = new Blob([JSON.stringify(payload, null, 2)], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url; anchor.download = "relay-server-config.json"; anchor.click();
      URL.revokeObjectURL(url);
      setMessage(`已导出 ${payload.servers.length} 台服务器的非敏感配置`);
    } catch (reason) { setError(errorMessage(reason)); } finally { setBusy(false); }
  };

  /** 校验版本化公共 JSON 后交给 Rust 端生成新服务器档案。 */
  const importServers = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0]; event.target.value = "";
    if (!file) return;
    setBusy(true); setError(null); setMessage(null);
    try {
      const payload = JSON.parse(await file.text()) as { format?: string; version?: number; encrypted?: boolean; servers?: PublicServerImport[] };
      if (payload.format !== "agentless-server-manager-backup" || payload.version !== 1 || payload.encrypted || !Array.isArray(payload.servers)) throw new Error("不是受支持的公共服务器配置文件");
      const imported = await api.importServers(payload.servers);
      await queryClient.invalidateQueries({ queryKey: ["servers"] });
      setMessage(`已导入 ${imported.length} 台服务器；密码和 sudo 凭据未包含，请逐台重新配置`);
    } catch (reason) { setError(errorMessage(reason)); } finally { setBusy(false); }
  };

  /** 使用用户临时输入的密码导出包含凭据的加密备份；密码不写入本地设置。 */
  const exportFullBackup = async () => {
    if (!backupPassword) { setError("请输入完整备份密码"); return; }
    setBusy(true); setError(null); setMessage(null);
    try {
      const payload = await api.exportFullBackup(backupPassword);
      const blob = new Blob([payload], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url; anchor.download = "relay-server-full-backup.json"; anchor.click();
      URL.revokeObjectURL(url);
      setMessage("已导出加密完整备份；请单独安全保存备份密码");
    } catch (reason) { setError(errorMessage(reason)); } finally { setBusy(false); }
  };

  /** 将加密备份交给 Rust 解密并重新写入系统凭据库。 */
  const importFullBackup = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0]; event.target.value = "";
    if (!file) return;
    if (!backupPassword) { setError("请先输入完整备份密码"); return; }
    setBusy(true); setError(null); setMessage(null);
    try {
      const imported = await api.importFullBackup(await file.text(), backupPassword);
      await queryClient.invalidateQueries({ queryKey: ["servers"] });
      setMessage(`已解密并导入 ${imported.length} 台服务器；每条记录均生成了新的本地 ID`);
    } catch (reason) { setError(errorMessage(reason)); } finally { setBusy(false); }
  };

  /** 生成脱敏诊断 JSON 下载；导出内容不包含密码、私钥内容或远端命令输出。 */
  const exportDiagnostics = async () => {
    setBusy(true); setError(null); setMessage(null);
    try {
      const payload = await api.exportDiagnostics();
      const blob = new Blob([JSON.stringify(payload, null, 2)], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url; anchor.download = "relay-diagnostics.json"; anchor.click();
      URL.revokeObjectURL(url);
      setMessage("已导出脱敏诊断信息；其中不包含凭据和远端输出");
    } catch (reason) { setError(errorMessage(reason)); } finally { setBusy(false); }
  };

  return (
    <section className="settings-page">
      <div className="workspace-header">
        <div><div className="breadcrumb">应用</div><h1>设置</h1><p>本地偏好与安全策略</p></div>
      </div>
      <div className="settings-list">
        <article><Moon /><div><strong>外观</strong><p>主题选择保存在本机，不影响服务器。</p></div><select value={theme} onChange={(event) => setTheme(event.target.value as "system" | "dark" | "light")}><option value="system">跟随系统</option><option value="dark">深色</option><option value="light">浅色</option></select></article>
        <article><Languages /><div><strong>语言偏好</strong><p>选择保存在本机；翻译资源会按同一 locale key 扩展。</p></div><select value={locale} onChange={(event) => setLocale(event.target.value as Locale)}><option value="zh-CN">简体中文</option><option value="en-US">English</option></select></article>
        <article><LockKeyhole /><div><strong>凭据</strong><p>SSH 与 sudo 密码存储在操作系统安全存储，不写入数据库。</p></div><span className="settings-badge">已保护</span></article>
        <article><RotateCcw /><div><strong>连接恢复</strong><p>保存是否恢复上次打开的工作区偏好；不会自动重放危险任务。</p></div><input type="checkbox" checked={restoreWorkspace} onChange={(event) => setRestoreWorkspace(event.target.checked)} /></article>
        <article className="settings-actions"><RefreshCw /><div><strong>更新能力</strong><p>更新通道和签名包校验入口已预留；当前版本以应用发布包为准，不会静默更新。</p></div><Button variant="ghost" size="sm" disabled><RefreshCw size={13} /> 检查更新（预留）</Button></article>
        <article className="settings-actions"><Download /><div><strong>服务器配置</strong><p>普通导出不包含密码、私钥内容或 sudo 凭据；导入会生成新档案，避免覆盖现有配置。</p></div><div><Button variant="secondary" size="sm" onClick={() => void exportServers()} disabled={busy}><Download size={13} /> 导出 JSON</Button><Button variant="ghost" size="sm" onClick={() => inputRef.current?.click()} disabled={busy}><Upload size={13} /> 导入 JSON</Button><input ref={inputRef} type="file" accept="application/json,.json" hidden onChange={(event) => void importServers(event)} /></div></article>
        <article className="settings-actions settings-backup"><LockKeyhole /><div><strong>加密完整备份</strong><p>使用 Argon2id + AES-256-GCM 加密配置和系统凭据；密码只在本次操作中使用。</p><label className="settings-secret"><span>备份密码</span><input type="password" autoComplete="new-password" value={backupPassword} onChange={(event) => setBackupPassword(event.target.value)} /></label></div><div><Button variant="secondary" size="sm" onClick={() => void exportFullBackup()} disabled={busy || !backupPassword}><Download size={13} /> 导出加密备份</Button><Button variant="ghost" size="sm" onClick={() => fullBackupInputRef.current?.click()} disabled={busy || !backupPassword}><Upload size={13} /> 导入加密备份</Button><input ref={fullBackupInputRef} type="file" accept="application/json,.json" hidden onChange={(event) => void importFullBackup(event)} /></div></article>
        <article className="settings-actions"><ClipboardList /><div><strong>诊断与审计</strong><p>导出本地连接状态、非敏感档案和最近审计记录，便于排障；不包含凭据和远端输出。</p></div><Button variant="secondary" size="sm" onClick={() => void exportDiagnostics()} disabled={busy}><Download size={13} /> 导出诊断 JSON</Button></article>
        <article className="settings-audit"><div><strong>最近审计记录</strong><p>只保存本地操作元数据，不保存命令输出。</p></div>{audit.isLoading && <small>正在读取…</small>}{audit.error && <small className="text-danger">{errorMessage(audit.error)}</small>}{audit.data?.length ? <div className="settings-audit__list">{audit.data.slice(0, 8).map((event) => <div key={event.id}><span className={`audit-result is-${event.result}`}>{event.result}</span><strong>{event.summary}</strong><small>{new Date(event.createdAt).toLocaleString()}</small></div>)}</div> : !audit.isLoading && <small>尚无审计记录。</small>}</article>
        <article><Info /><div><strong>关于 Relay</strong><p>Agentless Server Manager · 版本 0.3.0 · 本地优先桌面应用</p></div><span className="settings-badge">可诊断</span></article>
      </div>
      {message && <div className="settings-feedback is-success">{message}</div>}
      {error && <div className="settings-feedback is-error">{error}</div>}
    </section>
  );
}
