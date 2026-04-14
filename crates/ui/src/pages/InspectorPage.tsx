import { ChevronDown, ChevronUp, Wrench, MessageSquare, Monitor } from "lucide-react";
import { useState, useEffect, useRef, useCallback } from "react";
import { listSessions, listTools, fetchLogs, SessionInfo } from "../api";

const LEVEL_COLORS: Record<string, { bg: string; text: string }> = {
  info: { bg: "rgba(201,162,39,0.15)", text: "#c9a227" },
  warn: { bg: "rgba(245,158,11,0.15)", text: "#f59e0b" },
  error: { bg: "rgba(229,62,62,0.15)", text: "#ef4444" },
  debug: { bg: "rgba(100,100,100,0.15)", text: "#888" },
};

const POLL_INTERVAL = 2000;
const MAX_LOG_LINES = 500;

export function InspectorPage() {
  const [activeTab, setActiveTab] = useState<"tools" | "sessions">("tools");
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [tools, setTools] = useState<{ name: string; description: string }[]>([]);

  const [liveLogs, setLiveLogs] = useState<string[]>([]);
  const [logPanelHeight, setLogPanelHeight] = useState(180);
  const [logExpanded, setLogExpanded] = useState(true);
  const logEndRef = useRef<HTMLDivElement>(null);
  const lastCountRef = useRef(0);

  useEffect(() => {
    listSessions().then(s => setSessions(s)).catch(() => {});
  }, []);

  useEffect(() => {
    listTools()
      .then(data => {
        const mapped = data.map(t => {
          if (t.function && t.function.name) {
            return { name: t.function.name, description: t.function.description || "" };
          }
          return { name: t.name || "unknown", description: t.description || "" };
        });
        setTools(mapped);
      })
      .catch(() => {});
  }, []);

  const pollLogs = useCallback(async () => {
    try {
      const data = await fetchLogs(MAX_LOG_LINES);
      const lines = data.lines || [];
      if (lines.length !== lastCountRef.current) {
        lastCountRef.current = lines.length;
        setLiveLogs(lines);
      }
    } catch {}
  }, []);

  useEffect(() => {
    pollLogs();
    const timer = setInterval(pollLogs, POLL_INTERVAL);
    return () => clearInterval(timer);
  }, [pollLogs]);

  useEffect(() => {
    if (logEndRef.current) {
      logEndRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [liveLogs.length]);

  const handleResizeStart = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    const startY = e.clientY;
    const startH = logPanelHeight;
    const onMove = (ev: MouseEvent) => {
      const delta = startY - ev.clientY;
      setLogPanelHeight(Math.max(80, Math.min(500, startH + delta)));
    };
    const onUp = () => {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
    document.body.style.cursor = "ns-resize";
    document.body.style.userSelect = "none";
  }, [logPanelHeight]);

  const parseLine = (line: string) => {
    const m = line.match(/^\[(\w+)\]\s*(.*)/);
    return { level: m ? m[1].toLowerCase() : "info", message: m ? m[2] : line };
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", overflow: "hidden" }}>
      <div className="page-container inspector-page" style={{ flex: "1 1 auto", overflow: "auto", minHeight: 0 }}>
        <div className="page-header">
          <h1>Inspector</h1>
        </div>

        <div className="inspector-tabs">
          <button
            className={`tab-btn ${activeTab === "tools" ? "active" : ""}`}
            onClick={() => setActiveTab("tools")}
          >
            <Wrench size={14} /> Tools ({tools.length})
          </button>
          <button
            className={`tab-btn ${activeTab === "sessions" ? "active" : ""}`}
            onClick={() => setActiveTab("sessions")}
          >
            <MessageSquare size={14} /> Sessions ({sessions.length})
          </button>
        </div>

        <div className="inspector-content" style={{ padding: "0 24px 24px" }}>
          {activeTab === "tools" && (
            <div style={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fill, minmax(340px, 1fr))",
              gap: 6,
            }}>
              {tools.map(t => (
                <div key={t.name} style={{
                  display: "flex", alignItems: "center", gap: 10,
                  padding: "10px 14px",
                  background: "var(--bg-secondary)",
                  border: "1px solid var(--border)",
                  borderRadius: 8,
                }}>
                  <div style={{ display: "flex", flexDirection: "column", gap: 2, flex: 1, minWidth: 0 }}>
                    <span style={{
                      fontSize: 13, fontFamily: "monospace", fontWeight: 600,
                      color: "var(--accent)",
                    }}>
                      {t.name}
                    </span>
                    {t.description && (
                      <span style={{
                        fontSize: 12, color: "var(--text-secondary)", lineHeight: 1.4,
                        overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap",
                      }}>
                        {t.description}
                      </span>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )}

          {activeTab === "sessions" && (
            <div className="sessions-list">
              {sessions.length === 0 ? (
                <div className="inspector-empty">No sessions found</div>
              ) : (
                sessions.map(s => (
                  <div key={s.id} className="session-row">
                    <div className="session-info">
                      <span className="session-name" style={{ color: "var(--text-primary)" }}>
                        {s.model || "Chat"}
                      </span>
                      <span className="session-id">{s.id.slice(0, 8)}...</span>
                    </div>
                    <span className="session-time">
                      {s.updated_at ? new Date(s.updated_at).toLocaleString() : "—"}
                    </span>
                  </div>
                ))
              )}
            </div>
          )}
        </div>
      </div>

      <div
        onMouseDown={handleResizeStart}
        style={{
          flex: "0 0 4px",
          background: "var(--border)",
          cursor: "ns-resize",
          transition: "background 0.15s",
        }}
        onMouseOver={e => (e.currentTarget.style.background = "var(--accent)")}
        onMouseOut={e => (e.currentTarget.style.background = "var(--border)")}
      />

      <div style={{
        flex: "0 0 auto",
        height: logExpanded ? logPanelHeight : 32,
        display: "flex",
        flexDirection: "column",
        background: "var(--bg-primary)",
        borderTop: "1px solid var(--border)",
        overflow: "hidden",
      }}>
        <div style={{
          display: "flex", alignItems: "center", justifyContent: "space-between",
          padding: "6px 16px",
          background: "var(--bg-secondary)",
          borderBottom: "1px solid var(--border)",
          cursor: "pointer",
          flexShrink: 0,
        }} onClick={() => setLogExpanded(v => !v)}>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <Monitor size={14} style={{ color: "var(--accent)" }} />
            <span style={{ fontSize: 13, fontWeight: 600, color: "var(--text-primary)" }}>Live Logs</span>
            <span style={{ fontSize: 11, color: "var(--text-secondary)" }}>
              {liveLogs.length} lines (max {MAX_LOG_LINES})
            </span>
          </div>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <span style={{ fontSize: 10, color: "var(--text-secondary)" }}>
              {logPanelHeight}px
            </span>
            {logExpanded ? <ChevronDown size={14} style={{ color: "var(--text-secondary)" }} /> : <ChevronUp size={14} style={{ color: "var(--text-secondary)" }} />}
          </div>
        </div>

        {logExpanded && (
          <div style={{
            flex: 1, overflowY: "auto", overflowX: "hidden",
            padding: "4px 0",
            fontFamily: "monospace",
            fontSize: 12,
          }}>
            {liveLogs.length === 0 ? (
              <div style={{
                display: "flex", alignItems: "center", justifyContent: "center",
                height: "100%", color: "var(--text-secondary)", fontSize: 13,
              }}>
                Waiting for logs...
              </div>
            ) : (
              liveLogs.map((line, i) => {
                const { level, message } = parseLine(line);
                const colors = LEVEL_COLORS[level] || LEVEL_COLORS.info;
                return (
                  <div key={i} style={{
                    display: "flex", alignItems: "flex-start", gap: 8,
                    padding: "2px 16px",
                    color: "var(--text-primary)",
                    lineHeight: 1.5,
                  }}>
                    <span style={{
                      display: "inline-block", width: 50, flexShrink: 0,
                      fontSize: 10, fontWeight: 700, textTransform: "uppercase",
                      padding: "0 4px", borderRadius: 2,
                      background: colors.bg, color: colors.text,
                      textAlign: "center",
                      lineHeight: "18px",
                    }}>
                      {level}
                    </span>
                    <span style={{ flex: 1, minWidth: 0, wordBreak: "break-all", color: "var(--text-secondary)" }}>
                      {message}
                    </span>
                  </div>
                );
              })
            )}
            <div ref={logEndRef} />
          </div>
        )}
      </div>
    </div>
  );
}
