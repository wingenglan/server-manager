import Editor from "@monaco-editor/react";
import * as ContextMenu from "@radix-ui/react-context-menu";
import * as Dialog from "@radix-ui/react-dialog";
import { useVirtualizer } from "@tanstack/react-virtual";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { ArrowDownToLine, ArrowLeft, ArrowRight, ArrowUp, File, FilePlus2, FileText, Folder, FolderPlus, FolderUp, Pencil, RefreshCw, Save, Search, Trash2, Upload, X } from "lucide-react";
import { type KeyboardEvent as ReactKeyboardEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { NavLink, useParams } from "react-router-dom";
import { Button } from "../../components/ui/Button";
import { api } from "../../lib/api";
import { errorMessage, isAppError } from "../../lib/errors";
import { formatBytes } from "../../lib/format";
import type { RemoteFileEntry, RemoteTextFile } from "../../types/server";
import { useTransferStore } from "./transferStore";

type SortKey = "name" | "size" | "modifiedAt";
const join = (base: string, name: string) => base === "/" ? `/${name}` : `${base.replace(/\/$/, "")}/${name}`;
const parent = (value: string) => { if (value === "/") return "/"; const clean = value.replace(/\/$/, ""); return clean.slice(0, clean.lastIndexOf("/")) || "/"; };

export function FilesPage() {
  const { serverId = "" } = useParams();
  const queryClient = useQueryClient();
  const [path, setPath] = useState("/");
  const [pathInput, setPathInput] = useState("/");
  const [history, setHistory] = useState(["/"]);
  const [historyIndex, setHistoryIndex] = useState(0);
  const [filter, setFilter] = useState("");
  const [showHidden, setShowHidden] = useState(false);
  const [sort, setSort] = useState<SortKey>("name");
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [editorFile, setEditorFile] = useState<RemoteTextFile | null>(null);
  const [editorValue, setEditorValue] = useState("");
  const [editorDirty, setEditorDirty] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<RemoteFileEntry | null>(null);
  const [dropActive, setDropActive] = useState(false);
  const listRef = useRef<HTMLDivElement>(null);
  const listing = useQuery({ queryKey: ["files", serverId, path], queryFn: () => api.listDirectory(serverId, path) });
  const profile = useQuery({ queryKey: ["server", serverId], queryFn: () => api.getServer(serverId) });

  const entries = useMemo(() => {
    const values = (listing.data?.entries ?? []).filter((entry) => (showHidden || !entry.name.startsWith(".")) && entry.name.toLocaleLowerCase().includes(filter.toLocaleLowerCase()));
    return values.sort((left, right) => {
      if (left.kind === "directory" && right.kind !== "directory") return -1;
      if (right.kind === "directory" && left.kind !== "directory") return 1;
      if (sort === "size") return left.size - right.size;
      if (sort === "modifiedAt") return (right.modifiedAt ?? 0) - (left.modifiedAt ?? 0);
      return left.name.localeCompare(right.name, "zh-CN", { numeric: true });
    });
  }, [listing.data, showHidden, filter, sort]);
  // TanStack Virtual deliberately returns an imperative object that React Compiler cannot memoize.
  // eslint-disable-next-line react-hooks/incompatible-library
  const virtualizer = useVirtualizer({ count: entries.length, getScrollElement: () => listRef.current, estimateSize: () => 38, overscan: 12 });

  const navigatePath = (next: string, record = true) => {
    setPath(next); setPathInput(next); setSelected(new Set());
    if (record) { const nextHistory = [...history.slice(0, historyIndex + 1), next]; setHistory(nextHistory); setHistoryIndex(nextHistory.length - 1); }
  };
  const refresh = useCallback(() => { void queryClient.invalidateQueries({ queryKey: ["files", serverId, path] }); }, [path, queryClient, serverId]);
  const create = useMutation({ mutationFn: ({ name, directory }: { name: string; directory: boolean }) => api.createEntry(serverId, join(path, name), directory), onSuccess: refresh });
  const rename = useMutation({ mutationFn: ({ entry, name }: { entry: RemoteFileEntry; name: string }) => api.renameEntry(serverId, entry.path, join(path, name)), onSuccess: refresh });
  const remove = useMutation({ mutationFn: (entry: RemoteFileEntry) => api.removeEntry(serverId, entry.path, entry.kind === "directory"), onSuccess: () => { setDeleteTarget(null); void refresh(); } });

  const openEntry = async (entry: RemoteFileEntry) => {
    if (entry.kind === "directory") { navigatePath(entry.path); return; }
    try { const file = await api.readText(serverId, entry.path); setEditorFile(file); setEditorValue(file.content); setEditorDirty(false); } catch (reason) { window.alert(errorMessage(reason)); }
  };
  const promptCreate = (directory: boolean) => { const name = window.prompt(directory ? "新文件夹名称" : "新文件名称"); if (name?.trim()) create.mutate({ name: name.trim(), directory }); };
  const promptRename = (entry: RemoteFileEntry) => { const name = window.prompt("重命名为", entry.name); if (name?.trim() && name.trim() !== entry.name) rename.mutate({ entry, name: name.trim() }); };

  const startUpload = useCallback((localPath: string) => {
    const id = crypto.randomUUID();
    useTransferStore.getState().add({ id, label: localPath.split(/[\\/]/).pop() ?? localPath, direction: "upload", status: "queued", transferredBytes: 0, totalBytes: null, bytesPerSecond: 0, currentPath: path });
    void api.upload(id, serverId, localPath, path, (event) => useTransferStore.getState().event(event)).then(refresh).catch((reason) => { if (!isAppError(reason) || reason.code !== "CANCELLED") useTransferStore.getState().fail(id, errorMessage(reason)); });
  }, [path, refresh, serverId]);
  const chooseUpload = async () => { const paths = await open({ multiple: true, title: "选择要上传的文件" }); if (paths) (Array.isArray(paths) ? paths : [paths]).forEach(startUpload); };
  const startDownload = async (entry: RemoteFileEntry) => {
    const directory = await open({ directory: true, recursive: true, title: `选择“${entry.name}”的下载位置` });
    if (!directory || Array.isArray(directory)) return;
    const id = crypto.randomUUID();
    useTransferStore.getState().add({ id, label: entry.name, direction: "download", status: "queued", transferredBytes: 0, totalBytes: null, bytesPerSecond: 0, currentPath: entry.path });
    void api.download(id, serverId, entry.path, directory, (event) => useTransferStore.getState().event(event)).catch((reason) => { if (!isAppError(reason) || reason.code !== "CANCELLED") useTransferStore.getState().fail(id, errorMessage(reason)); });
  };

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "over") setDropActive(true);
      else if (event.payload.type === "leave") setDropActive(false);
      else if (event.payload.type === "drop") { setDropActive(false); event.payload.paths.forEach(startUpload); }
    }).then((value) => { unlisten = value; });
    return () => unlisten?.();
  }, [startUpload]);

  const onKeyDown = (event: ReactKeyboardEvent) => {
    const current = entries.find((entry) => selected.has(entry.path));
    if (event.key === "F2" && current) { event.preventDefault(); promptRename(current); }
    if (event.key === "Delete" && current) { event.preventDefault(); setDeleteTarget(current); }
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "a") { event.preventDefault(); setSelected(new Set(entries.map((entry) => entry.path))); }
  };

  return <section className="files-page" onKeyDown={onKeyDown} tabIndex={-1}>
    <div className="workspace-header terminal-header"><div><div className="breadcrumb">服务器 / {profile.data?.name ?? "…"} / <span>文件</span></div><h1>远程文件</h1><p>SFTP · {profile.data ? `${profile.data.username}@${profile.data.host}` : "正在载入"}</p></div><div className="workspace-header__actions"><Button variant="secondary" onClick={chooseUpload}><Upload size={14} /> 上传</Button><Button variant="ghost" onClick={() => promptCreate(true)}><FolderPlus size={14} /> 新建文件夹</Button><Button variant="ghost" onClick={() => promptCreate(false)}><FilePlus2 size={14} /> 新建文件</Button></div></div>
    <nav className="workspace-tabs"><NavLink end to={`/servers/${serverId}`}>概览</NavLink><NavLink className="active" to={`/servers/${serverId}/files`}>文件</NavLink><NavLink to={`/servers/${serverId}/terminal`}>终端</NavLink><NavLink to={`/servers/${serverId}/operations`}>端口与进程</NavLink><button disabled>服务</button><button disabled>Nginx</button><button disabled>Docker</button></nav>
    <div className="file-toolbar"><div className="file-history"><button disabled={historyIndex === 0} onClick={() => { const index = historyIndex - 1; setHistoryIndex(index); navigatePath(history[index], false); }}><ArrowLeft size={14} /></button><button disabled={historyIndex >= history.length - 1} onClick={() => { const index = historyIndex + 1; setHistoryIndex(index); navigatePath(history[index], false); }}><ArrowRight size={14} /></button><button disabled={path === "/"} onClick={() => navigatePath(parent(path))}><ArrowUp size={14} /></button><button onClick={() => void refresh()}><RefreshCw size={14} /></button></div><form className="path-input" onSubmit={(event) => { event.preventDefault(); navigatePath(pathInput); }}><FolderUp size={14} /><input value={pathInput} onChange={(event) => setPathInput(event.target.value)} /><button>转到</button></form><label className="file-filter"><Search size={13} /><input value={filter} onChange={(event) => setFilter(event.target.value)} placeholder="筛选当前目录" /></label><label className="hidden-toggle"><input type="checkbox" checked={showHidden} onChange={(event) => setShowHidden(event.target.checked)} /> 隐藏文件</label></div>
    <div className="file-browser"><div className="file-columns"><button onClick={() => setSort("name")}>名称</button><button onClick={() => setSort("size")}>大小</button><span>类型</span><span>权限</span><span>所有者</span><button onClick={() => setSort("modifiedAt")}>修改时间</button></div><div className="file-list" ref={listRef}>
      {listing.isLoading && <div className="file-state">正在读取远程目录…</div>}{listing.error && <div className="file-state is-error">{errorMessage(listing.error)}<Button size="sm" onClick={() => void refresh()}>重试</Button></div>}{!listing.isLoading && !listing.error && !entries.length && <div className="file-state">此目录为空</div>}
      <div style={{ height: `${virtualizer.getTotalSize()}px`, position: "relative" }}>{virtualizer.getVirtualItems().map((item) => { const entry = entries[item.index]; return <ContextMenu.Root key={entry.path}><ContextMenu.Trigger asChild><div className={`file-row ${selected.has(entry.path) ? "is-selected" : ""}`} style={{ transform: `translateY(${item.start}px)` }} onClick={(event) => { if (event.ctrlKey || event.metaKey) setSelected((current) => { const next = new Set(current); if (next.has(entry.path)) next.delete(entry.path); else next.add(entry.path); return next; }); else setSelected(new Set([entry.path])); }} onDoubleClick={() => void openEntry(entry)}><span className="file-name">{entry.kind === "directory" ? <Folder size={15} /> : entry.kind === "file" ? <FileText size={15} /> : <File size={15} />}<strong>{entry.name}</strong></span><span>{entry.kind === "directory" ? "—" : formatBytes(entry.size)}</span><span>{entry.kind === "directory" ? "文件夹" : entry.kind === "symlink" ? "符号链接" : "文件"}</span><span className="mono">{entry.permissions}</span><span>{entry.owner}:{entry.group}</span><span>{entry.modifiedAt ? new Date(entry.modifiedAt * 1000).toLocaleString() : "—"}</span></div></ContextMenu.Trigger><ContextMenu.Portal><ContextMenu.Content className="file-context"><ContextMenu.Item onSelect={() => void openEntry(entry)}>{entry.kind === "directory" ? <Folder size={13} /> : <FileText size={13} />} 打开</ContextMenu.Item><ContextMenu.Item onSelect={() => promptRename(entry)}><Pencil size={13} /> 重命名 <kbd>F2</kbd></ContextMenu.Item><ContextMenu.Item onSelect={() => void startDownload(entry)}><ArrowDownToLine size={13} /> 下载</ContextMenu.Item><ContextMenu.Separator /><ContextMenu.Item className="is-danger" onSelect={() => setDeleteTarget(entry)}><Trash2 size={13} /> 删除 <kbd>Del</kbd></ContextMenu.Item></ContextMenu.Content></ContextMenu.Portal></ContextMenu.Root>; })}</div>
    </div><footer className="file-status"><span>{entries.length} 项 · 已选 {selected.size} 项</span><span>双击打开 · F2 重命名 · Delete 删除</span></footer>{dropActive && <div className="file-drop"><Upload size={28} /><strong>上传到 {path}</strong><span>释放即可开始递归传输</span></div>}</div>
    {editorFile && <FileEditor file={editorFile} value={editorValue} dirty={editorDirty} canSudo={profile.data?.sudoMode !== "none"} onChange={(value) => { setEditorValue(value); setEditorDirty(value !== editorFile.content); }} onClose={() => { if (!editorDirty || window.confirm("放弃未保存的修改？")) setEditorFile(null); }} onSaved={(file) => { setEditorFile(file); setEditorValue(file.content); setEditorDirty(false); void refresh(); }} />}
    <Dialog.Root open={!!deleteTarget} onOpenChange={(opened) => !opened && setDeleteTarget(null)}><Dialog.Portal><Dialog.Overlay className="dialog-overlay" /><Dialog.Content className="dialog-content dialog-content--narrow confirm-dialog"><div className="destructive-icon"><Trash2 size={22} /></div><Dialog.Title>删除“{deleteTarget?.name}”</Dialog.Title><Dialog.Description>{deleteTarget?.kind === "directory" ? "将递归删除该文件夹及其全部内容。" : "将永久删除该远程文件。"} 删除后无法从本应用恢复。</Dialog.Description>{remove.error && <div className="form-error">{errorMessage(remove.error)}</div>}<div className="dialog-actions"><Button variant="ghost" onClick={() => setDeleteTarget(null)}>取消</Button><Button variant="danger" onClick={() => deleteTarget && remove.mutate(deleteTarget)} disabled={remove.isPending}>删除 {deleteTarget?.path}</Button></div></Dialog.Content></Dialog.Portal></Dialog.Root>
  </section>;
}

function FileEditor({ file, value, dirty, canSudo, onChange, onClose, onSaved }: { file: RemoteTextFile; value: string; dirty: boolean; canSudo: boolean; onChange: (value: string) => void; onClose: () => void; onSaved: (file: RemoteTextFile) => void }) {
  const { serverId = "" } = useParams();
  const [conflict, setConflict] = useState<"normal" | "sudo" | null>(null);
  const save = useMutation<RemoteTextFile, unknown, boolean>({ mutationFn: (force) => api.saveText({ serverId, path: file.path, content: value, expectedSize: file.size, expectedModifiedAt: file.modifiedAt, force }), onSuccess: (saved) => { setConflict(null); onSaved(saved); }, onError: (reason) => { if (isAppError(reason) && reason.code === "FILE_CONFLICT") setConflict("normal"); } });
  const sudoSave = useMutation<RemoteTextFile, unknown, boolean>({ mutationFn: (force) => api.saveTextPrivileged({ serverId, path: file.path, content: value, expectedSize: file.size, expectedModifiedAt: file.modifiedAt, force }), onSuccess: (saved) => { setConflict(null); onSaved(saved); }, onError: (reason) => { if (isAppError(reason) && reason.code === "FILE_CONFLICT") setConflict("sudo"); } });
  useEffect(() => { const handler = (event: KeyboardEvent) => { if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") { event.preventDefault(); if (dirty) save.mutate(false); } }; window.addEventListener("keydown", handler); return () => window.removeEventListener("keydown", handler); }, [dirty, save]);
  return <div className="editor-overlay"><div className="editor-window"><header><div><FileText size={16} /><strong>{file.path}</strong>{dirty && <i>未保存</i>}</div><div>{canSudo && <Button variant="ghost" size="sm" onClick={() => sudoSave.mutate(false)} disabled={!dirty || sudoSave.isPending}>sudo 保存</Button>}<Button size="sm" onClick={() => save.mutate(false)} disabled={!dirty || save.isPending}><Save size={13} /> {save.isPending ? "保存中…" : "保存"}</Button><button onClick={onClose}><X size={16} /></button></div></header>{!!(save.error || sudoSave.error) && <div className="editor-error">{errorMessage(save.error ?? sudoSave.error)}</div>}<Editor height="100%" path={file.path} value={value} onChange={(next) => onChange(next ?? "")} theme="vs-dark" options={{ minimap: { enabled: false }, wordWrap: "on", fontSize: 13, automaticLayout: true, scrollBeyondLastLine: false }} /></div>{conflict && <div className="editor-conflict"><strong>远程文件已改变</strong><p>保存已被阻止，避免覆盖其他程序的修改。可关闭并重新打开查看最新内容，或明确强制覆盖。</p><div><Button variant="ghost" onClick={() => setConflict(null)}>继续比较</Button><Button variant="danger" onClick={() => conflict === "sudo" ? sudoSave.mutate(true) : save.mutate(true)}>强制覆盖</Button></div></div>}</div>;
}
