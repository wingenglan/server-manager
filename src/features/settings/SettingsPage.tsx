import { LockKeyhole, Moon, RotateCcw } from "lucide-react";

export function SettingsPage() {
  return <section className="settings-page"><div className="workspace-header"><div><div className="breadcrumb">应用</div><h1>设置</h1><p>本地偏好与安全策略</p></div></div><div className="settings-list"><article><Moon /><div><strong>外观</strong><p>跟随系统主题；深色模式将随系统自动切换。</p></div><select defaultValue="system"><option value="system">跟随系统</option><option value="dark">深色</option><option value="light">浅色</option></select></article><article><LockKeyhole /><div><strong>凭据</strong><p>SSH 与 sudo 密码存储在操作系统安全存储，不写入数据库。</p></div><span className="settings-badge">已保护</span></article><article><RotateCcw /><div><strong>连接恢复</strong><p>启动时恢复上次打开的工作区，但不自动重放危险任务。</p></div><input type="checkbox" defaultChecked /></article></div></section>;
}
