import * as Dialog from "@radix-ui/react-dialog";
import { useMutation, useQuery } from "@tanstack/react-query";
import { Activity, CircleStop, Play, RefreshCw, RotateCw, Search, ServerCog, ShieldAlert, Waypoints } from "lucide-react";
import { useMemo, useState } from "react";
import { NavLink, useParams } from "react-router-dom";
import { Button } from "../../components/ui/Button";
import { api } from "../../lib/api";
import { errorMessage } from "../../lib/errors";
import { formatBytes, formatDuration } from "../../lib/format";
import type { OperationsSnapshot, TerminationResult } from "../../types/server";

type View = "all" | "ports" | "processes" | "services";
type Target = { pid: number; port?: number; command: string; unit?: string | null };

export function OperationsPage() {
  const { serverId = "" } = useParams();
  const [query, setQuery] = useState("");
  const [view, setView] = useState<View>("all");
  const [target, setTarget] = useState<Target | null>(null);
  const [force, setForce] = useState(false);
  const [privileged, setPrivileged] = useState(false);
  const [result, setResult] = useState<TerminationResult | null>(null);
  const profile = useQuery({ queryKey: ["server", serverId], queryFn: () => api.getServer(serverId) });
  const operations = useQuery({ queryKey: ["operations", serverId], queryFn: () => api.operations(serverId), refetchInterval: 10_000 });
  const terminate = useMutation({ mutationFn: () => api.terminateProcess({ serverId, pid: target!.pid, port: target?.port, force, privileged }), onSuccess: async (value) => { setResult(value); setTarget(null); setForce(false); setPrivileged(false); await operations.refetch(); } });
  const service = useMutation({ mutationFn: ({ name, action }: { name: string; action: "start" | "stop" | "restart" }) => api.manageService(serverId, name, action), onSuccess: () => operations.refetch() });
  const filtered = useMemo(() => filterSnapshot(operations.data, query), [operations.data, query]);

  return <section className="operations-page">
    <div className="workspace-header terminal-header"><div><div className="breadcrumb">服务器 / {profile.data?.name ?? "…"} / <span>端口与进程</span></div><h1>运行现场</h1><p>Processes · Listening sockets · systemd</p></div><div className="workspace-header__actions"><Button variant="secondary" onClick={() => operations.refetch()} disabled={operations.isFetching}><RefreshCw className={operations.isFetching ? "spin" : ""} size={14} /> 重新扫描</Button></div></div>
    <nav className="workspace-tabs"><NavLink end to={`/servers/${serverId}`}>概览</NavLink><NavLink to={`/servers/${serverId}/files`}>文件</NavLink><NavLink to={`/servers/${serverId}/terminal`}>终端</NavLink><NavLink className="active" to={`/servers/${serverId}/operations`}>端口与进程</NavLink><button disabled>服务</button><button disabled>Nginx</button><button disabled>Docker</button></nav>
    <div className="operations-toolbar"><label><Search size={16} /><input autoFocus value={query} onChange={(event) => setQuery(event.target.value)} placeholder="搜索 8080、node、nginx、PID 或 service" /></label><div>{(["all", "ports", "processes", "services"] as View[]).map((value) => <button key={value} className={view === value ? "active" : ""} onClick={() => setView(value)}>{({ all: "全部", ports: "监听端口", processes: "进程", services: "服务" })[value]}</button>)}</div></div>
    {operations.isLoading && <div className="page-state">正在从 ps、ss 与 systemd 读取真实状态…</div>}
    {operations.error && <div className="page-state page-state--error">{errorMessage(operations.error)}</div>}
    {result && <div className={`operation-result ${result.processExited && result.portReleased !== false ? "is-success" : ""}`}><Activity size={16} /><span>已发送 SIG{result.signal}：PID {result.pid} {result.processExited ? "已退出" : "仍在运行"}{result.portReleased !== null ? `，端口${result.portReleased ? "已释放" : "仍被占用"}` : ""}</span><button onClick={() => setResult(null)}>关闭</button></div>}
    {operations.data && <div className="operations-content">
      {(view === "all" || view === "ports") && <section className="operations-section"><header><Waypoints size={15} /><strong>监听端口</strong><span>{filtered.ports.length}</span></header><div className="ops-table ports-table"><div className="ops-head"><span>协议</span><span>地址</span><span>端口</span><span>进程</span><span>PID</span><span>操作</span></div>{filtered.ports.map((port, index) => <div className="ops-row" key={`${port.protocol}:${port.localAddress}:${port.port}:${index}`}><span className="protocol-badge">{port.protocol}</span><span className="mono">{port.localAddress}</span><strong>{port.port}</strong><span>{port.processName ?? (port.processVisible ? "—" : "权限不足")}</span><span className="mono">{port.pid ?? "—"}</span><span>{port.pid && <Button variant="danger" size="sm" onClick={() => setTarget({ pid: port.pid!, port: port.port, command: port.processName ?? `PID ${port.pid}` })}><CircleStop size={13} /> 释放端口</Button>}</span></div>)}</div></section>}
      {(view === "all" || view === "processes") && <section className="operations-section"><header><Activity size={15} /><strong>进程</strong><span>{filtered.processes.length}</span></header><div className="ops-table process-table"><div className="ops-head"><span>PID</span><span>用户</span><span>CPU</span><span>内存</span><span>运行时间</span><span>命令</span><span>操作</span></div>{filtered.processes.slice(0, 300).map((process) => <div className="ops-row" key={process.pid}><span className="mono">{process.pid}</span><span>{process.user}</span><span>{process.cpuPercent.toFixed(1)}%</span><span>{formatBytes(process.rssBytes)}</span><span>{formatDuration(process.elapsedSeconds)}</span><span className="command-cell" title={process.command}><strong>{process.name}</strong><small>{process.command}</small></span><span>{process.pid > 1 && <Button variant="ghost" size="sm" onClick={() => setTarget({ pid: process.pid, command: process.command, unit: process.systemdUnit })}>结束</Button>}</span></div>)}</div></section>}
      {(view === "all" || view === "services") && <section className="operations-section"><header><ServerCog size={15} /><strong>systemd 服务</strong><span>{filtered.services.length}</span></header><div className="ops-table services-table"><div className="ops-head"><span>单元</span><span>状态</span><span>说明</span><span>操作</span></div>{filtered.services.map((unit) => <div className="ops-row" key={unit.name}><strong className="mono">{unit.name}</strong><span className={`unit-state is-${unit.active}`}>{unit.active} / {unit.sub}</span><span>{unit.description}</span><span className="service-actions">{unit.active === "active" ? <Button variant="ghost" size="sm" onClick={() => service.mutate({ name: unit.name, action: "stop" })}><CircleStop size={12} /> 停止</Button> : <Button variant="ghost" size="sm" onClick={() => service.mutate({ name: unit.name, action: "start" })}><Play size={12} /> 启动</Button>}<Button variant="ghost" size="sm" onClick={() => service.mutate({ name: unit.name, action: "restart" })}><RotateCw size={12} /> 重启</Button></span></div>)}</div></section>}
    </div>}
    <Dialog.Root open={!!target} onOpenChange={(open) => !open && setTarget(null)}><Dialog.Portal><Dialog.Overlay className="dialog-overlay" /><Dialog.Content className="dialog-content dialog-content--narrow confirm-dialog"><div className="destructive-icon"><ShieldAlert size={22} /></div><Dialog.Title>{target?.port ? `释放端口 ${target.port}` : `结束 PID ${target?.pid}`}</Dialog.Title><Dialog.Description>将向 PID {target?.pid} 发送 {force ? "SIGKILL（强制）" : "SIGTERM（优雅退出）"}，然后重新扫描验证结果。</Dialog.Description><dl className="process-confirm"><div><dt>命令</dt><dd>{target?.command}</dd></div>{target?.unit && <div><dt>systemd</dt><dd>{target.unit}（建议优先停止服务）</dd></div>}</dl><div className="terminate-options"><label className="force-toggle"><input type="checkbox" checked={force} onChange={(event) => setForce(event.target.checked)} /> 强制使用 SIGKILL</label>{profile.data?.sudoMode !== "none" && <label className="force-toggle"><input type="checkbox" checked={privileged} onChange={(event) => setPrivileged(event.target.checked)} /> 使用已配置的 sudo</label>}</div>{terminate.error && <div className="form-error">{errorMessage(terminate.error)}</div>}<div className="dialog-actions"><Button variant="ghost" onClick={() => setTarget(null)}>取消</Button><Button variant="danger" onClick={() => terminate.mutate()} disabled={terminate.isPending}>{terminate.isPending ? "正在验证…" : force ? "强制结束并验证" : "结束并验证"}</Button></div></Dialog.Content></Dialog.Portal></Dialog.Root>
  </section>;
}

function filterSnapshot(value: OperationsSnapshot | undefined, query: string): OperationsSnapshot {
  if (!value) return { processes: [], ports: [], services: [] };
  const needle = query.trim().toLocaleLowerCase();
  if (!needle) return value;
  return {
    processes: value.processes.filter((process) => [process.pid, process.ppid, process.user, process.name, process.command, process.systemdUnit].some((item) => String(item ?? "").toLocaleLowerCase().includes(needle))),
    ports: value.ports.filter((port) => [port.port, port.pid, port.protocol, port.localAddress, port.processName].some((item) => String(item ?? "").toLocaleLowerCase().includes(needle))),
    services: value.services.filter((unit) => [unit.name, unit.active, unit.sub, unit.description].some((item) => item.toLocaleLowerCase().includes(needle))),
  };
}
