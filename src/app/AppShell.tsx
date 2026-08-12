import { useQuery } from "@tanstack/react-query";
import { Bell, Boxes, ChevronRight, Command, Plus, Search, Settings2, Server, Waypoints } from "lucide-react";
import { useEffect, useState } from "react";
import { NavLink, Outlet, useNavigate } from "react-router-dom";
import { Button } from "../components/ui/Button";
import { ServerDialog } from "../features/servers/ServerDialog";
import { TransferCenter } from "../features/files/TransferCenter";
import { useTransferStore } from "../features/files/transferStore";
import { api } from "../lib/api";

export function AppShell() {
  const navigate = useNavigate();
  const [addOpen, setAddOpen] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [transfersOpen, setTransfersOpen] = useState(false);
  const transferTasks = useTransferStore((state) => state.tasks);
  const activeTransfers = transferTasks.filter((task) => task.status === "queued" || task.status === "running").length;
  const servers = useQuery({ queryKey: ["servers"], queryFn: api.listServers });

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
          <span><strong>Relay</strong><small>SERVER OPERATIONS</small></span>
        </button>
        <button className="command-trigger" onClick={() => setPaletteOpen(true)}>
          <Search size={15} />
          <span>搜索服务器、端口或命令</span>
          <kbd><Command size={11} /> K</kbd>
        </button>
        <div className="topbar__actions">
          <button className="icon-control" aria-label="任务与通知"><Bell size={17} /></button>
          <Button variant="primary" size="sm" onClick={() => setAddOpen(true)}>
            <Plus size={15} /> 添加服务器
          </Button>
        </div>
      </header>

      <aside className="sidebar">
        <div className="sidebar__heading"><span>服务器</span><span>{servers.data?.length ?? 0}</span></div>
        <nav className="server-nav">
          {servers.isLoading && <div className="nav-skeleton" />}
          {servers.data?.map((server) => (
            <NavLink key={server.id} to={`/servers/${server.id}`} className="server-link">
              <span className="server-link__status" />
              <span className="server-link__body">
                <strong>{server.name}</strong>
                <small>{server.username}@{server.host}</small>
              </span>
              <ChevronRight size={14} />
            </NavLink>
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
        <button className="statusbar-transfer" onClick={() => setTransfersOpen((value) => !value)}><Waypoints size={11} /> 传输 {activeTransfers} · 共 {transferTasks.length}</button>
        <span>Relay 0.1.0</span>
      </footer>

      <ServerDialog key={addOpen ? "add-open" : "add-closed"} open={addOpen} onOpenChange={setAddOpen} />
      {paletteOpen && (
        <div className="palette-backdrop" onMouseDown={() => setPaletteOpen(false)}>
          <div className="palette" onMouseDown={(event) => event.stopPropagation()}>
            <div className="palette__input"><Search size={18} /><input autoFocus placeholder="输入服务器名称，或尝试 “port 8080”" /></div>
            <div className="palette__hint">命令面板将在服务器连接后搜索进程、端口和服务。</div>
          </div>
        </div>
      )}
      <TransferCenter open={transfersOpen} onClose={() => setTransfersOpen(false)} />
    </div>
  );
}
