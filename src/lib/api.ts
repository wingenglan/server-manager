import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  ConnectionSnapshot,
  HostKeyChallenge,
  SaveServerInput,
  ServerProfile,
  ServerGroup,
  SystemOverview,
  DirectoryListing,
  RemoteTextFile,
  RemoteBinaryPreview,
  OperationsSnapshot,
  TerminationResult,
  ToolInstallPlan,
  ToolInstallResult,
  ToolStatus,
  NginxSnapshot,
  DockerActionResult,
  DockerLogs,
  DockerSnapshot,
  DockerTextResult,
  DockerPullResult,
  DockerRunResult,
  DockerResourceActionResult,
  DockerComposeDetails,
  PublicServerExport,
  PublicServerImport,
  AuditEvent,
  DiagnosticsExport,
  ServiceDetail,
  ServiceLogs,
} from "../types/server";

export const api = {
  listServers: () => invoke<ServerProfile[]>("list_servers"),
  getServer: (serverId: string) => invoke<ServerProfile>("get_server", { serverId }),
  listServerGroups: () => invoke<ServerGroup[]>("list_server_groups"),
  createServerGroup: (name: string) => invoke<ServerGroup>("create_server_group", { name }),
  saveServer: (input: SaveServerInput) => invoke<ServerProfile>("save_server", { input }),
  duplicateServer: (serverId: string) => invoke<ServerProfile>("duplicate_server", { serverId }),
  deleteServer: (serverId: string) => invoke<void>("delete_server", { serverId }),
  connectionState: (serverId: string) =>
    invoke<ConnectionSnapshot>("connection_state", { serverId }),
  connectServer: (serverId: string) =>
    invoke<ConnectionSnapshot | HostKeyChallenge>("connect_server", { serverId }),
  reconnectServer: (serverId: string) =>
    invoke<ConnectionSnapshot | HostKeyChallenge>("reconnect_server", { serverId }),
  trustHostKey: (challenge: HostKeyChallenge) =>
    invoke<ConnectionSnapshot>("trust_host_key", { challenge }),
  disconnectServer: (serverId: string) => invoke<void>("disconnect_server", { serverId }),
  overview: (serverId: string) => invoke<SystemOverview>("get_system_overview", { serverId }),
  openTerminal: (
    serverId: string,
    columns: number,
    rows: number,
    onEvent: (event: TerminalEvent) => void,
  ) => {
    const channel = new Channel<TerminalEvent>();
    channel.onmessage = onEvent;
    return invoke<string>("open_terminal", { serverId, columns, rows, onEvent: channel });
  },
  writeTerminal: (terminalId: string, data: Uint8Array) =>
    invoke<void>("write_terminal", { terminalId, data: Array.from(data) }),
  resizeTerminal: (terminalId: string, columns: number, rows: number) =>
    invoke<void>("resize_terminal", { terminalId, columns, rows }),
  closeTerminal: (terminalId: string) => invoke<void>("close_terminal", { terminalId }),
  listDirectory: (serverId: string, path: string) =>
    invoke<DirectoryListing>("list_remote_directory", { serverId, path }),
  readText: (serverId: string, path: string) =>
    invoke<RemoteTextFile>("read_remote_text", { serverId, path }),
  readImagePreview: (serverId: string, path: string) =>
    invoke<RemoteBinaryPreview>("read_remote_image_preview", { serverId, path }),
  readTail: (serverId: string, path: string, lines = 5000) =>
    invoke<RemoteTextFile>("read_remote_tail", { serverId, path, lines }),
  saveText: (input: {
    serverId: string; path: string; content: string; expectedSize: number;
    expectedModifiedAt: number | null; force?: boolean;
  }) => invoke<RemoteTextFile>("save_remote_text", { input }),
  saveTextPrivileged: (input: {
    serverId: string; path: string; content: string; expectedSize: number;
    expectedModifiedAt: number | null; force?: boolean;
  }) => invoke<RemoteTextFile>("save_remote_text_privileged", { input }),
  createEntry: (serverId: string, path: string, directory: boolean) =>
    invoke<void>("create_remote_entry", { serverId, path, directory }),
  renameEntry: (serverId: string, oldPath: string, newPath: string) =>
    invoke<void>("rename_remote_entry", { serverId, oldPath, newPath }),
  removeEntry: (serverId: string, path: string, recursive: boolean) =>
    invoke<void>("remove_remote_entry", { serverId, path, recursive }),
  chmod: (input: { serverId: string; path: string; mode: number }) =>
    invoke<void>("chmod_remote", { input }),
  createSymlink: (input: { serverId: string; targetPath: string; linkPath: string }) =>
    invoke<void>("create_remote_symlink", { input }),
  copyMove: (input: { serverId: string; sourcePath: string; destinationPath: string; operation: "copy" | "move"; recursive: boolean; confirmed: boolean }) =>
    invoke<void>("copy_move_remote", { input }),
  upload: (transferId: string, serverId: string, localPath: string, remoteDirectory: string, conflict: "replace" | "skip" | "rename", onEvent: (event: TransferEvent) => void) => {
    const channel = new Channel<TransferEvent>();
    channel.onmessage = onEvent;
    return invoke<void>("upload_remote", { transferId, serverId, localPath, remoteDirectory, conflict, onEvent: channel });
  },
  download: (transferId: string, serverId: string, remotePath: string, localDirectory: string, onEvent: (event: TransferEvent) => void) => {
    const channel = new Channel<TransferEvent>();
    channel.onmessage = onEvent;
    return invoke<void>("download_remote", { transferId, serverId, remotePath, localDirectory, onEvent: channel });
  },
  cancelTransfer: (transferId: string) => invoke<void>("cancel_transfer", { transferId }),
  operations: (serverId: string, privileged = false) => invoke<OperationsSnapshot>("get_operations", { serverId, privileged }),
  terminateProcess: (input: { serverId: string; pid: number; port?: number; force?: boolean; privileged?: boolean }) =>
    invoke<TerminationResult>("terminate_process", { input }),
  manageService: (serverId: string, service: string, action: "start" | "stop" | "restart" | "enable" | "disable") =>
    invoke<void>("manage_service", { serverId, service, action }),
  serviceDetail: (serverId: string, service: string) => invoke<ServiceDetail>("get_service_detail", { serverId, service }),
  serviceLogs: (serverId: string, service: string, lines = 200) => invoke<ServiceLogs>("get_service_logs", { serverId, service, lines }),
  // Server config import/export intentionally has no credential payload.
  exportServers: () => invoke<PublicServerExport>("export_servers"),
  importServers: (values: PublicServerImport[]) => invoke<ServerProfile[]>("import_servers", { values }),
  exportDiagnostics: () => invoke<DiagnosticsExport>("export_diagnostics"),
  listAuditEvents: (limit = 50) => invoke<AuditEvent[]>("list_audit_events", { limit }),
  exportFullBackup: (password: string) => invoke<string>("export_full_backup", { input: { password } }),
  importFullBackup: (backup: string, password: string) => invoke<ServerProfile[]>("import_full_backup", { input: { backup, password } }),
  listTools: (serverId: string) => invoke<ToolStatus[]>("list_tools", { serverId }),
  toolInstallPlan: (serverId: string, toolId: string) =>
    invoke<ToolInstallPlan>("get_tool_install_plan", { serverId, toolId }),
  /** 通过可取消 task id 流式执行用户确认的工具安装。 */
  installTool: (input: { serverId: string; toolId: string; taskId: string }, onEvent: (event: CommandEvent) => void) => {
    const channel = new Channel<CommandEvent>();
    channel.onmessage = onEvent;
    return invoke<ToolInstallResult>("install_tool", { input, onEvent: channel });
  },
  nginx: (serverId: string) => invoke<NginxSnapshot>("get_nginx", { serverId }),
  testNginx: (serverId: string) => invoke<boolean>("test_nginx_config", { serverId }),
  probeNginxBackend: (input: { serverId: string; scheme: "http" | "https"; targetHost: string; targetPort: number }) =>
    invoke<{ reachable: boolean; statusCode: number | null; latencyMs: number | null; detail: string }>("probe_nginx_backend", { input }),
  saveNginxProxy: (input: {
    serverId: string; name: string; serverName: string; listenPort: number; enableHttps: boolean; httpsPort: number; certificatePath?: string; certificateKeyPath?: string;
    location: string; upstreamScheme: "http" | "https"; targetHost: string;
    targetPort: number; websocket: boolean; preserveHost: boolean;
  }) => invoke<NginxSnapshot>("save_nginx_proxy", { input }),
  docker: (serverId: string, privileged = false) => invoke<DockerSnapshot>("get_docker", { serverId, privileged }),
  dockerContainerAction: (input: { serverId: string; containerId: string; action: string; newName?: string; force?: boolean; sudo?: boolean; confirmed?: boolean }) =>
    invoke<DockerActionResult>("docker_container_action", { input }),
  dockerContainerLogs: (serverId: string, containerId: string, tail = 200, privileged = false) =>
    invoke<DockerLogs>("docker_container_logs", { serverId, containerId, tail, privileged }),
  dockerContainerInspect: (serverId: string, containerId: string, privileged = false) =>
    invoke<DockerTextResult>("docker_container_inspect", { serverId, containerId, privileged }),
  dockerContainerStats: (serverId: string, containerId: string, privileged = false) =>
    invoke<DockerTextResult>("docker_container_stats", { serverId, containerId, privileged }),
  dockerContainerTop: (serverId: string, containerId: string, privileged = false) =>
    invoke<DockerTextResult>("docker_container_top", { serverId, containerId, privileged }),
  dockerContainerExec: (input: { serverId: string; containerId: string; command: string; sudo?: boolean }) =>
    invoke<DockerTextResult>("docker_container_exec", { input }),
  /** 通过可取消 task id 读取容器 follow 日志。 */
  dockerContainerFollowLogs: (serverId: string, containerId: string, tail: number, sudo: boolean, taskId: string, onEvent: (event: CommandEvent) => void) => {
    const channel = new Channel<CommandEvent>();
    channel.onmessage = onEvent;
    return invoke<DockerLogs>("docker_container_follow_logs", { serverId, containerId, tail, sudo, taskId, onEvent: channel });
  },
  dockerResourceAction: (input: { serverId: string; kind: "volume" | "network"; name: string; action: "create" | "remove"; sudo?: boolean; confirmed?: boolean }) =>
    invoke<DockerResourceActionResult>("docker_resource_action", { input }),
  dockerImageAction: (input: { serverId: string; image: string; action: "remove"; force?: boolean; sudo?: boolean; confirmed?: boolean }) =>
    invoke<DockerResourceActionResult>("docker_image_action", { input }),
  dockerResourceInspect: (input: { serverId: string; kind: "volume" | "network"; name: string; sudo?: boolean }) =>
    invoke<DockerTextResult>("docker_resource_inspect", { input }),
  /** 执行 Compose 生命周期或显式 cleanup，并由 Rust 校验 destructive confirmation。 */
  dockerComposeAction: (input: { serverId: string; project: string; workingDir?: string; action: "up" | "down" | "start" | "stop" | "restart" | "pull" | "build" | "cleanup"; sudo?: boolean; confirmed?: boolean }) =>
    invoke<DockerResourceActionResult>("docker_compose_action", { input }),
  /** 保存 Compose 原始 YAML；Rust 端会先 config -q，失败自动恢复。 */
  dockerComposeSaveYaml: (input: { serverId: string; project: string; workingDir?: string; configPath: string; content: string; expectedSize: number; expectedModifiedAt: number | null; force?: boolean; sudo?: boolean; confirmed: boolean }) =>
    invoke<RemoteTextFile>("docker_compose_save_yaml", { input }),
  /** 读取 Compose 服务、脱敏渲染配置和资源候选。 */
  dockerComposeDetails: (serverId: string, project: string, workingDir: string | undefined, sudo = false) =>
    invoke<DockerComposeDetails>("docker_compose_details", { serverId, project, workingDir, sudo }),
  /** 读取 Compose 项目或服务的最近日志。 */
  dockerComposeLogs: (serverId: string, project: string, workingDir: string | undefined, service: string | undefined, tail = 200, sudo = false) =>
    invoke<DockerLogs>("docker_compose_logs", { serverId, project, workingDir, service, tail, sudo }),
  /** 通过可取消 task id 流式拉取镜像层输出。 */
  dockerPullImage: (input: { serverId: string; image: string; taskId: string; sudo?: boolean }, onEvent: (event: CommandEvent) => void) => {
    const channel = new Channel<CommandEvent>();
    channel.onmessage = onEvent;
    return invoke<DockerPullResult>("docker_pull_image", { input, onEvent: channel });
  },
  /** 请求 Rust 关闭指定流式 SSH 命令的远端 channel。 */
  cancelCommandTask: (taskId: string) => invoke<void>("cancel_command_task", { taskId }),
  /** 调用受控 Run 向导；API 层固定带上用户已完成表单确认。 */
  dockerRunContainer: (input: { serverId: string; image: string; name?: string; ports: string[]; environment: string[]; network?: string; restartPolicy?: string; autoRemove: boolean; privileged: boolean; sudo?: boolean; confirmed?: boolean }) =>
    invoke<DockerRunResult>("docker_run_container", { input: { ...input, confirmed: true } }),
};

export type TerminalEvent =
  | { event: "data"; data: { data: string } }
  | { event: "exit"; data: { exitStatus: number } }
  | { event: "closed" };

export type CommandEvent =
  | { event: "output"; data: { stream: "stdout" | "stderr"; data: string } }
  | { event: "completed"; data: { exitCode: number } }
  | { event: "cancelled" };

export type TransferEvent =
  | { event: "started"; data: { transferId: string; totalBytes: number | null } }
  | { event: "progress"; data: { transferId: string; transferredBytes: number; totalBytes: number | null; bytesPerSecond: number; currentPath: string } }
  | { event: "completed"; data: { transferId: string; transferredBytes: number } }
  | { event: "cancelled"; data: { transferId: string; transferredBytes: number } };

export function isHostKeyChallenge(
  value: ConnectionSnapshot | HostKeyChallenge,
): value is HostKeyChallenge {
  return "fingerprint" in value;
}
