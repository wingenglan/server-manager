import { FitAddon } from "@xterm/addon-fit";
import { SearchAddon } from "@xterm/addon-search";
import { Terminal } from "@xterm/xterm";
import { useQuery } from "@tanstack/react-query";
import { Eraser, Minus, Pencil, Plus, RotateCw, Search, TerminalSquare, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { NavLink, useParams } from "react-router-dom";
import { Button } from "../../components/ui/Button";
import { api, type TerminalEvent } from "../../lib/api";
import { errorMessage } from "../../lib/errors";

interface TerminalTab {
  key: string;
  title: string;
  revision: number;
}

function newTab(index: number): TerminalTab {
  return { key: crypto.randomUUID(), title: `Shell ${index}`, revision: 0 };
}

function decodeBase64(value: string): Uint8Array {
  const binary = atob(value);
  return Uint8Array.from(binary, (character) => character.charCodeAt(0));
}

/** 管理基于 SSH 的多标签交互终端，并同步远端终端事件。 */
export function TerminalPage() {
  const { serverId = "" } = useParams();
  const profile = useQuery({ queryKey: ["server", serverId], queryFn: () => api.getServer(serverId) });
  const connection = useQuery({ queryKey: ["connection", serverId], queryFn: () => api.connectionState(serverId), refetchInterval: 3000 });
  const [tabs, setTabs] = useState<TerminalTab[]>(() => [newTab(1)]);
  const [activeKey, setActiveKey] = useState(() => tabs[0].key);
  const [editingKey, setEditingKey] = useState<string | null>(null);
  const [fontSize, setFontSize] = useState(13);
  const [searchOpen, setSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [searchRequest, setSearchRequest] = useState(0);
  const [clearRequest, setClearRequest] = useState(0);

  const addTab = () => {
    const tab = newTab(tabs.length + 1);
    setTabs((current) => [...current, tab]);
    setActiveKey(tab.key);
  };
  const closeTab = (key: string) => {
    setTabs((current) => {
      const index = current.findIndex((tab) => tab.key === key);
      const next = current.filter((tab) => tab.key !== key);
      if (!next.length) {
        const replacement = newTab(1);
        setActiveKey(replacement.key);
        return [replacement];
      }
      if (activeKey === key) setActiveKey(next[Math.min(index, next.length - 1)].key);
      return next;
    });
  };
  const renameTab = (key: string, title: string) => {
    const value = title.trim();
    if (value) setTabs((current) => current.map((tab) => tab.key === key ? { ...tab, title: value } : tab));
    setEditingKey(null);
  };
  const reopenTab = (key: string) => {
    setTabs((current) => current.map((tab) => tab.key === key ? { ...tab, revision: tab.revision + 1 } : tab));
  };

  return <section className="terminal-page">
    <div className="workspace-header terminal-header">
      <div><div className="breadcrumb">服务器 / {profile.data?.name ?? "…"} / <span>终端</span></div><h1>SSH 终端</h1><p>{profile.data ? `${profile.data.username}@${profile.data.host}` : "正在载入"}</p></div>
      <div className="workspace-header__actions"><span className={`connection-pill ${connection.data?.status === "online" ? "is-online" : ""}`}><i /> {connection.data?.status === "online" ? `${tabs.length} 个交互会话` : "SSH 已断开"}</span></div>
    </div>
    <nav className="workspace-tabs"><NavLink end to={`/servers/${serverId}`}>概览</NavLink><NavLink to={`/servers/${serverId}/files`}>文件</NavLink><NavLink className="active" to={`/servers/${serverId}/terminal`}>终端</NavLink><NavLink to={`/servers/${serverId}/operations`}>端口与进程</NavLink><NavLink to={`/servers/${serverId}/services`}>服务</NavLink><NavLink to={`/servers/${serverId}/tools`}>工具</NavLink><NavLink to={`/servers/${serverId}/nginx`}>Nginx</NavLink><NavLink to={`/servers/${serverId}/docker`}>Docker</NavLink></nav>
    <div className="terminal-workspace">
      <div className="terminal-tabbar">
        <div className="terminal-tabs">
          {tabs.map((tab) => <button key={tab.key} className={`terminal-tab ${activeKey === tab.key ? "active" : ""}`} onClick={() => setActiveKey(tab.key)} onDoubleClick={() => setEditingKey(tab.key)}>
            <TerminalSquare size={13} />
            {editingKey === tab.key ? <input autoFocus defaultValue={tab.title} onClick={(event) => event.stopPropagation()} onBlur={(event) => renameTab(tab.key, event.target.value)} onKeyDown={(event) => { if (event.key === "Enter") renameTab(tab.key, event.currentTarget.value); if (event.key === "Escape") setEditingKey(null); }} /> : <span>{tab.title}</span>}
            <Pencil className="terminal-rename" size={10} />
            <X size={12} onClick={(event) => { event.stopPropagation(); closeTab(tab.key); }} />
          </button>)}
        </div>
        <button className="terminal-add" title="新建终端" onClick={addTab}><Plus size={14} /></button>
        {searchOpen && <form className="terminal-search" onSubmit={(event) => { event.preventDefault(); setSearchRequest((value) => value + 1); }}><Search size={12} /><input autoFocus value={searchQuery} onChange={(event) => setSearchQuery(event.target.value)} placeholder="搜索输出" /><button type="button" onClick={() => setSearchOpen(false)}><X size={12} /></button></form>}
        <div className="terminal-tools"><button title="搜索终端输出" onClick={() => setSearchOpen((value) => !value)}><Search size={14} /></button><button title="清屏" onClick={() => setClearRequest((value) => value + 1)}><Eraser size={14} /></button><button onClick={() => setFontSize((value) => Math.max(9, value - 1))} title="缩小字体"><Minus size={14} /></button><span>{fontSize}px</span><button onClick={() => setFontSize((value) => Math.min(24, value + 1))} title="放大字体"><Plus size={14} /></button></div>
      </div>
      {connection.data?.status !== "online" && <div className="terminal-blocked">服务器连接已断开。<NavLink to={`/servers/${serverId}`}>返回概览重新连接</NavLink></div>}
      {tabs.map((tab) => <SessionTerminal key={`${tab.key}:${tab.revision}`} serverId={serverId} active={activeKey === tab.key} fontSize={fontSize} searchQuery={searchQuery} searchRequest={activeKey === tab.key ? searchRequest : 0} clearRequest={activeKey === tab.key ? clearRequest : 0} onReconnect={() => reopenTab(tab.key)} />)}
    </div>
  </section>;
}

interface SessionProps {
  serverId: string;
  active: boolean;
  fontSize: number;
  searchQuery: string;
  searchRequest: number;
  clearRequest: number;
  onReconnect: () => void;
}

function SessionTerminal({ serverId, active, fontSize, searchQuery, searchRequest, clearRequest, onReconnect }: SessionProps) {
  const hostRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const searchRef = useRef<SearchAddon | null>(null);
  const terminalIdRef = useRef<string | null>(null);
  const activeRef = useRef(active);
  const [status, setStatus] = useState<"opening" | "online" | "closed" | "error">("opening");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    activeRef.current = active;
  }, [active]);

  useEffect(() => {
    if (!hostRef.current) return;
    let disposed = false;
    const terminal = new Terminal({
      cursorBlink: true, cursorStyle: "bar", fontFamily: '"Cascadia Mono", "JetBrains Mono", monospace',
      fontSize: 13, lineHeight: 1.25, scrollback: 10_000, allowProposedApi: false,
      theme: { background: "#090d0e", foreground: "#d9e2df", cursor: "#c7f36b", selectionBackground: "#40502b", black: "#111718", brightBlack: "#65716e", green: "#b7dc62", brightGreen: "#c7f36b", red: "#ff735c", brightRed: "#ff9180", yellow: "#e8b85e", brightYellow: "#f3ca7c", blue: "#6fa8dc", brightBlue: "#8ebfe9", magenta: "#b28ad5", brightMagenta: "#c8a4e6", cyan: "#66b8ad", brightCyan: "#82d0c5", white: "#c8d0ce", brightWhite: "#f2f6f4" },
    });
    const fit = new FitAddon();
    const search = new SearchAddon();
    terminal.loadAddon(fit);
    terminal.loadAddon(search);
    terminal.open(hostRef.current);
    terminalRef.current = terminal;
    fitRef.current = fit;
    searchRef.current = search;
    fit.fit();

    const handleEvent = (event: TerminalEvent) => {
      if (disposed) return;
      if (event.event === "data") terminal.write(decodeBase64(event.data.data));
      if (event.event === "exit") terminal.writeln(`\r\n\x1b[33m[进程退出: ${event.data.exitStatus}]\x1b[0m`);
      if (event.event === "closed") { setStatus("closed"); terminal.writeln("\r\n\x1b[31m[SSH 终端已断开]\x1b[0m"); }
    };
    api.openTerminal(serverId, terminal.cols, terminal.rows, handleEvent).then((terminalId) => {
      if (disposed) { void api.closeTerminal(terminalId); return; }
      terminalIdRef.current = terminalId;
      setStatus("online");
      if (activeRef.current) terminal.focus();
    }).catch((reason) => { if (!disposed) { setStatus("error"); setError(errorMessage(reason)); } });

    const dataDisposable = terminal.onData((data) => {
      if ((data.includes("\n") || data.includes("\r")) && data.length > 2 && !window.confirm("即将粘贴多行内容到远程终端，确定继续吗？")) return;
      if (terminalIdRef.current) void api.writeTerminal(terminalIdRef.current, new TextEncoder().encode(data)).catch((reason) => setError(errorMessage(reason)));
    });
    terminal.attachCustomKeyEventHandler((event) => {
      if (event.type !== "keydown" || !event.ctrlKey || !event.shiftKey) return true;
      if (event.key.toLowerCase() === "c" && terminal.hasSelection()) { void navigator.clipboard.writeText(terminal.getSelection()); return false; }
      if (event.key.toLowerCase() === "v") { void navigator.clipboard.readText().then((value) => terminal.paste(value)); return false; }
      return true;
    });
    const resizeObserver = new ResizeObserver(() => {
      if (!activeRef.current) return;
      fit.fit();
      if (terminalIdRef.current) void api.resizeTerminal(terminalIdRef.current, terminal.cols, terminal.rows);
    });
    resizeObserver.observe(hostRef.current);
    return () => {
      disposed = true;
      resizeObserver.disconnect();
      dataDisposable.dispose();
      terminal.dispose();
      terminalRef.current = null;
      fitRef.current = null;
      searchRef.current = null;
      if (terminalIdRef.current) void api.closeTerminal(terminalIdRef.current);
      terminalIdRef.current = null;
    };
  }, [serverId]);

  useEffect(() => { if (terminalRef.current) terminalRef.current.options.fontSize = fontSize; fitRef.current?.fit(); }, [fontSize]);
  useEffect(() => { if (active) { requestAnimationFrame(() => { fitRef.current?.fit(); terminalRef.current?.focus(); }); } }, [active]);
  useEffect(() => { if (searchRequest && searchQuery) searchRef.current?.findNext(searchQuery, { caseSensitive: false, incremental: false }); }, [searchQuery, searchRequest]);
  useEffect(() => { if (clearRequest) terminalRef.current?.clear(); }, [clearRequest]);

  return <div className={`terminal-session ${active ? "is-active" : ""}`}><div ref={hostRef} className="terminal-host" aria-label="SSH 交互终端" />{status === "opening" && <div className="terminal-session-state">正在创建 PTY…</div>}{error && <div className="terminal-error">{error}<Button size="sm" onClick={onReconnect}><RotateCw size={13} /> 重开会话</Button></div>}{status === "closed" && !error && <div className="terminal-error">会话已关闭<Button size="sm" onClick={onReconnect}><RotateCw size={13} /> 重开会话</Button></div>}</div>;
}
