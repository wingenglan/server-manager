import { ArrowRight, FolderKey, Network, ServerCog, ShieldCheck } from "lucide-react";
import { useState } from "react";
import { Button } from "../../components/ui/Button";
import { ServerDialog } from "./ServerDialog";

export function ServerLandingPage() {
  const [open, setOpen] = useState(false);
  return (
    <section className="landing">
      <div className="landing__eyebrow"><span /> AGENTLESS OPERATIONS</div>
      <h1>你的服务器，<br /><em>少一点距离。</em></h1>
      <p>通过安全的 SSH 与 SFTP，在一个本地桌面工作台里管理 Linux、Nginx 和 Docker。远端无需安装任何 Agent。</p>
      <div className="landing__actions"><Button variant="primary" onClick={() => setOpen(true)}>连接第一台服务器 <ArrowRight size={16} /></Button><span>凭据由操作系统安全存储保护</span></div>
      <div className="capability-strip">
        <article><Network /><span><strong>实时状态</strong><small>系统、端口与进程</small></span></article>
        <article><FolderKey /><span><strong>SFTP 文件</strong><small>浏览、编辑与传输</small></span></article>
        <article><ServerCog /><span><strong>服务管理</strong><small>systemd、Nginx、Docker</small></span></article>
        <article><ShieldCheck /><span><strong>本地优先</strong><small>Host Key 严格校验</small></span></article>
      </div>
      <ServerDialog key={open ? "landing-open" : "landing-closed"} open={open} onOpenChange={setOpen} />
    </section>
  );
}
