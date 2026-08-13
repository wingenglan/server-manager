import { create } from "zustand";

export type CommandTaskType = "tool-install" | "docker-pull" | "docker-follow" | "docker-compose";
export type TaskStatus = "queued" | "running" | "success" | "failed" | "cancelled";

export interface CommandTask {
  id: string;
  type: CommandTaskType;
  serverId: string;
  title: string;
  status: TaskStatus;
  progress: number | null;
  bytesTransferred: number;
  totalBytes: number | null;
  startedAt: number;
  finishedAt?: number;
  error?: string;
  cancelSupported: boolean;
}

interface CommandTaskState {
  tasks: CommandTask[];
  add: (task: Omit<CommandTask, "startedAt" | "cancelSupported" | "progress" | "bytesTransferred" | "totalBytes"> & { cancelSupported?: boolean; progress?: number | null; bytesTransferred?: number; totalBytes?: number | null }) => void;
  running: (id: string) => void;
  success: (id: string) => void;
  fail: (id: string, error: string) => void;
  cancelled: (id: string) => void;
  clearFinished: () => void;
}

/** 维护跨页面可见的远程命令任务状态；仅保存元数据，不保存命令输出或机密内容。 */
export const useCommandTaskStore = create<CommandTaskState>((set) => ({
  tasks: [],
  /** 注册一个跨页面可见的命令任务，只保留任务元数据。 */
  add: (task) => set((state) => ({
    tasks: [
      { ...task, startedAt: Date.now(), cancelSupported: task.cancelSupported ?? true, progress: task.progress ?? null, bytesTransferred: task.bytesTransferred ?? 0, totalBytes: task.totalBytes ?? null },
      ...state.tasks.filter((current) => current.id !== task.id),
    ],
  })),
  /** 将已产生远程输出的任务标为执行中。 */
  running: (id) => set((state) => ({ tasks: state.tasks.map((task) => task.id === id ? { ...task, status: "running" } : task) })),
  /** 将远程命令标记为成功并记录完成时间。 */
  success: (id) => set((state) => ({ tasks: state.tasks.map((task) => task.id === id ? { ...task, status: "success", finishedAt: Date.now() } : task) })),
  /** 将远程命令标记为失败并保存脱敏错误文本。 */
  fail: (id, error) => set((state) => ({ tasks: state.tasks.map((task) => task.id === id ? { ...task, status: "failed", finishedAt: Date.now(), error } : task) })),
  /** 将用户取消的远程命令标记为取消完成。 */
  cancelled: (id) => set((state) => ({ tasks: state.tasks.map((task) => task.id === id ? { ...task, status: "cancelled", finishedAt: Date.now() } : task) })),
  /** 清除终态任务，保留当前仍在执行的任务。 */
  clearFinished: () => set((state) => ({ tasks: state.tasks.filter((task) => task.status === "queued" || task.status === "running") })),
}));
