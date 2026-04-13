import { Brain, RefreshCw } from "lucide-react";
import { useState, useEffect, useCallback } from "react";
import { listDir, readFile, writeFile } from "../api";

function MemoryFileTree({ path, onSelect }: { path: string; onSelect: (path: string) => void }) {
  const [entries, setEntries] = useState<{ name: string; path: string; is_dir: boolean }[]>([]);
  const [expanded, setExpanded] = useState<Set<string>>(new Set());

  const load = useCallback(async (dirPath: string) => {
    try {
      const ents = await listDir(dirPath);
      setEntries(ents);
    } catch { }
  }, []);

  useEffect(() => { load(path); }, [path, load]);

  const handleRefresh = () => load(path);

  return (
    <div className="memory-tree">
      <div className="memory-tree-header">
        <span className="tree-title">Memory Files</span>
        <button className="tree-refresh" onClick={handleRefresh} title="Refresh">
          <RefreshCw size={14} />
        </button>
      </div>
      {entries.map(entry => (
        <div
          key={entry.path}
          className={`tree-item ${entry.is_dir ? "folder" : "file"}`}
          onClick={() => {
            if (entry.is_dir) {
              const next = new Set(expanded);
              if (next.has(entry.path)) next.delete(entry.path);
              else next.add(entry.path);
              setExpanded(next);
            } else {
              onSelect(entry.path);
            }
          }}
        >
          <span className="tree-icon">{entry.is_dir ? "📁" : "📄"}</span>
          <span className="tree-name">{entry.name}</span>
        </div>
      ))}
      {entries.length === 0 && <div className="tree-empty">No memory files</div>}
    </div>
  );
}

export function MemoryPage() {
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [content, setContent] = useState("");
  const [isDirty, setIsDirty] = useState(false);
  const [saveStatus, setSaveStatus] = useState<"idle" | "saving" | "saved" | "error">("idle");

  const memoryRoot = ".";

  const handleSelect = async (path: string) => {
    if (isDirty && selectedPath) {
      const confirm = window.confirm("You have unsaved changes. Discard?");
      if (!confirm) return;
    }
    setSelectedPath(path);
    try {
      const data = await readFile(path);
      setContent(data.content);
      setIsDirty(false);
    } catch {
      setContent("[Error loading file]");
    }
  };

  const handleSave = async () => {
    if (!selectedPath) return;
    setSaveStatus("saving");
    try {
      await writeFile(selectedPath, content);
      setIsDirty(false);
      setSaveStatus("saved");
      setTimeout(() => setSaveStatus("idle"), 2000);
    } catch {
      setSaveStatus("error");
      setTimeout(() => setSaveStatus("idle"), 3000);
    }
  };

  return (
    <div className="page-container memory-page">
      <div className="page-header">
        <h1>Memory</h1>
        {selectedPath && (
          <div className="page-actions">
            <span className="file-name">{selectedPath.split(/[/\\]/).pop()}</span>
            <button
              className={`save-btn ${saveStatus}`}
              onClick={handleSave}
              disabled={saveStatus === "saving" || !isDirty}
            >
              {saveStatus === "saving" ? "Saving..." : saveStatus === "saved" ? "✓ Saved" : saveStatus === "error" ? "Error" : isDirty ? "Save" : "Saved"}
            </button>
          </div>
        )}
      </div>
      <div className="memory-layout">
        <aside className="memory-sidebar">
          <MemoryFileTree path={memoryRoot} onSelect={handleSelect} />
        </aside>
        <main className="memory-editor">
          {selectedPath ? (
            <textarea
              className="memory-textarea"
              value={content}
              onChange={e => { setContent(e.target.value); setIsDirty(true); }}
              placeholder="Select a memory file to view..."
            />
          ) : (
            <div className="memory-placeholder">
              <Brain size={48} />
              <h2>Memory Browser</h2>
              <p>Browse and edit persistent memory files</p>
              <p className="placeholder-note">Select a file from the sidebar</p>
            </div>
          )}
        </main>
      </div>
    </div>
  );
}
