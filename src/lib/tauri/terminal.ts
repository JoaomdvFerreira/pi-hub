import { invoke } from "@tauri-apps/api/core";

export function openTerminalSession(deviceId: string): Promise<string> {
  return invoke<string>("open_terminal_session", { deviceId });
}

export function writeTerminalInput(sessionId: string, data: string): Promise<void> {
  return invoke<void>("write_terminal_input", { sessionId, data });
}

export function resizeTerminalSession(
  sessionId: string,
  cols: number,
  rows: number,
): Promise<void> {
  return invoke<void>("resize_terminal_session", { sessionId, cols, rows });
}

export function closeTerminalSession(sessionId: string): Promise<void> {
  return invoke<void>("close_terminal_session", { sessionId });
}
