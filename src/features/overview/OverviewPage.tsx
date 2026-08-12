import * as Dialog from "@radix-ui/react-dialog";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Activity, Box, Cpu, HardDrive, KeyRound, LogOut, MemoryStick, Network, Pencil, Power, RefreshCw, ShieldAlert, TerminalSquare, Trash2 } from "lucide-react";
import { useState } from "react";
import { NavLink, useNavigate, useParams } from "react-router-dom";
import { Button } from "../../components/ui/Button";
import { api, isHostKeyChallenge } from "../../lib/api";
import { formatBytes, formatDuration } from "../../lib/format";
import { errorMessage } from "../../lib/errors";
import type { HostKeyChallenge } from "../../types/server";
import { ServerDialog } from "../servers/ServerDialog";

export function OverviewPage() {
  const { serverId = "" } = useParams();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [challenge, setChallenge] = useState<HostKeyChallenge | null>(null);
  const [editOpen, setEditOpen] = useState(false);
  const [deleteOpen, setDeleteOpen] = useState(false);
  const profile = useQuery({ queryKey: ["server", serverId], queryFn: () => api.getServer(serverId), enabled: !!serverId });
  const connection = useQuery({ queryKey: ["connection", serverId], queryFn: () => api.connectionState(serverId), enabled: !!serverId, refetchInterval: 5000 });
  const overview = useQuery({ queryKey: ["overview", serverId], queryFn: () => api.overview(serverId), enabled: connection.data?.status === "online", refetchInterval: 5000 });
  const connect = useMutation({ mutationFn: () => api.connectServer(serverId), onSuccess: async (value) => { if (isHostKeyChallenge(value)) setChallenge(value); else await queryClient.invalidateQueries({ queryKey: ["connection", serverId] }); } });
  const trust = useMutation({ mutationFn: () => api.trustHostKey(challenge!), onSuccess: async () => { setChallenge(null); await queryClient.invalidateQueries({ queryKey: ["connection", serverId] }); } });
  const disconnect = useMutation({ mutationFn: () => api.disconnectServer(serverId), onSuccess: async () => queryClient.invalidateQueries({ queryKey: ["connection", serverId] }) });
  const remove = useMutation({ mutationFn: () => api.deleteServer(serverId), onSuccess: async () => { await queryClient.invalidateQueries({ queryKey: ["servers"] }); navigate("/"); } });

  if (profile.isLoading) return <div className="page-state">正在读取服务器配置…</div>;
  if (profile.error || !profile.data) return <div className="page-state page-state--error">无法读取服务器配置。</div>;
  const server = profile.data;
  const online = connection.data?.status === "online";

  return (
    <section className="overview-page">
      <div className="workspace-header">
        <div><div className="breadcrumb">服务器 / <span>{server.name}</span></div><h1>{server.name}</h1><p>{server.username}@{server.host}:{server.port}</p></div>
        <div className="workspace-header__actions"><span className={`connection-pill ${online ? "is-online" : ""}`}><i /> {online ? "已连接" : connection.data?.status === "connecting" ? "连接中" : "离线"}</span><Button variant="ghost" onClick={() => setEditOpen(true)}><Pencil size={14} /> 编辑</Button>{online ? <><Button variant="ghost" onClick={() => disconnect.mutate()} disabled={disconnect.isPending}><LogOut size={14} /> 断开</Button><Button variant="secondary" onClick={() => navigate(`/servers/${serverId}/terminal`)}><TerminalSquare size={15} /> 打开终端</Button></> : <Button variant="primary" onClick={() => connect.mutate()} disabled={connect.isPending}><Power size={15} /> {connect.isPending ? "连接中…" : "连接"}</Button>}<Button variant="danger" onClick={() => setDeleteOpen(true)} aria-label="删除服务器"><Trash2 size={14} /></Button></div>
      </div>
      <nav className="workspace-tabs"><NavLink end to={`/servers/${serverId}`}>概览</NavLink><NavLink to={`/servers/${serverId}/files`}>文件</NavLink><NavLink to={`/servers/${serverId}/terminal`}>终端</NavLink><NavLink to={`/servers/${serverId}/operations`}>端口与进程</NavLink><button disabled>服务</button><button disabled>Nginx</button><button disabled>Docker</button></nav>

      {!online && <div className="connect-panel"><div className="connect-panel__icon"><Network size={27} /></div><h2>建立安全 SSH 会话</h2><p>连接后将从远程标准接口读取真实系统状态。首次连接需要核对服务器 Host Key 指纹。</p><Button variant="primary" onClick={() => connect.mutate()} disabled={connect.isPending}>{connect.isPending ? <RefreshCw className="spin" size={16} /> : <KeyRound size={16} />} {connect.isPending ? "正在握手" : "连接服务器"}</Button>{connect.error && <div className="form-error">{errorMessage(connect.error)}</div>}</div>}

      {online && overview.isLoading && <div className="metrics-skeleton"><div /><div /><div /><div /></div>}
      {online && overview.data && <OverviewContent data={overview.data} />}

      <Dialog.Root open={!!challenge} onOpenChange={(open) => !open && setChallenge(null)}><Dialog.Portal><Dialog.Overlay className="dialog-overlay" /><Dialog.Content className="dialog-content dialog-content--narrow"><div className="hostkey-hero"><ShieldAlert size={30} /><span>首次连接</span></div><Dialog.Title>核对服务器身份</Dialog.Title><Dialog.Description>服务器返回了一个尚未信任的 Host Key。请通过可信渠道核对指纹后再继续。</Dialog.Description><dl className="fingerprint"><div><dt>主机</dt><dd>{challenge?.host}:{challenge?.port}</dd></div><div><dt>算法</dt><dd>{challenge?.keyType}</dd></div><div><dt>SHA256 指纹</dt><dd>{challenge?.fingerprint}</dd></div></dl><div className="dialog-actions"><Button variant="ghost" onClick={() => setChallenge(null)}>取消</Button><Button variant="primary" onClick={() => trust.mutate()} disabled={trust.isPending}>信任并连接</Button></div></Dialog.Content></Dialog.Portal></Dialog.Root>
      <ServerDialog key={`${server.updatedAt}:${editOpen}`} open={editOpen} onOpenChange={setEditOpen} profile={server} />
      <Dialog.Root open={deleteOpen} onOpenChange={setDeleteOpen}><Dialog.Portal><Dialog.Overlay className="dialog-overlay" /><Dialog.Content className="dialog-content dialog-content--narrow confirm-dialog"><div className="destructive-icon"><Trash2 size={22} /></div><Dialog.Title>删除“{server.name}”</Dialog.Title><Dialog.Description>这会删除本机保存的服务器档案和关联凭据，不会删除或修改远端服务器。此操作无法撤销。</Dialog.Description>{remove.error && <div className="form-error">{errorMessage(remove.error)}</div>}<div className="dialog-actions"><Button variant="ghost" onClick={() => setDeleteOpen(false)}>取消</Button><Button variant="danger" onClick={() => remove.mutate()} disabled={remove.isPending}>{remove.isPending ? "正在删除…" : `删除 ${server.name}`}</Button></div></Dialog.Content></Dialog.Portal></Dialog.Root>
    </section>
  );
}

function OverviewContent({ data }: { data: Awaited<ReturnType<typeof api.overview>> }) {
  const memoryUsed = data.memoryTotalBytes - data.memoryAvailableBytes;
  return <div className="overview-grid">
    <article className="metric-card metric-card--accent"><div className="metric-card__top"><span>CPU 使用率</span><Cpu size={18} /></div><strong>{data.cpuUsagePercent?.toFixed(1) ?? "—"}<small>%</small></strong><div className="meter"><i style={{ width: `${data.cpuUsagePercent ?? 0}%` }} /></div><footer>{data.cpuModel} · {data.logicalCores} 核</footer></article>
    <article className="metric-card"><div className="metric-card__top"><span>内存</span><MemoryStick size={18} /></div><strong>{formatBytes(memoryUsed)}<small> / {formatBytes(data.memoryTotalBytes)}</small></strong><div className="meter"><i style={{ width: `${(memoryUsed / data.memoryTotalBytes) * 100}%` }} /></div><footer>可用 {formatBytes(data.memoryAvailableBytes)}</footer></article>
    <article className="metric-card"><div className="metric-card__top"><span>负载</span><Activity size={18} /></div><strong>{data.load[0].toFixed(2)}</strong><div className="load-values"><span>1 分钟</span><span>{data.load[1].toFixed(2)} / {data.load[2].toFixed(2)}</span></div><footer>运行 {formatDuration(data.uptimeSeconds)}</footer></article>
    <article className="metric-card"><div className="metric-card__top"><span>根磁盘</span><HardDrive size={18} /></div><strong>{data.disks[0]?.usagePercent.toFixed(0) ?? "—"}<small>%</small></strong><div className="meter"><i style={{ width: `${data.disks[0]?.usagePercent ?? 0}%` }} /></div><footer>{data.disks[0] ? `${formatBytes(data.disks[0].usedBytes)} / ${formatBytes(data.disks[0].totalBytes)}` : "无数据"}</footer></article>
    <article className="system-card"><div><span className="section-kicker">SYSTEM IDENTITY</span><h2>{data.hostname}</h2><p>{data.osName} {data.osVersion}</p></div><dl><div><dt>内核</dt><dd>{data.kernel}</dd></div><div><dt>架构</dt><dd>{data.architecture}</dd></div><div><dt>采样时间</dt><dd>{new Date(data.sampledAt).toLocaleTimeString()}</dd></div></dl></article>
    <article className="runtime-card"><div className="runtime-card__header"><span className="section-kicker">RUNTIMES</span><Box size={18} /></div><div className="runtime-row"><span className={`runtime-icon ${data.docker.running ? "ok" : ""}`}>D</span><div><strong>Docker</strong><small>{data.docker.installed ? data.docker.version ?? "已安装" : "未安装"}</small></div><em>{data.docker.running ? "运行中" : data.docker.installed ? "已停止" : "不可用"}</em></div><div className="runtime-row"><span className={`runtime-icon ${data.nginx.running ? "ok" : ""}`}>N</span><div><strong>Nginx</strong><small>{data.nginx.installed ? data.nginx.version ?? "已安装" : "未安装"}</small></div><em>{data.nginx.running ? "运行中" : data.nginx.installed ? "已停止" : "不可用"}</em></div></article>
  </div>;
}
