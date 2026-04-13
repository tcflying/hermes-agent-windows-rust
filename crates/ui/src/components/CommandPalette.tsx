import React, { useState, useEffect, useCallback, useRef } from "react";
import { useNavigate } from "react-router-dom";
import { Search, MessageSquare, FileText, Terminal, Brain, Settings, LayoutDashboard, Plus, Moon, Sun, X } from "lucide-react";

interface Command {
  id: string;
  label: string;
  description: string;
  icon: React.ReactNode;
  action: () => void;
  shortcut?: string;
  category: string;
}

interface CommandPaletteProps {
  isOpen: boolean;
  onClose: () => void;
  onNewChat?: () => void;
  onToggleTheme?: () => void;
  isDark?: boolean;
}

export function CommandPalette({ isOpen, onClose, onNewChat, onToggleTheme, isDark = true }: CommandPaletteProps) {
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const navigate = useNavigate();

  const commands: Command[] = [
    {
      id: "new-chat",
      label: "New Chat",
      description: "Start a new conversation",
      icon: <Plus size={18} />,
      action: () => { navigate("/chat"); onNewChat?.(); onClose(); },
      shortcut: "Ctrl+N",
      category: "Chat",
    },
    {
      id: "chat",
      label: "Go to Chat",
      description: "Navigate to chat page",
      icon: <MessageSquare size={18} />,
      action: () => { navigate("/chat"); onClose(); },
      category: "Navigation",
    },
    {
      id: "files",
      label: "Go to Files",
      description: "Browse project files",
      icon: <FileText size={18} />,
      action: () => { navigate("/files"); onClose(); },
      category: "Navigation",
    },
    {
      id: "terminal",
      label: "Go to Terminal",
      description: "Open terminal",
      icon: <Terminal size={18} />,
      action: () => { navigate("/terminal"); onClose(); },
      category: "Navigation",
    },
    {
      id: "memory",
      label: "Go to Memory",
      description: "View memory and context",
      icon: <Brain size={18} />,
      action: () => { navigate("/memory"); onClose(); },
      category: "Navigation",
    },
    {
      id: "skills",
      label: "Go to Skills",
      description: "Browse available skills",
      icon: <Brain size={18} />,
      action: () => { navigate("/skills"); onClose(); },
      category: "Navigation",
    },
    {
      id: "settings",
      label: "Go to Settings",
      description: "Configure application",
      icon: <Settings size={18} />,
      action: () => { navigate("/settings"); onClose(); },
      category: "Navigation",
    },
    {
      id: "dashboard",
      label: "Go to Dashboard",
      description: "View workspace dashboard",
      icon: <LayoutDashboard size={18} />,
      action: () => { navigate("/dashboard"); onClose(); },
      category: "Navigation",
    },
    {
      id: "toggle-theme",
      label: isDark ? "Switch to Light Mode" : "Switch to Dark Mode",
      description: isDark ? "Switch to light color scheme" : "Switch to dark color scheme",
      icon: isDark ? <Sun size={18} /> : <Moon size={18} />,
      action: () => { onToggleTheme?.(); onClose(); },
      category: "Settings",
    },
  ];

  const filteredCommands = query.trim()
    ? commands.filter(
        cmd =>
          cmd.label.toLowerCase().includes(query.toLowerCase()) ||
          cmd.description.toLowerCase().includes(query.toLowerCase()) ||
          cmd.category.toLowerCase().includes(query.toLowerCase())
      )
    : commands;

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedIndex(i => Math.min(i + 1, filteredCommands.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedIndex(i => Math.max(i - 1, 0));
      } else if (e.key === "Enter") {
        e.preventDefault();
        filteredCommands[selectedIndex]?.action();
      } else if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    },
    [filteredCommands, selectedIndex, onClose]
  );

  useEffect(() => {
    if (isOpen) {
      setQuery("");
      setSelectedIndex(0);
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [isOpen]);

  useEffect(() => {
    setSelectedIndex(0);
  }, [query]);

  if (!isOpen) return null;

  const groupedCommands = filteredCommands.reduce((acc, cmd) => {
    if (!acc[cmd.category]) acc[cmd.category] = [];
    acc[cmd.category].push(cmd);
    return acc;
  }, {} as Record<string, Command[]>);

  let globalIndex = 0;

  return (
    <div className="command-palette-overlay" onClick={onClose}>
      <div className="command-palette" onClick={e => e.stopPropagation()}>
        <div className="command-palette-header">
          <Search size={18} className="command-palette-search-icon" />
          <input
            ref={inputRef}
            type="text"
            className="command-palette-input"
            placeholder="Type a command or search..."
            value={query}
            onChange={e => setQuery(e.target.value)}
            onKeyDown={handleKeyDown}
          />
          <button className="command-palette-close" onClick={onClose}>
            <X size={16} />
          </button>
        </div>
        <div className="command-palette-body">
          {filteredCommands.length === 0 ? (
            <div className="command-palette-empty">No commands found</div>
          ) : (
            Object.entries(groupedCommands).map(([category, cmds]) => (
              <div key={category} className="command-palette-group">
                <div className="command-palette-group-title">{category}</div>
                {cmds.map(cmd => {
                  const idx = globalIndex++;
                  return (
                    <div
                      key={cmd.id}
                      className={`command-palette-item ${idx === selectedIndex ? "selected" : ""}`}
                      onClick={() => cmd.action()}
                      onMouseEnter={() => setSelectedIndex(idx)}
                    >
                      <div className="command-palette-item-icon">{cmd.icon}</div>
                      <div className="command-palette-item-content">
                        <div className="command-palette-item-label">{cmd.label}</div>
                        <div className="command-palette-item-description">{cmd.description}</div>
                      </div>
                      {cmd.shortcut && (
                        <div className="command-palette-item-shortcut">{cmd.shortcut}</div>
                      )}
                    </div>
                  );
                })}
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
