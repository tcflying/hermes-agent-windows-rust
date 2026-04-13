import { Outlet, NavLink } from "react-router-dom";
import { MessageSquare, FolderOpen, Terminal, Brain, Sparkles, Settings, LayoutDashboard, Menu, X, Activity, Eye } from "lucide-react";
import { useState, useEffect, useCallback } from "react";
import { CommandPalette } from "./CommandPalette";

const NAV_ITEMS = [
  { path: "/chat", icon: MessageSquare, label: "Chat" },
  { path: "/files", icon: FolderOpen, label: "Files" },
  { path: "/terminal", icon: Terminal, label: "Terminal" },
  { path: "/memory", icon: Brain, label: "Memory" },
  { path: "/skills", icon: Sparkles, label: "Skills" },
  { path: "/inspector", icon: Activity, label: "Inspector" },
  { path: "/settings", icon: Settings, label: "Settings" },
  { path: "/dashboard", icon: LayoutDashboard, label: "Dashboard" },
  { path: "/hud", icon: Eye, label: "HUD" },
];

export function WorkspaceLayout() {
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);
  const [isDark, setIsDark] = useState(true);

  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if ((e.metaKey || e.ctrlKey) && e.key === "k") {
      e.preventDefault();
      setCommandPaletteOpen(true);
    }
    if (e.key === "Escape") {
      setCommandPaletteOpen(false);
    }
  }, []);

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  const toggleTheme = () => {
    setIsDark(!isDark);
    document.documentElement.setAttribute("data-theme", isDark ? "light" : "dark");
  };

  return (
    <>
      <div className="workspace-layout">
        <aside className={`sidebar ${sidebarOpen ? "open" : "collapsed"}`}>
          <div className="sidebar-header">
            <div className="sidebar-title">
              <span className="logo">☤</span>
              {sidebarOpen && <span>Hermes</span>}
            </div>
            <button className="sidebar-toggle" onClick={() => setSidebarOpen(!sidebarOpen)}>
              {sidebarOpen ? <X size={18} /> : <Menu size={18} />}
            </button>
          </div>
          
          <nav className="sidebar-nav">
            {NAV_ITEMS.map(({ path, icon: Icon, label }) => (
              <NavLink
                key={path}
                to={path}
                className={({ isActive }) => `nav-item ${isActive ? "active" : ""}`}
              >
                <Icon size={20} />
                {sidebarOpen && <span>{label}</span>}
              </NavLink>
            ))}
          </nav>

          <div className="sidebar-shortcut-hint" onClick={() => setCommandPaletteOpen(true)}>
            {sidebarOpen ? (
              <span>Press <kbd>Ctrl</kbd>+<kbd>K</kbd> for commands</span>
            ) : (
              <span>⌘K</span>
            )}
          </div>

          {sidebarOpen && (
            <div className="sidebar-footer">
              <div className="sidebar-version">v0.1.0</div>
            </div>
          )}
        </aside>

        <main className="workspace-main">
          <Outlet />
        </main>
      </div>

      <CommandPalette
        isOpen={commandPaletteOpen}
        onClose={() => setCommandPaletteOpen(false)}
        onToggleTheme={toggleTheme}
        isDark={isDark}
      />
    </>
  );
}
