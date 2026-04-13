import { useState, useEffect, useCallback } from "react";
import {
  LayoutDashboard,
  TrendingUp,
  Clock,
  FolderKanban,
  HeartPulse,
  AlertTriangle,
  UserCog,
  FileText,
  Radio,
  RefreshCw,
  Plus,
  Loader2,
  MessageSquare,
  Zap,
  Brain,
  Activity,
  Server,
  Timer,
  CheckCircle,
  XCircle,
} from "lucide-react";

const API_BASE = "http://localhost:3848";

// ── Types ──────────────────────────────────────────────────────────

interface HudStats {
  total_sessions: number;
  total_messages: number;
  total_skills: number;
  active_model: string;
  uptime_seconds: number;
  backend_status: string;
}

interface GrowthEntry {
  date: string;
  count: number;
  new_skills: number;
}

interface HealthData {
  api_reachable: boolean;
  model: string;
  provider: string;
  last_error: string | null;
  sessions_db_ok: boolean;
  memory_entries: number;
  skills_count: number;
}

interface LogEntry {
  timestamp: string;
  level: string;
  target: string;
  message: string;
  model?: string;
  session_id?: string;
}

interface LogsResponse {
  entries: LogEntry[];
  total: number;
}

interface ModelEntry {
  id: string;
  provider: string;
  name: string;
}

interface ModelsResponse {
  models: ModelEntry[];
  current: string;
}

interface SkillEntry {
  name: string;
  description?: string;
  enabled?: boolean;
}

interface ConfigData {
  model: string;
  provider: string;
  api_url: string;
  [key: string]: unknown;
}

type TabId =
  | "dashboard"
  | "growth"
  | "cron"
  | "project"
  | "health"
  | "corrections"
  | "profiles"
  | "patterns"
  | "operator";

interface TabDef {
  id: TabId;
  label: string;
  icon: React.ComponentType<{ size?: number | string }>;
}

const TABS: TabDef[] = [
  { id: "dashboard", label: "Dashboard", icon: LayoutDashboard },
  { id: "growth", label: "Growth", icon: TrendingUp },
  { id: "cron", label: "Cron", icon: Clock },
  { id: "project", label: "Project", icon: FolderKanban },
  { id: "health", label: "Health", icon: HeartPulse },
  { id: "corrections", label: "Corrections", icon: AlertTriangle },
  { id: "profiles", label: "Profiles", icon: UserCog },
  { id: "patterns", label: "Prompt Patterns", icon: FileText },
  { id: "operator", label: "Operator", icon: Radio },
];

// ── Helpers ────────────────────────────────────────────────────────

function formatUptime(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

function formatTimestamp(ts: string): string {
  try {
    return new Date(ts).toLocaleString();
  } catch {
    return ts;
  }
}

function levelColor(level: string): string {
  switch (level.toLowerCase()) {
    case "error":
      return "var(--danger)";
    case "warn":
    case "warning":
      return "var(--accent)";
    case "info":
      return "var(--success)";
    default:
      return "var(--text-secondary)";
  }
}

async function apiFetch<T>(path: string): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

// ── Component ──────────────────────────────────────────────────────

export function HUDPage() {
  const [activeTab, setActiveTab] = useState<TabId>("dashboard");
  const [loading, setLoading] = useState(false);

  const [stats, setStats] = useState<HudStats | null>(null);
  const [growth, setGrowth] = useState<GrowthEntry[]>([]);
  const [health, setHealth] = useState<HealthData | null>(null);
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [errorLogs, setErrorLogs] = useState<LogEntry[]>([]);
  const [models, setModels] = useState<ModelsResponse | null>(null);
  const [skills, setSkills] = useState<SkillEntry[]>([]);
  const [config, setConfig] = useState<ConfigData | null>(null);
  const [logLevelFilter, setLogLevelFilter] = useState<string>("all");
  const [error, setError] = useState<string | null>(null);

  // ── Data loaders ───────────────────────────────────────────────

  const loadStats = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await apiFetch<HudStats>("/api/hud/stats");
      setStats(data);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load stats");
    } finally {
      setLoading(false);
    }
  }, []);

  const loadGrowth = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await apiFetch<{ skills_over_time: GrowthEntry[] }>("/api/hud/growth");
      setGrowth(data.skills_over_time ?? []);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load growth data");
    } finally {
      setLoading(false);
    }
  }, []);

  const loadHealth = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await apiFetch<HealthData>("/api/hud/health");
      setHealth(data);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load health");
    } finally {
      setLoading(false);
    }
  }, []);

  const loadErrorLogs = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await apiFetch<LogsResponse>("/api/logs?level=error&limit=50");
      setErrorLogs(data.entries ?? []);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load error logs");
    } finally {
      setLoading(false);
    }
  }, []);

  const loadLogs = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await apiFetch<LogsResponse>("/api/logs?limit=100");
      setLogs(data.entries ?? []);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load logs");
    } finally {
      setLoading(false);
    }
  }, []);

  const loadModels = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await apiFetch<ModelsResponse>("/api/models/list");
      setModels(data);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load models");
    } finally {
      setLoading(false);
    }
  }, []);

  const loadSkills = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await apiFetch<SkillEntry[]>("/api/skills");
      setSkills(data ?? []);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load skills");
    } finally {
      setLoading(false);
    }
  }, []);

  const loadConfig = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await apiFetch<ConfigData>("/api/config");
      setConfig(data);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load config");
    } finally {
      setLoading(false);
    }
  }, []);

  // ── Tab switch data loading ────────────────────────────────────

  useEffect(() => {
    switch (activeTab) {
      case "dashboard":
        loadStats();
        break;
      case "growth":
        loadGrowth();
        break;
      case "project":
        loadStats();
        loadSkills();
        break;
      case "health":
        loadHealth();
        break;
      case "corrections":
        loadErrorLogs();
        break;
      case "profiles":
        loadConfig();
        loadModels();
        break;
      case "operator":
        loadLogs();
        break;
    }
  }, [activeTab, loadStats, loadGrowth, loadSkills, loadHealth, loadErrorLogs, loadLogs, loadModels, loadConfig]);

  // ── Refresh handler ───────────────────────────────────────────

  const handleRefresh = () => {
    switch (activeTab) {
      case "dashboard":
        loadStats();
        break;
      case "growth":
        loadGrowth();
        break;
      case "project":
        loadStats();
        loadSkills();
        break;
      case "health":
        loadHealth();
        break;
      case "corrections":
        loadErrorLogs();
        break;
      case "profiles":
        loadConfig();
        loadModels();
        break;
      case "operator":
        loadLogs();
        break;
    }
  };

  // ── Filtered logs for operator ─────────────────────────────────

  const filteredLogs =
    logLevelFilter === "all"
      ? logs
      : logs.filter((l) => l.level.toLowerCase() === logLevelFilter.toLowerCase());

  // ── Render helpers ─────────────────────────────────────────────

  const renderSpinner = () => (
    <div className="hud-loading">
      <Loader2 size={24} className="hud-spin" />
      <span>Loading...</span>
    </div>
  );

  const renderError = () =>
    error ? <div className="hud-error">{error}</div> : null;

  // ── Tab content renderers ──────────────────────────────────────

  const renderDashboard = () => {
    if (loading && !stats) return renderSpinner();
    if (!stats) return renderError() || <div className="hud-empty">No data available</div>;

    return (
      <>
        {renderError()}
        <div className="hud-stats-grid">
          <div className="stat-card">
            <div className="stat-icon"><MessageSquare size={24} /></div>
            <div className="stat-info">
              <div className="stat-value">{stats.total_sessions}</div>
              <div className="stat-label">Total Sessions</div>
            </div>
          </div>
          <div className="stat-card">
            <div className="stat-icon"><Activity size={24} /></div>
            <div className="stat-info">
              <div className="stat-value">{stats.total_messages}</div>
              <div className="stat-label">Total Messages</div>
            </div>
          </div>
          <div className="stat-card">
            <div className="stat-icon"><Brain size={24} /></div>
            <div className="stat-info">
              <div className="stat-value">{stats.total_skills}</div>
              <div className="stat-label">Skills Count</div>
            </div>
          </div>
          <div className="stat-card">
            <div className="stat-icon"><Zap size={24} /></div>
            <div className="stat-info">
              <div className="stat-value" style={{ fontSize: "16px" }}>
                {stats.active_model?.split("/").pop() || stats.active_model || "—"}
              </div>
              <div className="stat-label">Active Model</div>
            </div>
          </div>
          <div className="stat-card">
            <div className="stat-icon"><Timer size={24} /></div>
            <div className="stat-info">
              <div className="stat-value">{formatUptime(stats.uptime_seconds)}</div>
              <div className="stat-label">Uptime</div>
            </div>
          </div>
          <div className="stat-card">
            <div className="stat-icon"><Server size={24} /></div>
            <div className="stat-info">
              <div className="stat-value" style={{ display: "flex", alignItems: "center", gap: 8 }}>
                <span
                  className="hud-status-dot"
                  style={{ background: stats.backend_status === "ok" ? "var(--success)" : "var(--danger)" }}
                />
                {stats.backend_status === "ok" ? "Online" : "Offline"}
              </div>
              <div className="stat-label">Backend Status</div>
            </div>
          </div>
        </div>
      </>
    );
  };

  const renderGrowth = () => {
    if (loading && growth.length === 0) return renderSpinner();

    if (growth.length === 0) {
      return (
        <>
          {renderError()}
          <div className="hud-empty">
            <TrendingUp size={48} />
            <h2>No Growth Data</h2>
            <p>Skills growth data will appear here once collected.</p>
          </div>
        </>
      );
    }

    const maxCount = Math.max(...growth.map((g) => g.count), 1);

    return (
      <>
        {renderError()}
        <div className="hud-section">
          <h2>Skills Over Time</h2>
          <div className="hud-growth-chart">
            {growth.map((entry, i) => (
              <div key={i} className="hud-growth-bar-group">
                <div className="hud-growth-bar-track">
                  <div
                    className="hud-growth-bar"
                    style={{ width: `${(entry.count / maxCount) * 100}%` }}
                  />
                </div>
                <div className="hud-growth-label">
                  <span className="hud-growth-date">{entry.date}</span>
                  <span className="hud-growth-new">+{entry.new_skills} new</span>
                  <span className="hud-growth-total">{entry.count} total</span>
                </div>
              </div>
            ))}
          </div>
        </div>

        <div className="hud-section" style={{ marginTop: 24 }}>
          <h2>Growth Table</h2>
          <table className="hud-table">
            <thead>
              <tr>
                <th>Date</th>
                <th>New Skills</th>
                <th>Total Count</th>
              </tr>
            </thead>
            <tbody>
              {growth.map((entry, i) => (
                <tr key={i}>
                  <td>{entry.date}</td>
                  <td style={{ color: "var(--success)" }}>+{entry.new_skills}</td>
                  <td>{entry.count}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </>
    );
  };

  const renderCron = () => (
    <div className="hud-empty">
      <Clock size={48} />
      <h2>No Cron Jobs Configured</h2>
      <p>Schedule recurring tasks and automated workflows.</p>
      <button className="hud-add-btn">
        <Plus size={16} /> Add Cron Job
      </button>
    </div>
  );

  const renderProject = () => {
    if (loading && !stats) return renderSpinner();

    return (
      <>
        {renderError()}
        <div className="hud-section">
          <h2>Project Overview</h2>
          <div className="hud-stats-grid" style={{ gridTemplateColumns: "repeat(auto-fit, minmax(180px, 1fr))" }}>
            <div className="stat-card">
              <div className="stat-icon"><MessageSquare size={20} /></div>
              <div className="stat-info">
                <div className="stat-value">{stats?.total_sessions ?? "—"}</div>
                <div className="stat-label">Sessions</div>
              </div>
            </div>
            <div className="stat-card">
              <div className="stat-icon"><Brain size={20} /></div>
              <div className="stat-info">
                <div className="stat-value">{skills.length || (stats?.total_skills ?? "—")}</div>
                <div className="stat-label">Skills</div>
              </div>
            </div>
            <div className="stat-card">
              <div className="stat-icon"><Activity size={20} /></div>
              <div className="stat-info">
                <div className="stat-value">{stats?.total_messages ?? "—"}</div>
                <div className="stat-label">Messages</div>
              </div>
            </div>
          </div>
        </div>

        {skills.length > 0 && (
          <div className="hud-section" style={{ marginTop: 24 }}>
            <h2>Project Skills</h2>
            <div className="hud-skills-list">
              {skills.map((s, i) => (
                <div key={i} className="hud-skill-item">
                  <Brain size={16} />
                  <div>
                    <div className="hud-skill-name">{s.name}</div>
                    {s.description && <div className="hud-skill-desc">{s.description}</div>}
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}
      </>
    );
  };

  const renderHealth = () => {
    if (loading && !health) return renderSpinner();
    if (!health) return renderError() || <div className="hud-empty">No health data</div>;

    const cards = [
      { label: "API Reachable", value: health.api_reachable, type: "bool" as const },
      { label: "Model", value: health.model, type: "text" as const },
      { label: "Provider", value: health.provider, type: "text" as const },
      { label: "Sessions DB OK", value: health.sessions_db_ok, type: "bool" as const },
      { label: "Memory Entries", value: health.memory_entries, type: "number" as const },
      { label: "Skills Count", value: health.skills_count, type: "number" as const },
    ];

    return (
      <>
        {renderError()}
        <div className="hud-health-grid">
          {cards.map((card) => (
            <div key={card.label} className="hud-health-card">
              <div className="hud-health-label">{card.label}</div>
              <div className="hud-health-value">
                {card.type === "bool" ? (
                  <span style={{ display: "flex", alignItems: "center", gap: 8 }}>
                    {card.value ? (
                      <CheckCircle size={20} style={{ color: "var(--success)" }} />
                    ) : (
                      <XCircle size={20} style={{ color: "var(--danger)" }} />
                    )}
                    {card.value ? "Yes" : "No"}
                  </span>
                ) : card.type === "number" ? (
                  String(card.value)
                ) : (
                  String(card.value || "—")
                )}
              </div>
            </div>
          ))}
        </div>
        {health.last_error && (
          <div className="hud-section" style={{ marginTop: 24 }}>
            <h2>Last Error</h2>
            <div className="hud-error-banner">{health.last_error}</div>
          </div>
        )}
      </>
    );
  };

  const renderCorrections = () => {
    if (loading && errorLogs.length === 0) return renderSpinner();

    if (errorLogs.length === 0) {
      return (
        <>
          {renderError()}
          <div className="hud-empty">
            <CheckCircle size={48} style={{ color: "var(--success)" }} />
            <h2>No Errors Found</h2>
            <p>All systems operating normally.</p>
          </div>
        </>
      );
    }

    return (
      <>
        {renderError()}
        <div className="hud-section">
          <h2>Error Logs ({errorLogs.length})</h2>
          <div className="hud-table-scroll">
            <table className="hud-table">
              <thead>
                <tr>
                  <th>Timestamp</th>
                  <th>Level</th>
                  <th>Target</th>
                  <th>Message</th>
                </tr>
              </thead>
              <tbody>
                {errorLogs.map((entry, i) => (
                  <tr key={i}>
                    <td className="hud-ts">{formatTimestamp(entry.timestamp)}</td>
                    <td>
                      <span className="hud-level-badge" style={{ background: levelColor(entry.level) }}>
                        {entry.level}
                      </span>
                    </td>
                    <td className="hud-mono">{entry.target}</td>
                    <td className="hud-msg">{entry.message}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </>
    );
  };

  const renderProfiles = () => {
    if (loading && !config) return renderSpinner();
    if (!config) return renderError() || <div className="hud-empty">No config data</div>;

    return (
      <>
        {renderError()}
        <div className="hud-section">
          <h2>Current Configuration</h2>
          <div className="hud-profile-grid">
            <div className="hud-profile-item">
              <div className="hud-profile-label">Model</div>
              <div className="hud-profile-value">{config.model || "—"}</div>
            </div>
            <div className="hud-profile-item">
              <div className="hud-profile-label">Provider</div>
              <div className="hud-profile-value">{config.provider || "—"}</div>
            </div>
            <div className="hud-profile-item">
              <div className="hud-profile-label">API URL</div>
              <div className="hud-profile-value hud-mono">{config.api_url || "—"}</div>
            </div>
          </div>
        </div>

        {models && models.models.length > 0 && (
          <div className="hud-section" style={{ marginTop: 24 }}>
            <h2>Available Models</h2>
            <div className="hud-table-scroll">
              <table className="hud-table">
                <thead>
                  <tr>
                    <th>ID</th>
                    <th>Provider</th>
                    <th>Name</th>
                    <th>Current</th>
                  </tr>
                </thead>
                <tbody>
                  {models.models.map((m) => (
                    <tr key={m.id}>
                      <td className="hud-mono">{m.id}</td>
                      <td>{m.provider}</td>
                      <td>{m.name}</td>
                      <td>
                        {models.current === m.id ? (
                          <CheckCircle size={16} style={{ color: "var(--success)" }} />
                        ) : (
                          ""
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </div>
        )}
      </>
    );
  };

  const renderPatterns = () => (
    <div className="hud-empty">
      <FileText size={48} />
      <h2>No Prompt Patterns Collected Yet</h2>
      <p>Patterns will be automatically collected as you interact with the agent.</p>
    </div>
  );

  const renderOperator = () => {
    if (loading && logs.length === 0) return renderSpinner();

    return (
      <>
        {renderError()}
        <div className="hud-section">
          <div className="hud-operator-header">
            <h2>Operator Logs</h2>
            <div className="hud-filter-group">
              <label>Level:</label>
              <select
                value={logLevelFilter}
                onChange={(e) => setLogLevelFilter(e.target.value)}
                className="hud-select"
              >
                <option value="all">All</option>
                <option value="error">Error</option>
                <option value="warn">Warn</option>
                <option value="info">Info</option>
                <option value="debug">Debug</option>
              </select>
            </div>
          </div>
          <div className="hud-table-scroll">
            <table className="hud-table">
              <thead>
                <tr>
                  <th>Timestamp</th>
                  <th>Level</th>
                  <th>Target</th>
                  <th>Message</th>
                  <th>Model</th>
                </tr>
              </thead>
              <tbody>
                {filteredLogs.map((entry, i) => (
                  <tr key={i}>
                    <td className="hud-ts">{formatTimestamp(entry.timestamp)}</td>
                    <td>
                      <span className="hud-level-badge" style={{ background: levelColor(entry.level) }}>
                        {entry.level}
                      </span>
                    </td>
                    <td className="hud-mono">{entry.target}</td>
                    <td className="hud-msg">{entry.message}</td>
                    <td>{entry.model || "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          {filteredLogs.length === 0 && logs.length > 0 && (
            <div className="hud-empty" style={{ padding: "24px 0" }}>
              <p>No logs matching filter "{logLevelFilter}"</p>
            </div>
          )}
        </div>
      </>
    );
  };

  // ── Tab content router ────────────────────────────────────────

  const renderTabContent = () => {
    switch (activeTab) {
      case "dashboard":
        return renderDashboard();
      case "growth":
        return renderGrowth();
      case "cron":
        return renderCron();
      case "project":
        return renderProject();
      case "health":
        return renderHealth();
      case "corrections":
        return renderCorrections();
      case "profiles":
        return renderProfiles();
      case "patterns":
        return renderPatterns();
      case "operator":
        return renderOperator();
    }
  };

  // ── Main render ───────────────────────────────────────────────

  return (
    <div className="page-container hud-page">
      <div className="page-header hud-header">
        <div className="hud-header-left">
          <h1>Hermes HUD</h1>
        </div>
        <button
          className="hud-refresh-btn"
          onClick={handleRefresh}
          disabled={loading}
          title="Refresh data"
        >
          <RefreshCw size={16} className={loading ? "hud-spin" : ""} />
          Refresh
        </button>
      </div>

      <div className="hud-tabs">
        {TABS.map((tab) => {
          const Icon = tab.icon;
          return (
            <button
              key={tab.id}
              className={`tab-btn ${activeTab === tab.id ? "active" : ""}`}
              onClick={() => setActiveTab(tab.id)}
            >
              <Icon size={14} />
              <span>{tab.label}</span>
            </button>
          );
        })}
      </div>

      <div className="page-content hud-content">{renderTabContent()}</div>
    </div>
  );
}
