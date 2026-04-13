const API_BASE = "http://localhost:3848";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: Record<string, unknown>;
  }
}

const isTauri = typeof window !== "undefined" && !!window.__TAURI_INTERNALS__;

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke: tauriInvoke } = await import("@tauri-apps/api/core");
  return tauriInvoke<T>(cmd, args);
}

async function listen<T>(event: string, handler: (event: { payload: T }) => void): Promise<() => void> {
  const { listen: tauriListen } = await import("@tauri-apps/api/event");
  return tauriListen<T>(event, handler);
}

export interface ChatMessage {
  role: "user" | "assistant" | "system";
  content: string;
}

export interface ChatRequest {
  model: string;
  messages: ChatMessage[];
  api_url?: string;
  api_key?: string;
  session_id?: string;
}

export interface ChatResponse {
  content: string;
  session_id?: string;
}

export interface SessionInfo {
  id: string;
  created_at: string;
  updated_at: string;
  model?: string;
}

export interface ToolInfo {
  name: string;
  description: string;
}

export async function chat(request: ChatRequest): Promise<ChatResponse> {
  if (isTauri) {
    return invoke<ChatResponse>("chat", { request });
  }
  const res = await fetch(`${API_BASE}/api/chat`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(request),
  });
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: "Unknown error" }));
    throw new Error(err.error || `HTTP ${res.status}`);
  }
  return res.json();
}

export interface StreamChunk {
  content: string;
  done: boolean;
}

export interface ChatStreamOptions {
  onChunk: (chunk: StreamChunk) => void;
  onError?: (err: Error) => void;
  onDone?: () => void;
}

export function chatStream(request: ChatRequest, options: ChatStreamOptions): Promise<string | null> {
  const { onChunk, onError, onDone } = options;

  if (isTauri) {
    let unlisten: (() => void) | null = null;

    const chunkHandler = (event: { payload: { content: string; done: boolean } }) => {
      onChunk({ content: event.payload.content, done: event.payload.done });
      if (event.payload.done) {
        onDone?.();
        if (unlisten) unlisten();
      }
    };

    const errorHandler = (event: { payload: { error: string } }) => {
      onError?.(new Error(event.payload.error));
    };

    let sessionId: string | null = null;

    return Promise.all([
      listen<{ content: string; done: boolean }>("chat-chunk", chunkHandler),
      listen<{ error: string }>("chat-error", errorHandler),
    ]).then(([unlistenChunk, unlistenError]) => {
      unlisten = () => {
        unlistenChunk();
        unlistenError();
      };
    }).then(() => {
      return invoke<string>("chat_stream", { request });
    }).then(sid => {
      sessionId = sid || null;
      return sessionId;
    }).catch(err => {
      onError?.(new Error(err));
      return null;
    });
  }

  const trySse = (): Promise<string | null> => {
    return fetch(`${API_BASE}/api/chat/stream`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request),
    }).then(async res => {
      if (!res.ok) {
        const err = await res.json().catch(() => ({ error: "Unknown error" }));
        throw new Error(err.error || `HTTP ${res.status}`);
      }
      const reader = res.body?.getReader();
      if (!reader) throw new Error("No response body");
      
      const decoder = new TextDecoder();
      let buffer = "";
      let sessionId: string | null = null;

      const processBuffer = () => {
        while (true) {
          const newlineIndex = buffer.indexOf("\n");
          if (newlineIndex === -1) break;
          
          const line = buffer.slice(0, newlineIndex);
          buffer = buffer.slice(newlineIndex + 1);
          
          if (line.startsWith("data: ")) {
            try {
              const data = JSON.parse(line.slice(6));
              if (data.session_id && !sessionId) {
                sessionId = data.session_id;
              }
              if (data.done) {
                onChunk({ content: "", done: true });
                onDone?.();
              } else if (data.content !== undefined) {
                onChunk({ content: data.content || "", done: false });
              }
            } catch {}
          }
        }
      };

      const pump = () => {
        reader.read().then(({ done, value }) => {
          if (done) {
            if (buffer.startsWith("data: ")) {
              try {
                const data = JSON.parse(buffer.slice(6));
                if (data.done) {
                  onChunk({ content: "", done: true });
                  onDone?.();
                }
              } catch {}
            }
            return;
          }
          buffer += decoder.decode(value, { stream: true });
          processBuffer();
          pump();
        }).catch(err => {
          onError?.(err);
        });
      };
      
      pump();
      return sessionId;
    });
  };

  return trySse().catch(err => {
    if (err.message === "Failed to fetch" || err.message.includes("network") || err.name === "TypeError") {
      return chat(request).then(data => {
        const chars = data.content.split("");
        let i = 0;
        const pump = () => {
          if (i < chars.length) {
            onChunk({ content: chars[i], done: false });
            i++;
            setTimeout(pump, 15);
          } else {
            onChunk({ content: "", done: true });
            onDone?.();
          }
        };
        pump();
        return data.session_id || null;
      });
    }
    onError?.(err);
    throw err;
  });
}

export async function listSessions(): Promise<SessionInfo[]> {
  if (isTauri) {
    return invoke<SessionInfo[]>("list_sessions");
  }
  const res = await fetch(`${API_BASE}/api/sessions`);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function createSession(model?: string): Promise<SessionInfo> {
  if (isTauri) {
    return invoke<SessionInfo>("create_session_cmd", { model: model || null });
  }
  const res = await fetch(`${API_BASE}/api/sessions`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ model }),
  });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function getSession(id: string): Promise<SessionInfo> {
  if (isTauri) {
    return invoke<SessionInfo>("get_session", { id });
  }
  const res = await fetch(`${API_BASE}/api/sessions/${id}`);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export interface SessionMessage {
  role: string;
  content: string;
}

export async function getSessionMessages(id: string): Promise<SessionMessage[]> {
  if (isTauri) {
    return invoke<SessionMessage[]>("get_session_messages", { id });
  }
  const res = await fetch(`${API_BASE}/api/sessions/${id}/messages`);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export interface LogEntry {
  timestamp: string;
  level: string;
  message: string;
  details?: string;
}

export interface LogsResponse {
  entries: LogEntry[];
}

export async function fetchLogs(limit?: number): Promise<LogsResponse> {
  if (isTauri) {
    const result = await invoke<unknown>("fetch_logs_cmd", { limit: limit || null });
    return result as LogsResponse;
  }
  const url = limit ? `${API_BASE}/api/logs?limit=${limit}` : `${API_BASE}/api/logs`;
  const res = await fetch(url);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function listTools(): Promise<ToolInfo[]> {
  if (isTauri) {
    return invoke<ToolInfo[]>("list_tools_cmd");
  }
  const res = await fetch(`${API_BASE}/api/tools`);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function healthCheck(): Promise<string> {
  if (isTauri) {
    return invoke<string>("health_check");
  }
  const res = await fetch(`${API_BASE}/health`);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.text();
}

export interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  size: number;
}

export async function listDir(path: string): Promise<FileEntry[]> {
  if (isTauri) {
    const result = await invoke<unknown>("list_dir_cmd", { path });
    return result as FileEntry[];
  }
  const res = await fetch(`${API_BASE}/api/files/list`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ path }),
  });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export interface ReadFileResponse {
  content: string;
  encoding: string;
}

export async function readFile(path: string): Promise<ReadFileResponse> {
  if (isTauri) {
    const result = await invoke<unknown>("read_file_cmd", { path });
    return result as ReadFileResponse;
  }
  const res = await fetch(`${API_BASE}/api/files/read`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ path }),
  });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function writeFile(path: string, content: string): Promise<void> {
  if (isTauri) {
    await invoke<unknown>("write_file_cmd", { path, content });
    return;
  }
  const res = await fetch(`${API_BASE}/api/files/write`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ path, content }),
  });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
}

export interface TerminalRequest {
  command: string;
  cwd?: string;
}

export interface TerminalResponse {
  output: string;
  exit_code: number;
}

export async function execTerminal(command: string, cwd?: string): Promise<TerminalResponse> {
  if (isTauri) {
    const result = await invoke<unknown>("exec_terminal_cmd", { command, cwd: cwd || null });
    return result as TerminalResponse;
  }
  const res = await fetch(`${API_BASE}/api/terminal`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ command, cwd }),
  });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function deleteSession(id: string): Promise<void> {
  if (isTauri) {
    await invoke<unknown>("delete_session_cmd", { id });
    return;
  }
  const res = await fetch(`${API_BASE}/api/sessions/${id}`, {
    method: "DELETE",
  });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
}

export interface AppConfig {
  model: string;
  provider: string;
  api_url: string;
  api_key: string;
  skin: string;
  platforms: {
    telegram: { bot_token: string; enabled: boolean };
    discord: { bot_token: string; enabled: boolean };
    slack: { bot_token: string; enabled: boolean };
    whatsapp: { bridge_url: string; api_token: string; enabled: boolean };
    signal: { http_url: string; account: string; enabled: boolean };
  };
}

export async function getConfig(): Promise<AppConfig> {
  if (isTauri) {
    const result = await invoke<unknown>("get_config_cmd");
    return result as AppConfig;
  }
  const res = await fetch(`${API_BASE}/api/config`);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function updateConfig(update: Partial<AppConfig>): Promise<AppConfig> {
  if (isTauri) {
    const result = await invoke<unknown>("update_config_cmd", { update });
    return result as AppConfig;
  }
  const res = await fetch(`${API_BASE}/api/config`, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(update),
  });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export async function interruptChat(): Promise<void> {
  if (isTauri) {
    await invoke("interrupt_chat_cmd");
    return;
  }
  const res = await fetch(`${API_BASE}/api/chat/interrupt`, { method: "POST" });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
}

export interface ProviderModel {
  id: string;
  name: string;
  aliases: string[];
  context_length?: number;
}

export interface ProviderInfo {
  id: string;
  name: string;
  base_url: string;
  api_key_env: string;
  models: ProviderModel[];
}

export interface ProvidersResponse {
  providers: ProviderInfo[];
  credentials: Record<string, boolean>;
}

export async function getProviders(): Promise<ProvidersResponse> {
  if (isTauri) {
    const result = await invoke<unknown>("get_providers_cmd");
    return result as ProvidersResponse;
  }
  const res = await fetch(`${API_BASE}/api/config/providers`);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

export interface SwitchModelResponse {
  status: string;
  model: string;
}

export async function switchModel(model: string): Promise<SwitchModelResponse> {
  if (isTauri) {
    const result = await invoke<unknown>("switch_model_cmd", { model });
    return result as SwitchModelResponse;
  }
  const res = await fetch(`${API_BASE}/api/models/switch`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ model }),
  });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}