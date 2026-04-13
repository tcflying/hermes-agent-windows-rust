import { MessageSquare, Zap, Activity, Users, Terminal } from "lucide-react";
import { useState, useEffect } from "react";
import { listSessions, getConfig, healthCheck } from "../api";

function formatDate(iso: string): string {
  try {
    const d = new Date(iso);
    const now = new Date();
    const diffMs = now.getTime() - d.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    if (diffMins < 1) return "just now";
    if (diffMins < 60) return `${diffMins}m ago`;
    const diffHours = Math.floor(diffMins / 60);
    if (diffHours < 24) return `${diffHours}h ago`;
    const diffDays = Math.floor(diffHours / 24);
    return `${diffDays}d ago`;
  } catch {
    return "";
  }
}

export function DashboardPage() {
  const [stats, setStats] = useState({
    totalSessions: 0,
    totalMessages: 0,
    recentSessions: [] as { id: string; model?: string; updated_at: string }[],
  });
  const [model, setModel] = useState("MiniMax-M2.7-highspeed");
  const [backendUp, setBackendUp] = useState(false);

  useEffect(() => {
    const load = async () => {
      try {
        const [sessions, cfg, up] = await Promise.all([
          listSessions(),
          getConfig().catch(() => ({ model: "MiniMax-M2.7-highspeed", api_key: "" })),
          healthCheck().then(() => true).catch(() => false),
        ]);
        setBackendUp(up);
        setModel(cfg.model);
        setStats({
          totalSessions: sessions.length,
          totalMessages: 0,
          recentSessions: sessions.slice(0, 5),
        });
      } catch { }
    };
    load();
  }, []);

  return (
    <div className="page-container dashboard-page">
      <div className="page-header">
        <h1>Dashboard</h1>
      </div>

      <div className="dashboard-grid">
        <div className="stat-card">
          <div className="stat-icon">
            <MessageSquare size={24} />
          </div>
          <div className="stat-info">
            <div className="stat-value">{stats.totalSessions}</div>
            <div className="stat-label">Total Sessions</div>
          </div>
        </div>

        <div className="stat-card">
          <div className="stat-icon">
            <Activity size={24} />
          </div>
          <div className="stat-info">
            <div className="stat-value">{backendUp ? "Online" : "Offline"}</div>
            <div className="stat-label">Backend Status</div>
          </div>
        </div>

        <div className="stat-card">
          <div className="stat-icon">
            <Zap size={24} />
          </div>
          <div className="stat-info">
            <div className="stat-value">{model.split("/")[1] || model}</div>
            <div className="stat-label">Current Model</div>
          </div>
        </div>

        <div className="stat-card">
          <div className="stat-icon">
            <Terminal size={24} />
          </div>
          <div className="stat-info">
            <div className="stat-value">v0.1.0</div>
            <div className="stat-label">Hermes Version</div>
          </div>
        </div>
      </div>

      {stats.recentSessions.length > 0 && (
        <div className="dashboard-section">
          <h2>Recent Sessions</h2>
          <div className="recent-sessions-list">
            {stats.recentSessions.map(s => (
              <div key={s.id} className="recent-session-item">
                <MessageSquare size={16} />
                <div className="recent-session-info">
                  <span className="recent-session-model">{s.model || "Chat"}</span>
                  <span className="recent-session-date">{formatDate(s.updated_at)}</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      <div className="dashboard-section">
        <h2>Quick Start</h2>
        <div className="quickstart-cards">
          <div className="quickstart-card">
            <MessageSquare size={20} />
            <div>
              <h3>Start Chatting</h3>
              <p>Go to the Chat tab and start a conversation with Hermes</p>
            </div>
          </div>
          <div className="quickstart-card">
            <Terminal size={20} />
            <div>
              <h3>Terminal Access</h3>
              <p>Use the Terminal tab to run shell commands</p>
            </div>
          </div>
          <div className="quickstart-card">
            <Users size={20} />
            <div>
              <h3>Memory</h3>
              <p>Hermes remembers your preferences across sessions</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
