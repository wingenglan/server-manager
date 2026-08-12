import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  ConnectionSnapshot,
  HostKeyChallenge,
  SaveServerInput,
  ServerProfile,
  SystemOverview,
  DirectoryListing,
  RemoteTextFile,
  OperationsSnapshot,
  TerminationResult,
} from "../types/server";

export const api = {
  listServers: () => invoke<ServerProfile[]>("list_servers"),
  getServer: (serverId: string) => invoke<ServerProfile>("get_server", { serverId }),
  saveServer: (input: SaveServerInput) => invoke<ServerProfile>("save_server", { input }),
  deleteServer: (serverId: string) => invoke<void>("delete_server", { serverId }),
  connectionState: (serverId: string) =>
    invoke<ConnectionSnapshot>("connection_state", { serverId }),
  connectServer: (serverId: string) =>
    invoke<ConnectionSnapshot | HostKeyChallenge>("connect_server", { serverId }),
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
  upload: (transferId: string, serverId: string, localPath: string, remoteDirectory: string, onEvent: (event: TransferEvent) => void) => {
    const channel = new Channel<TransferEvent>();
    channel.onmessage = onEvent;
    return invoke<void>("upload_remote", { transferId, serverId, localPath, remoteDirectory, onEvent: channel });
  },
  download: (transferId: string, serverId: string, remotePath: string, localDirectory: string, onEvent: (event: TransferEvent) => void) => {
    const channel = new Channel<TransferEvent>();
    channel.onmessage = onEvent;
    return invoke<void>("download_remote", { transferId, serverId, remotePath, localDirectory, onEvent: channel });
  },
  cancelTransfer: (transferId: string) => invoke<void>("cancel_transfer", { transferId }),
  operations: (serverId: string) => invoke<OperationsSnapshot>("get_operations", { serverId }),
  terminateProcess: (input: { serverId: string; pid: number; port?: number; force?: boolean; privileged?: boolean }) =>
    invoke<TerminationResult>("terminate_process", { input }),
  manageService: (serverId: string, service: string, action: "start" | "stop" | "restart") =>
    invoke<void>("manage_service", { serverId, service, action }),
};

export type TerminalEvent =
  | { event: "data"; data: { data: string } }
  | { event: "exit"; data: { exitStatus: number } }
  | { event: "closed" };

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
