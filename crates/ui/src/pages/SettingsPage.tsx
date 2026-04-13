import { Key, Palette, Globe, CheckCircle2, XCircle, Loader2, Zap } from "lucide-react";
import { useState, useEffect, useCallback } from "react";
import {
  getConfig,
  updateConfig,
  getProviders,
  switchModel,
  AppConfig,
  ProvidersResponse,
} from "../api";

const THEMES = [
  { id: "official-dark", name: "Official Dark", accent: "#c9a227" },
  { id: "official-light", name: "Official Light", accent: "#c9a227" },
  { id: "classic-dark", name: "Classic Dark", accent: "#4a9eff" },
  { id: "classic-light", name: "Classic Light", accent: "#0066cc" },
  { id: "slate-dark", name: "Slate Dark", accent: "#38bdf8" },
  { id: "slate-light", name: "Slate Light", accent: "#0284c7" },
  { id: "mono-dark", name: "Mono Dark", accent: "#a3a3a3" },
  { id: "mono-light", name: "Mono Light", accent: "#404040" },
];

export function SettingsPage() {
  const [activeTab, setActiveTab] = useState("providers");
  const [apiKey, setApiKey] = useState("");
  const [selectedTheme, setSelectedTheme] = useState("official-dark");
  const [saveStatus, setSaveStatus] = useState<"idle" | "saving" | "saved" | "error">("idle");

  const [config, setConfig] = useState<AppConfig | null>(null);
  const [providersData, setProvidersData] = useState<ProvidersResponse | null>(null);
  const [switching, setSwitching] = useState(false);
  const [toast, setToast] = useState<{ message: string; type: "success" | "error" } | null>(null);

  const [manualModel, setManualModel] = useState("");
  const [manualApiUrl, setManualApiUrl] = useState("");
  const [manualApiKey, setManualApiKey] = useState("");
  const [manualSaveStatus, setManualSaveStatus] = useState<"idle" | "saving" | "saved" | "error">("idle");

  const showToast = useCallback((message: string, type: "success" | "error" = "success") => {
    setToast({ message, type });
    setTimeout(() => setToast(null), 3000);
  }, []);

  useEffect(() => {
    const saved = localStorage.getItem("hermes-theme") || "official-dark";
    setSelectedTheme(saved);
    document.documentElement.setAttribute("data-theme", saved);
  }, []);

  useEffect(() => {
    getConfig().then(cfg => {
      setConfig(cfg);
      setApiKey(cfg.api_key || "");
      setManualModel(cfg.model || "");
      setManualApiUrl(cfg.api_url || "");
      setManualApiKey(cfg.api_key || "");
    }).catch(() => {});
  }, []);

  useEffect(() => {
    getProviders().then(setProvidersData).catch(() => {});
  }, []);

  const handleThemeChange = (themeId: string) => {
    setSelectedTheme(themeId);
    localStorage.setItem("hermes-theme", themeId);
    document.documentElement.setAttribute("data-theme", themeId);
    updateConfig({ skin: themeId }).catch(() => {});
  };

  const handleSaveApiKey = async () => {
    setSaveStatus("saving");
    try {
      await updateConfig({ api_key: apiKey });
      setSaveStatus("saved");
      setTimeout(() => setSaveStatus("idle"), 2000);
    } catch {
      setSaveStatus("error");
      setTimeout(() => setSaveStatus("idle"), 3000);
    }
  };

  const handleModelSwitch = async (modelId: string) => {
    setSwitching(true);
    try {
      const result = await switchModel(modelId);
      setConfig(prev => prev ? { ...prev, model: result.model, provider: result.model.split("/")[0] || prev.provider } : prev);
      showToast(`Switched to ${result.model}`);
    } catch (err) {
      showToast(`Failed to switch model: ${err instanceof Error ? err.message : "Unknown error"}`, "error");
    } finally {
      setSwitching(false);
    }
  };

  const handleManualSave = async () => {
    setManualSaveStatus("saving");
    try {
      const updated = await updateConfig({
        model: manualModel,
        api_url: manualApiUrl,
        api_key: manualApiKey,
      });
      setConfig(updated);
      showToast("Configuration saved");
      setManualSaveStatus("saved");
      setTimeout(() => setManualSaveStatus("idle"), 2000);
    } catch {
      setManualSaveStatus("error");
      showToast("Failed to save configuration", "error");
      setTimeout(() => setManualSaveStatus("idle"), 3000);
    }
  };

  const isModelActive = (modelId: string) =>
    config?.model === modelId || config?.model?.endsWith(`/${modelId}`);

  return (
    <div className="page-container settings-page">
      {toast && (
        <div className={`toast toast-${toast.type}`}>
          {toast.type === "success" ? <CheckCircle2 size={16} /> : <XCircle size={16} />}
          <span>{toast.message}</span>
        </div>
      )}

      <div className="page-header">
        <h1>Settings</h1>
      </div>

      {config && (
        <div className="current-model-banner">
          <Zap size={18} className="text-blue-400" />
          <div className="current-model-info">
            <span className="current-model-label">Active Model</span>
            <span className="current-model-name">{config.model}</span>
          </div>
          {config.api_url && (
            <span className="current-model-url">{config.api_url}</span>
          )}
        </div>
      )}

      <div className="settings-layout">
        <nav className="settings-nav">
          <button
            className={`settings-nav-item ${activeTab === "providers" ? "active" : ""}`}
            onClick={() => setActiveTab("providers")}
          >
            <Globe size={18} />
            <span>Providers</span>
          </button>
          <button
            className={`settings-nav-item ${activeTab === "appearance" ? "active" : ""}`}
            onClick={() => setActiveTab("appearance")}
          >
            <Palette size={18} />
            <span>Appearance</span>
          </button>
          <button
            className={`settings-nav-item ${activeTab === "api" ? "active" : ""}`}
            onClick={() => setActiveTab("api")}
          >
            <Key size={18} />
            <span>API Keys</span>
          </button>
          <button
            className={`settings-nav-item ${activeTab === "manual" ? "active" : ""}`}
            onClick={() => setActiveTab("manual")}
          >
            <Zap size={18} />
            <span>Manual Config</span>
          </button>
        </nav>

        <div className="settings-content">
          {activeTab === "providers" && (
            <div className="settings-section">
              <h2>LLM Providers</h2>
              <p className="settings-description">Switch models across providers. Green checkmark = credentials configured.</p>

              {!providersData ? (
                <div className="loading-state">
                  <Loader2 size={24} className="animate-spin" />
                  <span>Loading providers...</span>
                </div>
              ) : (
                <div className="provider-grid">
                  {providersData.providers.map(provider => {
                    const hasCredentials = providersData.credentials[provider.id] ?? false;
                    const activeModelInProvider = provider.models.find(m => isModelActive(m.id));

                    return (
                      <div
                        key={provider.id}
                        className={`provider-card ${activeModelInProvider ? "provider-card-active" : ""}`}
                      >
                        <div className="provider-card-header">
                          <div className="provider-card-title">
                            <span className="provider-name">{provider.name}</span>
                            {hasCredentials ? (
                              <CheckCircle2 size={16} className="credential-ok" />
                            ) : (
                              <XCircle size={16} className="credential-missing" />
                            )}
                          </div>
                          <span className="provider-card-url">{provider.base_url}</span>
                        </div>

                        <div className="provider-card-body">
                          <select
                            className="model-select"
                            value={activeModelInProvider?.id ?? ""}
                            onChange={e => handleModelSwitch(e.target.value)}
                            disabled={switching}
                          >
                            <option value="" disabled>
                              Select a model...
                            </option>
                            {provider.models.map(model => (
                              <option key={model.id} value={model.id}>
                                {model.name}
                                {model.context_length ? ` (${(model.context_length / 1000).toFixed(0)}k)` : ""}
                              </option>
                            ))}
                          </select>
                          {activeModelInProvider && (
                            <div className="active-badge">Active</div>
                          )}
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          )}

          {activeTab === "appearance" && (
            <div className="settings-section">
              <h2>Theme</h2>
              <p className="settings-description">Choose from 8 preset themes</p>
              <div className="theme-grid">
                {THEMES.map(t => (
                  <button
                    key={t.id}
                    className={`theme-card ${selectedTheme === t.id ? "selected" : ""}`}
                    onClick={() => handleThemeChange(t.id)}
                  >
                    <div
                      className="theme-preview"
                      style={{
                        background: t.id.endsWith("-dark") ? "#1a1a1f" : "#ffffff",
                        border: `2px solid ${t.accent}`,
                      }}
                    />
                    <div className="theme-name">{t.name}</div>
                  </button>
                ))}
              </div>
            </div>
          )}

          {activeTab === "api" && (
            <div className="settings-section">
              <h2>API Keys</h2>
              <p className="settings-description">Manage your API keys for LLM providers</p>
              <div className="api-key-form">
                <label>
                  <span>MiniMax API Key</span>
                  <input
                    type="password"
                    value={apiKey}
                    onChange={e => setApiKey(e.target.value)}
                    placeholder="sk-cp-c0SQS3..."
                  />
                </label>
                <button
                  className={`save-btn ${saveStatus}`}
                  onClick={handleSaveApiKey}
                  disabled={saveStatus === "saving"}
                >
                  {saveStatus === "saving" ? "Saving..." : saveStatus === "saved" ? "✓ Saved" : saveStatus === "error" ? "Error" : "Save"}
                </button>
              </div>
            </div>
          )}

          {activeTab === "manual" && (
            <div className="settings-section">
              <h2>Manual Configuration</h2>
              <p className="settings-description">Override model, API URL, and API key directly</p>
              <div className="manual-config-form">
                <label>
                  <span>Model</span>
                  <input
                    type="text"
                    value={manualModel}
                    onChange={e => setManualModel(e.target.value)}
                    placeholder="e.g. minimax/MiniMax-M2.7-highspeed"
                  />
                </label>
                <label>
                  <span>API URL</span>
                  <input
                    type="text"
                    value={manualApiUrl}
                    onChange={e => setManualApiUrl(e.target.value)}
                    placeholder="https://api.minimaxi.com/v1"
                  />
                </label>
                <label>
                  <span>API Key</span>
                  <input
                    type="password"
                    value={manualApiKey}
                    onChange={e => setManualApiKey(e.target.value)}
                    placeholder="sk-..."
                  />
                </label>
                <button
                  className={`save-btn ${manualSaveStatus}`}
                  onClick={handleManualSave}
                  disabled={manualSaveStatus === "saving"}
                >
                  {manualSaveStatus === "saving" ? "Saving..." : manualSaveStatus === "saved" ? "✓ Saved" : manualSaveStatus === "error" ? "Error" : "Save Configuration"}
                </button>
              </div>
            </div>
          )}
        </div>
      </div>

      <style>{`
        .toast {
          position: fixed;
          top: 20px;
          right: 20px;
          display: flex;
          align-items: center;
          gap: 8px;
          padding: 12px 20px;
          border-radius: 8px;
          font-size: 14px;
          font-weight: 500;
          z-index: 1000;
          animation: toast-in 0.3s ease-out;
        }
        .toast-success {
          background: #065f46;
          color: #6ee7b7;
          border: 1px solid #10b981;
        }
        .toast-error {
          background: #7f1d1d;
          color: #fca5a5;
          border: 1px solid #ef4444;
        }
        @keyframes toast-in {
          from { opacity: 0; transform: translateY(-10px); }
          to { opacity: 1; transform: translateY(0); }
        }

        .current-model-banner {
          display: flex;
          align-items: center;
          gap: 12px;
          padding: 12px 20px;
          margin: 0 0 20px 0;
          background: #1e293b;
          border: 1px solid #334155;
          border-radius: 10px;
        }
        .current-model-info {
          display: flex;
          flex-direction: column;
          gap: 2px;
        }
        .current-model-label {
          font-size: 11px;
          color: #94a3b8;
          text-transform: uppercase;
          letter-spacing: 0.5px;
        }
        .current-model-name {
          font-size: 15px;
          font-weight: 600;
          color: #e2e8f0;
        }
        .current-model-url {
          margin-left: auto;
          font-size: 12px;
          color: #64748b;
          font-family: monospace;
        }

        .provider-grid {
          display: grid;
          grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
          gap: 16px;
          margin-top: 16px;
        }
        .provider-card {
          background: #1e293b;
          border: 1px solid #334155;
          border-radius: 10px;
          padding: 16px;
          transition: border-color 0.2s, box-shadow 0.2s;
        }
        .provider-card:hover {
          border-color: #475569;
        }
        .provider-card-active {
          border-color: #3b82f6;
          box-shadow: 0 0 0 1px #3b82f6, 0 0 20px rgba(59, 130, 246, 0.15);
        }
        .provider-card-header {
          display: flex;
          flex-direction: column;
          gap: 4px;
          margin-bottom: 12px;
        }
        .provider-card-title {
          display: flex;
          align-items: center;
          gap: 8px;
        }
        .provider-card-title .provider-name {
          font-size: 15px;
          font-weight: 600;
          color: #e2e8f0;
        }
        .credential-ok {
          color: #22c55e;
          flex-shrink: 0;
        }
        .credential-missing {
          color: #ef4444;
          flex-shrink: 0;
        }
        .provider-card-url {
          font-size: 12px;
          color: #64748b;
          font-family: monospace;
        }
        .provider-card-body {
          display: flex;
          align-items: center;
          gap: 10px;
        }
        .model-select {
          flex: 1;
          background: #0f172a;
          border: 1px solid #334155;
          border-radius: 6px;
          color: #e2e8f0;
          padding: 8px 12px;
          font-size: 13px;
          outline: none;
          cursor: pointer;
        }
        .model-select:focus {
          border-color: #3b82f6;
        }
        .model-select:disabled {
          opacity: 0.6;
          cursor: not-allowed;
        }
        .active-badge {
          font-size: 11px;
          font-weight: 600;
          color: #3b82f6;
          background: rgba(59, 130, 246, 0.15);
          padding: 3px 8px;
          border-radius: 4px;
          white-space: nowrap;
        }

        .loading-state {
          display: flex;
          align-items: center;
          gap: 10px;
          padding: 32px;
          color: #94a3b8;
          justify-content: center;
        }

        .manual-config-form {
          display: flex;
          flex-direction: column;
          gap: 16px;
          margin-top: 16px;
        }
        .manual-config-form label {
          display: flex;
          flex-direction: column;
          gap: 6px;
        }
        .manual-config-form label span {
          font-size: 13px;
          color: #94a3b8;
          font-weight: 500;
        }
        .manual-config-form input {
          background: #0f172a;
          border: 1px solid #334155;
          border-radius: 6px;
          color: #e2e8f0;
          padding: 10px 14px;
          font-size: 14px;
          outline: none;
        }
        .manual-config-form input:focus {
          border-color: #3b82f6;
        }
      `}</style>
    </div>
  );
}
