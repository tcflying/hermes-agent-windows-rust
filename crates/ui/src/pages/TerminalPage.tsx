import React, { useEffect, useRef, useState } from "react";
import { Terminal as TerminalIcon, Send, Trash2 } from "lucide-react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import { execTerminal } from "../api";

const DEFAULT_CWD = ".";

export function TerminalPage() {
  const terminalRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const [cwd, setCwd] = useState(DEFAULT_CWD);
  const [command, setCommand] = useState("");
  const [history, setHistory] = useState<string[]>([]);
  const historyIdxRef = useRef(-1);

  useEffect(() => {
    if (!terminalRef.current || termRef.current) return;

    const term = new Terminal({
      cursorBlink: true,
      fontSize: 14,
      fontFamily: "Consolas, 'Courier New', monospace",
      theme: {
        background: "#1e1e1e",
        foreground: "#cccccc",
        cursor: "#cccccc",
        selectionBackground: "#264f78",
      },
      rows: 24,
      cols: 80,
    });

    const fitAddon = new FitAddon();
    term.loadAddon(fitAddon);
    term.open(terminalRef.current);
    fitAddon.fit();

    termRef.current = term;
    fitRef.current = fitAddon;

    term.write("Hermes Terminal\r\n");
    term.write("Type 'help' for available commands\r\n\r\n");
    term.write(`${cwd}> `);

    const handleResize = () => {
      if (fitRef.current) {
        fitRef.current.fit();
      }
    };
    window.addEventListener("resize", handleResize);

    return () => {
      window.removeEventListener("resize", handleResize);
      term.dispose();
      termRef.current = null;
    };
  }, []);

  useEffect(() => {
    if (fitRef.current) {
      fitRef.current.fit();
    }
  }, [terminalRef]);

  const executeCommand = async (cmd: string) => {
    const term = termRef.current;
    if (!term) return;

    if (!cmd.trim()) {
      term.write(`\r\n${cwd}> `);
      return;
    }

    term.write(`\r\n`);

    if (cmd === "clear") {
      term.clear();
      term.write(`${cwd}> `);
      return;
    }

    if (cmd === "help") {
      term.write("Available commands:\r\n");
      term.write("  clear  - Clear the terminal\r\n");
      term.write("  cd     - Change directory (cd <path>)\r\n");
      term.write("  pwd    - Print working directory\r\n");
      term.write("  help   - Show this help message\r\n");
      term.write(`${cwd}> `);
      return;
    }

    if (cmd.startsWith("cd ")) {
      const newPath = cmd.slice(3).trim();
      setCwd(newPath);
      term.write(`Changed directory to: ${newPath}\r\n`);
      term.write(`${newPath}> `);
      return;
    }

    if (cmd === "pwd") {
      term.write(`${cwd}\r\n${cwd}> `);
      return;
    }

    try {
      const result = await execTerminal(cmd, cwd);
      if (result.output) {
        term.write(result.output.replace(/\n/g, "\r\n"));
      }
      if (result.exit_code !== 0 && result.output.trim() === "") {
        term.write(`Command exited with code ${result.exit_code}\r\n`);
      }
    } catch (e) {
      term.write(`Error: ${e instanceof Error ? e.message : "Unknown error"}\r\n`);
    }

    term.write(`${cwd}> `);
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!command.trim()) return;

    executeCommand(command);
    setHistory((prev) => [command, ...prev].slice(0, 50));
    historyIdxRef.current = -1;
    setCommand("");
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowUp") {
      e.preventDefault();
      const newIndex = Math.min(historyIdxRef.current + 1, history.length - 1);
      if (newIndex >= 0 && history[newIndex]) {
        setCommand(history[newIndex]);
        historyIdxRef.current = newIndex;
      }
    } else if (e.key === "ArrowDown") {
      e.preventDefault();
      const newIndex = Math.max(historyIdxRef.current - 1, -1);
      setCommand(newIndex === -1 ? "" : history[newIndex] || "");
      historyIdxRef.current = newIndex;
    }
  };

  const handleClear = () => {
    const term = termRef.current;
    if (term) {
      term.clear();
      term.write(`${cwd}> `);
    }
  };

  return (
    <div className="terminal-page">
      <div className="terminal-header">
        <div className="terminal-title">
          <TerminalIcon size={14} />
          <span>Terminal</span>
          <span className="terminal-cwd">{cwd}</span>
        </div>
        <button className="terminal-clear-btn" onClick={handleClear} title="Clear">
          <Trash2 size={14} />
        </button>
      </div>
      <div className="terminal-container" ref={terminalRef} />
      <form className="terminal-input-area" onSubmit={handleSubmit}>
        <span className="terminal-prompt">{cwd}&gt;</span>
        <input
          ref={inputRef}
          type="text"
          className="terminal-input"
          value={command}
          onChange={(e) => setCommand(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Enter command..."
          autoFocus
        />
        <button type="submit" className="terminal-send-btn" disabled={!command.trim()}>
          <Send size={14} />
        </button>
      </form>
    </div>
  );
}
