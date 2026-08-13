export type AuthType = "password" | "private_key" | "ssh_agent";
export type SudoMode = "none" | "passwordless" | "password";

export interface ServerProfile {
  id: string;
  name: string;
  description: string;
  host: string;
  port: number;
  username: string;
  authType: AuthType;
  privateKeyPath: string | null;
  sudoMode: SudoMode;
  groupId: string | null;
  tags: string[];
  favorite: boolean;
  connectTimeout: number;
  keepalive: number;
  encoding: string;
  lastConnectedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface SaveServerInput {
  id?: string;
  name: string;
  description: string;
  host: string;
  port: number;
  username: string;
  authType: AuthType;
  password?: string;
  privateKeyPath?: string;
  privateKeyPassphrase?: string;
  sudoMode: SudoMode;
  sudoPassword?: string;
  groupId?: string;
  connectTimeout?: number;
  keepalive?: number;
  encoding?: string;
  tags: string[];
  favorite: boolean;
}

export interface ServerGroup {
  id: string;
  name: string;
  sortOrder: number;
  createdAt: string;
}

export interface PublicServerProfile {
  name: string;
  description: string;
  host: string;
  port: number;
  username: string;
  authType: AuthType;
  privateKeyPath: string | null;
  sudoMode: SudoMode;
  groupId: string | null;
  tags: string[];
  favorite: boolean;
  connectTimeout: number;
  keepalive: number;
  encoding: string;
}

export type PublicServerImport = Omit<PublicServerProfile, "sudoMode" | "groupId"> & { sudoMode?: SudoMode; groupId?: string | null };

export interface PublicServerExport {
  format: "agentless-server-manager-backup";
  version: number;
  encrypted: false;
  servers: PublicServerProfile[];
}

export interface HostKeyChallenge {
  serverId: string;
  host: string;
  port: number;
  keyType: string;
  fingerprint: string;
}

export interface ConnectionSnapshot {
  serverId: string;
  status: "offline" | "connecting" | "online" | "error";
  connectedAt: string | null;
  error: AppError | null;
}

export interface SystemOverview {
  hostname: string;
  osName: string;
  osVersion: string;
  kernel: string;
  architecture: string;
  currentUser: string;
  currentTime: string;
  timezone: string;
  primaryIp: string;
  defaultGateway: string;
  packageManager: string;
  systemdRunning: boolean;
  uptimeSeconds: number;
  cpuModel: string;
  logicalCores: number;
  cpuUsagePercent: number | null;
  load: [number, number, number];
  memoryTotalBytes: number;
  memoryAvailableBytes: number;
  swapTotalBytes: number;
  swapFreeBytes: number;
  networkRxBytesPerSecond: number;
  networkTxBytesPerSecond: number;
  failedServices: number;
  listeningPorts: number;
  disks: Array<{ mount: string; totalBytes: number; usedBytes: number; usagePercent: number }>;
  topProcesses: Array<{ pid: number; name: string; cpuPercent: number; memoryPercent: number; command: string }>;
  mounts: Array<{ mount: string; source: string; filesystem: string; options: string }>;
  docker: { installed: boolean; running: boolean; version: string | null };
  nginx: { installed: boolean; running: boolean; version: string | null };
  capabilities: Record<string, boolean>;
  serverCapabilities: { adapter: string; packageManager: string; serviceManager: string; firewall: string | null; commandPaths: Record<string, string>; dockerCommand: string | null; nginxCommand: string | null };
  sampledAt: string;
}

export interface AppError {
  code: string;
  category: string;
  message: string;
  details?: string | null;
  serverId?: string | null;
  recoverable: boolean;
  suggestedAction?: string | null;
}

export type RemoteFileKind = "directory" | "file" | "symlink" | "other";

export interface RemoteFileEntry {
  name: string;
  path: string;
  kind: RemoteFileKind;
  size: number;
  permissions: string;
  owner: string;
  group: string;
  modifiedAt: number | null;
}

export interface DirectoryListing {
  path: string;
  entries: RemoteFileEntry[];
}

export interface RemoteTextFile {
  path: string;
  content: string;
  size: number;
  modifiedAt: number | null;
  permissions: number | null;
}

export interface OperationsSnapshot {
  processes: Array<{ pid: number; ppid: number; user: string; state: string; cpuPercent: number; memoryPercent: number; rssBytes: number; elapsedSeconds: number; name: string; command: string; systemdUnit: string | null }>;
  ports: Array<{ protocol: string; localAddress: string; port: number; pid: number | null; processName: string | null; ipv6: boolean; processVisible: boolean }>;
  portsSource: string;
  portsWarning: string | null;
  services: Array<{ name: string; load: string; active: string; sub: string; description: string }>;
}

export type ShortcutScope = "global" | "server";

export interface ShortcutRecord {
  id: string;
  scope: ShortcutScope;
  serverId: string | null;
  name: string;
  commandTemplate: string;
  description: string;
  tags: string[];
  enabled: boolean;
  builtin: boolean;
  usageCount: number;
  createdAt: string;
  updatedAt: string;
}

export interface SaveShortcutInput {
  id?: string;
  scope: ShortcutScope;
  serverId?: string;
  name: string;
  commandTemplate: string;
  description: string;
  tags: string[];
  enabled: boolean;
}

export type LogSource = "system" | "systemd" | "nginx-access" | "nginx-error" | "docker" | "docker-compose";

export interface LogQuery {
  serverId: string;
  source: LogSource;
  target?: string;
  workingDir?: string;
  service?: string;
  tail: number;
  privileged: boolean;
}

export interface LogSnapshot {
  source: LogSource;
  target: string | null;
  output: string;
  fetchedAt: string;
  truncated: boolean;
}

export interface MetricSample {
  sampledAt: string;
  cpuUsagePercent: number | null;
  memoryUsedBytes: number;
  memoryTotalBytes: number;
  loadOne: number;
  networkRxBytesPerSecond: number;
  networkTxBytesPerSecond: number;
  diskUsagePercent: number | null;
}

export type PersistedTaskStatus = "queued" | "running" | "success" | "failed" | "cancelled" | "interrupted";

export interface TaskRecord {
  id: string;
  taskType: string;
  serverId: string | null;
  title: string;
  status: PersistedTaskStatus;
  progress: number | null;
  bytesTransferred: number;
  totalBytes: number | null;
  startedAt: string;
  finishedAt: string | null;
  errorCode: string | null;
  errorMessage: string | null;
  cancelSupported: boolean;
  retryPayloadJson: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface SaveTaskInput {
  id: string;
  taskType: string;
  serverId?: string;
  title: string;
  status: PersistedTaskStatus;
  progress: number | null;
  bytesTransferred: number;
  totalBytes: number | null;
  startedAt: string;
  finishedAt?: string | null;
  errorCode?: string | null;
  errorMessage?: string | null;
  cancelSupported: boolean;
  retryPayloadJson?: string | null;
}

export interface RemoteBinaryPreview {
  path: string;
  mimeType: string;
  dataBase64: string;
  size: number;
  modifiedAt: number | null;
}

export interface TerminationResult {
  pid: number;
  signal: string;
  processExited: boolean;
  portReleased: boolean | null;
}

export interface ServiceDetail {
  name: string;
  description: string;
  load: string;
  active: string;
  sub: string;
  mainPid: number | null;
  fragmentPath: string;
  unitFileState: string;
}

export interface ServiceLogs {
  name: string;
  output: string;
}

export interface AuditEvent {
  id: string;
  serverId: string | null;
  action: string;
  resourceType: string;
  resourceId: string | null;
  result: string;
  summary: string;
  createdAt: string;
}

export interface DiagnosticsExport {
  generatedAt: string;
  appVersion: string;
  platform: string;
  architecture: string;
  servers: Array<{ id: string; name: string; host: string; port: number; username: string; authType: string; sudoMode: string; favorite: boolean; tags: string[] }>;
  connections: Array<{ serverId: string; status: string; connectedAt: string | null; errorCode: string | null }>;
  recentAudit: AuditEvent[];
}

export interface ToolStatus {
  id: string;
  name: string;
  description: string;
  installed: boolean;
  version: string | null;
  running: boolean | null;
  packageManager: string | null;
  installPackage: string | null;
  requiresSudo: boolean;
}

export interface ToolInstallPlan {
  tool: ToolStatus;
  command: string;
  risk: string;
}

export interface ToolInstallResult {
  toolId: string;
  output: string;
  verified: ToolStatus;
}

export interface ReverseProxy {
  serverNames: string[];
  listen: string[];
  location: string;
  upstream: string;
  targetHost: string;
  targetPort: number | null;
  ssl: boolean;
  sourceFile: string;
  sourceLine: number;
}

export interface NginxSnapshot {
  installed: boolean;
  running: boolean;
  version: string | null;
  configPath: string | null;
  configTest: boolean | null;
  managedConfSupported: boolean;
  proxies: ReverseProxy[];
  certificates: Array<{ certificatePath: string; privateKeyPath: string | null; sourceFile: string; sourceLine: number; expiresAt: string | null; daysRemaining: number | null }>;
  configSources: string[];
  servers: number;
  upstreams: number;
  warnings: string[];
}

export interface DockerSnapshot {
  installed: boolean;
  running: boolean;
  version: string | null;
  apiVersion: string | null;
  os: string | null;
  architecture: string | null;
  storageDriver: string | null;
  cgroupVersion: string | null;
  rootDir: string | null;
  diskUsage: string | null;
  containers: DockerContainerInfo[];
  images: DockerImageInfo[];
  volumes: DockerVolumeInfo[];
  networks: DockerNetworkInfo[];
  composeProjects: DockerComposeProject[];
}

export interface DockerContainerInfo {
  id: string;
  name: string;
  image: string;
  status: string;
  health: string | null;
  created: string;
  ports: string;
  composeProject: string | null;
  restartPolicy: string | null;
  cpuLimitNano: number | null;
  memoryLimitBytes: number | null;
}

export interface DockerImageInfo {
  repository: string;
  tag: string;
  id: string;
  size: string;
  created: string;
  dangling: boolean;
}

export interface DockerVolumeInfo {
  name: string;
  driver: string;
  mountpoint: string;
  labels: string;
}

export interface DockerNetworkInfo {
  id: string;
  name: string;
  driver: string;
  scope: string;
}

export interface DockerComposeProject {
  name: string;
  status: string;
  configFiles: string;
  workingDir: string;
}

export interface DockerComposeService {
  name: string;
  service: string;
  image: string;
  state: string;
  status: string;
  ports: string;
}

export interface DockerComposeDetails {
  project: string;
  services: DockerComposeService[];
  config: string;
  configPath: string | null;
  configSize: number | null;
  configModifiedAt: number | null;
  volumes: string[];
  networks: string[];
}

export interface DockerTextResult {
  containerId: string;
  output: string;
}

export interface DockerPullResult {
  image: string;
  output: string;
}

export interface DockerRunResult {
  containerId: string;
  output: string;
}

export interface DockerActionResult {
  containerId: string;
  action: string;
  verifiedStatus: string;
}

export interface DockerResourceActionResult {
  kind: string;
  name: string;
  action: string;
  verified: boolean;
}

export interface DockerLogs {
  containerId: string;
  output: string;
}
