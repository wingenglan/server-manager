import { useMutation, useQuery } from "@tanstack/react-query";
import { Bell, Boxes, ChevronRight, Command, Plus, Search, Settings2, Server, Waypoints } from "lucide-react";
import { useEffect, useState } from "react";
import { NavLink, Outlet, useNavigate } from "react-router-dom";
import { Button } from "../components/ui/Button";
import { NoticeHost } from "../components/ui/NoticeHost";
import { ServerDialog } from "../features/servers/ServerDialog";
import { useTransferStore } from "../features/files/transferStore";
import { TaskCenter } from "../features/tasks/TaskCenter";
import { useCommandTaskStore } from "../features/tasks/taskStore";
import { api } from "../lib/api";
import { applyLocale, connectionStatusLabel, readLocale } from "../lib/i18n";
import type { ServerProfile } from "../types/server";

/** 渲染全局导航、服务器列表、命令面板和传输中心。 */
export function AppShell() {
  const navigate = useNavigate();
  const [addOpen, setAddOpen] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [paletteQuery, setPaletteQuery] = useState("");
  const [tasksOpen, setTasksOpen] = useState(false);
  const [sidebarQuery, setSidebarQuery] = useState("");
  const transferTasks = useTransferStore((state) => state.tasks);
  const activeTransfers = transferTasks.filter((task) => task.status === "queued" || task.status === "running").length;
  const commandTasks = useCommandTaskStore((state) => state.tasks);
  const activeTasks = activeTransfers + commandTasks.filter((task) => task.status === "queued" || task.status === "running").length;
  const servers = useQuery({ queryKey: ["servers"], queryFn: api.listServers });
  const groups = useQuery({ queryKey: ["server-groups"], queryFn: api.listServerGroups });
  const createGroup = useMutation({ mutationFn: api.createServerGroup, onSuccess: () => groups.refetch() });
  const visibleServers = (servers.data ?? []).filter((server) => `${server.name} ${server.host} ${server.username}`.toLocaleLowerCase().includes(sidebarQuery.trim().toLocaleLowerCase()));

  useEffect(() => { applyLocale(readLocale()); }, []);

  useEffect(() => {
    void api.listTasks().then((records) => useCommandTaskStore.getState().hydrate(records)).catch(() => undefined);
  }, []);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setPaletteOpen((value) => !value);
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  return (
    <div className="app-frame">
      <header className="topbar">
        <button className="brand" onClick={() => navigate("/")} aria-label="返回服务器列表">
          <span className="brand__mark"><Boxes size={18} strokeWidth={1.8} /></span>
          <span><strong>Relay</strong><small>服务器运维</small></span>
        </button>
        <button className="command-trigger" onClick={() => setPaletteOpen(true)}>
          <Search size={15} />
          <span>搜索服务器、端口或命令</span>
          <kbd><Command size={11} /> K</kbd>
        </button>
        <div className="topbar__actions">
          <button className="icon-control topbar-task-button" aria-label="任务与通知" onClick={() => setTasksOpen((value) => !value)}><Bell size={17} />{activeTasks > 0 && <i>{activeTasks}</i>}</button>
          <Button variant="primary" size="sm" onClick={() => setAddOpen(true)}>
            <Plus size={15} /> 添加服务器
          </Button>
        </div>
      </header>

      <aside className="sidebar">
        <div className="sidebar__heading"><span>服务器</span><span>{visibleServers.length} / {servers.data?.length ?? 0}</span></div><div className="sidebar-search"><Search size={13} /><input value={sidebarQuery} onChange={(event) => setSidebarQuery(event.target.value)} placeholder="筛选服务器" /><button aria-label="新建服务器分组" onClick={() => { const name = window.prompt("分组名称"); if (name?.trim()) createGroup.mutate(name.trim()); }}>+</button></div>
        <nav className="server-nav">
          {servers.isLoading && <div className="nav-skeleton" />}
          {visibleServers.map((server) => (
            <ServerLink key={server.id} serverId={server.id} name={server.name} address={`${server.username}@${server.host}`} groupName={groups.data?.find((group) => group.id === server.groupId)?.name} />
          ))}
          {!servers.isLoading && !servers.data?.length && (
            <button className="sidebar-empty" onClick={() => setAddOpen(true)}>
              <Server size={20} /><span>尚无服务器<small>添加第一个 SSH 连接</small></span>
            </button>
          )}
        </nav>
        <div className="sidebar__footer">
          <NavLink to="/settings"><Settings2 size={16} /> 设置</NavLink>
          <div className="local-first"><span /> 本地优先 · 凭据受保护</div>
        </div>
      </aside>

      <main className="workspace"><Outlet /></main>
      <footer className="statusbar">
        <span><i className="statusbar__dot" /> 本机服务就绪</span>
        <button className="statusbar-transfer" onClick={() => setTasksOpen(true)}><Waypoints size={11} /> 任务 {activeTasks} · 共 {transferTasks.length + commandTasks.length}</button>
        <span>Relay 0.3.0</span>
      </footer>

      <ServerDialog key={addOpen ? "add-open" : "add-closed"} open={addOpen} onOpenChange={setAddOpen} />
      {paletteOpen && (
        <div className="palette-backdrop" onMouseDown={() => setPaletteOpen(false)}>
          <div className="palette" onMouseDown={(event) => event.stopPropagation()}>
            <div className="palette__input"><Search size={18} /><input autoFocus value={paletteQuery} onChange={(event) => setPaletteQuery(event.target.value)} placeholder="输入服务器名称，或尝试“打开文件 生产”" /></div>
            <PaletteResults query={paletteQuery} servers={servers.data ?? []} onNavigate={(path) => { setPaletteOpen(false); setPaletteQuery(""); navigate(path); }} />
          </div>
        </div>
      )}
      <TaskCenter open={tasksOpen} onClose={() => setTasksOpen(false)} />
      <NoticeHost />
    </div>
  );
}

/** 将命令面板输入解析为真实的服务器工作区导航命令。 */
function PaletteResults({ query, servers, onNavigate }: { query: string; servers: ServerProfile[]; onNavigate: (path: string) => void }) {
  const input = query.trim().toLocaleLowerCase();
  const mode = input === "nginx" || input.startsWith("nginx ") || input === "反向代理" || input.startsWith("反向代理 ") ? "nginx" : input === "docker" || input.startsWith("docker ") || input === "容器" || input.startsWith("容器 ") ? "docker" : input === "tools" || input.startsWith("tools ") || input === "工具" || input.startsWith("工具 ") ? "tools" : input === "terminal" || input.startsWith("terminal ") || input === "终端" || input.startsWith("终端 ") ? "terminal" : input === "logs" || input.startsWith("logs ") || input === "日志" || input.startsWith("日志 ") ? "logs" : input.startsWith("open files ") || input.startsWith("打开文件 ") ? "files" : input === "port" || input.startsWith("operations ") || input.startsWith("ports ") || input.startsWith("processes ") || input.startsWith("services ") || input.startsWith("port ") || input === "端口" || input.startsWith("端口 ") || input === "进程" || input.startsWith("进程 ") ? "operations" : "overview";
  const needle = input.replace(/^(open files|打开文件|terminal|终端|nginx|反向代理|tools|工具|docker|容器|logs|日志|operations|ports|processes|services|port|端口|进程)\s+/, "");
  const matches = servers.filter((server) => `${server.name} ${server.host} ${server.username}`.toLocaleLowerCase().includes(needle));
  if (!matches.length) return <div className="palette__hint">没有匹配的服务器。端口、进程和服务搜索请在对应服务器工作区中执行。</div>;
  return <div className="palette-results">{matches.slice(0, 8).map((server) => <button key={server.id} onClick={() => onNavigate(`/servers/${server.id}${mode === "overview" ? "" : `/${mode}`}`)}><span><strong>{mode === "overview" ? "打开概览" : `打开${mode === "files" ? "文件" : mode === "terminal" ? "终端" : mode === "nginx" ? "Nginx" : mode === "tools" ? "工具" : mode === "docker" ? "Docker" : mode === "logs" ? "日志" : "端口与进程"}`}</strong><small>{server.name} · {server.host}</small></span><ChevronRight size={14} /></button>)}</div>;
}

/** 显示服务器档案，并读取本地 SSH 会话快照更新在线/错误状态。 */
function ServerLink({ serverId, name, address, groupName }: { serverId: string; name: string; address: string; groupName?: string }) {
  const connection = useQuery({ queryKey: ["connection", serverId], queryFn: () => api.connectionState(serverId), refetchInterval: 5_000 });
  const state = connection.data?.status ?? "offline";
  return <NavLink to={`/servers/${serverId}`} className="server-link">
    <span className={`server-link__status is-${state}`} title={connection.data?.error?.message ?? connectionStatusLabel(state)} />
    <span className="server-link__body"><strong>{name}</strong><small>{groupName ? `${groupName} · ` : ""}{address}</small></span>
    <ChevronRight size={14} />
  </NavLink>;
}
