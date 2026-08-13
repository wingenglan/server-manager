import { Check, CircleX, LoaderCircle, RefreshCw, Trash2, X } from "lucide-react";
import { api } from "../../lib/api";
import { formatBytes } from "../../lib/format";
import { useTransferStore } from "../files/transferStore";
import { useCommandTaskStore, type CommandTask } from "./taskStore";

/** 展示所有远程命令与文件传输任务，并提供统一取消和清理入口。 */
export function TaskCenter({ open, onClose }: { open: boolean; onClose: () => void }) {
  const commandTasks = useCommandTaskStore((state) => state.tasks);
  const transferTasks = useTransferStore((state) => state.tasks);
  const clearCommands = useCommandTaskStore((state) => state.clearFinished);
  const clearTransfers = useTransferStore((state) => state.clearFinished);
  if (!open) return null;
  const activeCount = commandTasks.filter(isActive).length + transferTasks.filter((task) => task.status === "queued" || task.status === "running").length;
  return <aside className="task-center" aria-label="任务中心">
    <header><div><strong>任务中心</strong><small>{activeCount} 个活动任务</small></div><button onClick={onClose} aria-label="关闭"><X size={15} /></button></header>
    <div className="task-list">
      {!commandTasks.length && !transferTasks.length && <div className="task-empty">尚无远程任务</div>}
      {commandTasks.map((task) => <CommandTaskRow key={task.id} task={task} />)}
      {transferTasks.map((task) => <article className="task-row" key={task.id}><div className="task-row__icon"><LoaderCircle size={15} className={isActive(task) ? "spin" : ""} /></div><div><strong>{task.label}</strong><small>{task.error ?? task.currentPath}</small><footer><span>{formatBytes(task.transferredBytes)}{task.totalBytes ? ` / ${formatBytes(task.totalBytes)}` : ""}</span><span>{task.status === "running" ? `${formatBytes(task.bytesPerSecond)}/s` : statusLabel(task.status)}</span></footer></div><span className={`task-status is-${task.status}`}>{task.status === "running" || task.status === "queued" ? <button title="取消传输" onClick={() => void api.cancelTransfer(task.id)}><CircleX size={15} /></button> : task.status === "success" ? <Check size={15} /> : task.retry ? <button title="重试传输" onClick={task.retry}><RefreshCw size={15} /></button> : task.status === "failed" ? <CircleX size={15} /> : <LoaderCircle size={15} />}</span></article>)}
    </div>
    <footer><button onClick={() => { clearCommands(); clearTransfers(); }}><Trash2 size={13} /> 清除已完成</button></footer>
  </aside>;
}

/** 渲染单个可取消的远程命令任务。 */
function CommandTaskRow({ task }: { task: CommandTask }) {
  const cancel = () => { if (task.cancelSupported) void api.cancelCommandTask(task.id); };
  const progress = task.progress === null ? null : `${Math.round(task.progress)}%`;
  return <article className="task-row"><div className="task-row__icon"><LoaderCircle size={15} className={isActive(task) ? "spin" : ""} /></div><div><strong>{task.title}</strong><small>{task.error ?? task.serverId}</small>{progress && <div className="task-progress"><i style={{ width: `${Math.max(0, Math.min(100, task.progress ?? 0))}%` }} /></div>}<footer><span>{typeLabel(task.type)}{task.totalBytes ? ` · ${task.bytesTransferred}/${task.totalBytes} B` : ""}</span><span>{progress ?? statusLabel(task.status)}</span></footer></div><span className={`task-status is-${task.status}`}>{isActive(task) && task.cancelSupported ? <button title="取消远程任务" onClick={cancel}><CircleX size={15} /></button> : task.status === "success" ? <Check size={15} /> : task.status === "failed" ? <CircleX size={15} /> : <LoaderCircle size={15} />}</span></article>;
}

/** 判断任务是否仍在队列或执行中。 */
function isActive(task: { status: string }) { return task.status === "queued" || task.status === "running"; }

/** 将任务类型转换为用户可读的中文标签。 */
function typeLabel(type: CommandTask["type"]) { return { "tool-install": "工具安装", "docker-pull": "Docker Pull", "docker-follow": "Docker Follow", "docker-compose": "Docker Compose" }[type]; }

/** 将统一任务状态转换为用户可读文本。 */
function statusLabel(status: string) { return { queued: "等待中", running: "执行中", success: "已完成", failed: "失败", cancelled: "已取消" }[status] ?? status; }
