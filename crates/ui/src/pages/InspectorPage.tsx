import { Activity, Clock, AlertTriangle, CheckCircle2, XCircle } from "lucide-react";
import { useState, useEffect } from "react";
import { listSessions, listTools, fetchLogs } from "../api";

interface LogEntry {
  id: string;
  timestamp: string;
  level: "info" | "warn" | "error" | "success";
  message: string;
  details?: string;
}

export function InspectorPage() {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [activeTab, setActiveTab] = useState<"logs" | "sessions" | "tools">("logs");
  const [sessions, setSessions] = useState<{ id: string; model?: string; updated_at: string }[]>([]);
  const [tools, setTools] = useState<{ name: string; description: string }[]>([]);

  useEffect(() => {
    fetchLogs(50)
      .then(data => {
        const entries = (data.entries || []).map((e, i) => ({
          id: String(i),
          timestamp: e.timestamp || new Date().toISOString(),
          level: (e.level as "info" | "warn" | "error" | "success") || "info",
          message: e.message || "",
          details: e.details,
        }));
        setLogs(entries);
      })
      .catch(() => {});
  }, []);

  useEffect(() => {
    listSessions().then(s => setSessions(s)).catch(() => {});
  }, []);

  useEffect(() => {
    listTools()
      .then(data => setTools(data.map(t => ({ name: t.name, description: t.description }))))
      .catch(() => {});
  }, []);

  const clearLogs = () => setLogs([]);

  return (
    <div className="page-container inspector-page">
      <div className="page-header">
        <h1>Inspector</h1>
        <div className="inspector-status">
          <span className="status-dot online" />
          <span>Live</span>
        </div>
      </div>

      <div className="inspector-tabs">
        <button
          className={`tab-btn ${activeTab === "logs" ? "active" : ""}`}
          onClick={() => setActiveTab("logs")}
        >
          <Activity size={14} /> Logs ({logs.length})
        </button>
        <button
          className={`tab-btn ${activeTab === "sessions" ? "active" : ""}`}
          onClick={() => setActiveTab("sessions")}
        >
          Sessions ({sessions.length})
        </button>
        <button
          className={`tab-btn ${activeTab === "tools" ? "active" : ""}`}
          onClick={() => setActiveTab("tools")}
        >
          Tools
        </button>
      </div>

      <div className="inspector-content">
        {activeTab === "logs" && (
          <div className="logs-panel">
            <div className="logs-actions">
              <button className="logs-clear-btn" onClick={clearLogs}>Clear Logs</button>
            </div>
            <div className="logs-list">
              {logs.map(log => (
                <div key={log.id} className={`log-entry ${log.level}`}>
                  <div className="log-icon">
                    {log.level === "info" && <Activity size={14} />}
                    {log.level === "warn" && <AlertTriangle size={14} />}
                    {log.level === "error" && <XCircle size={14} />}
                    {log.level === "success" && <CheckCircle2 size={14} />}
                  </div>
                  <div className="log-content">
                    <div className="log-header">
                      <span className="log-message">{log.message}</span>
                      <span className="log-time">
                        <Clock size={11} /> {new Date(log.timestamp).toLocaleTimeString()}
                      </span>
                    </div>
                    {log.details && <div className="log-details">{log.details}</div>}
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {activeTab === "sessions" && (
          <div className="sessions-panel">
            {sessions.length === 0 ? (
              <div className="inspector-empty">No sessions found</div>
            ) : (
              <div className="sessions-list">
                {sessions.map(s => (
                  <div key={s.id} className="session-row">
                    <div className="session-info">
                      <span className="session-name">{s.model || "Chat"}</span>
                      <span className="session-id">{s.id.slice(0, 8)}...</span>
                    </div>
                    <span className="session-time">
                      {new Date(s.updated_at).toLocaleString()}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}

        {activeTab === "tools" && (
          <div className="tools-panel">
            {tools.map(t => (
              <div key={t.name} className="tool-row">
                <span className="tool-name">{t.name}</span>
                <span className="tool-status enabled">enabled</span>
              </div>
            ))}
            {tools.length === 0 && <div className="inspector-empty">No tools loaded</div>}
          </div>
        )}
      </div>
    </div>
  );
}
