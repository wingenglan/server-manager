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
  tags: string[];
  favorite: boolean;
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
  uptimeSeconds: number;
  cpuModel: string;
  logicalCores: number;
  cpuUsagePercent: number | null;
  load: [number, number, number];
  memoryTotalBytes: number;
  memoryAvailableBytes: number;
  swapTotalBytes: number;
  swapFreeBytes: number;
  disks: Array<{ mount: string; totalBytes: number; usedBytes: number; usagePercent: number }>;
  docker: { installed: boolean; running: boolean; version: string | null };
  nginx: { installed: boolean; running: boolean; version: string | null };
  capabilities: Record<string, boolean>;
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
  services: Array<{ name: string; load: string; active: string; sub: string; description: string }>;
}

export interface TerminationResult {
  pid: number;
  signal: string;
  processExited: boolean;
  portReleased: boolean | null;
}
