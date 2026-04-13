import { useState, useEffect } from "react";
import { FolderOpen, FolderClosed, File, ChevronRight, ChevronDown, RefreshCw } from "lucide-react";
import { listDir, FileEntry } from "../api";

interface FileTreeProps {
  onFileSelect: (path: string) => void;
  selectedPath: string | null;
  rootPath: string;
}

export function FileTree({ onFileSelect, selectedPath, rootPath }: FileTreeProps) {
  const [entries, setEntries] = useState<FileEntry[]>([]);
  const [expandedDirs, setExpandedDirs] = useState<Set<string>>(new Set());
  const [error, setError] = useState<string | null>(null);

  const loadDir = async (path: string) => {
    try {
      setError(null);
      const files = await listDir(path);
      setEntries(files);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to load directory");
    }
  };

  useEffect(() => {
    loadDir(rootPath);
  }, [rootPath]);

  const toggleDir = (path: string) => {
    const newExpanded = new Set(expandedDirs);
    if (newExpanded.has(path)) {
      newExpanded.delete(path);
    } else {
      newExpanded.add(path);
    }
    setExpandedDirs(newExpanded);
  };

  const renderEntry = (entry: FileEntry, depth: number = 0) => {
    const isExpanded = expandedDirs.has(entry.path);
    const isSelected = selectedPath === entry.path;
    const paddingLeft = depth * 16 + 8;

    return (
      <div key={entry.path}>
        <div
          className={`file-tree-item ${isSelected ? "selected" : ""}`}
          style={{ paddingLeft }}
          onClick={() => {
            if (entry.is_dir) {
              toggleDir(entry.path);
            } else {
              onFileSelect(entry.path);
            }
          }}
        >
          {entry.is_dir ? (
            <>
              {isExpanded ? (
                <ChevronDown size={14} />
              ) : (
                <ChevronRight size={14} />
              )}
              {isExpanded ? (
                <FolderOpen size={14} />
              ) : (
                <FolderClosed size={14} />
              )}
            </>
          ) : (
            <>
              <span style={{ width: 14 }} />
              <File size={14} />
            </>
          )}
          <span className="file-name">{entry.name}</span>
        </div>
        {entry.is_dir && isExpanded && (
          <SubDir path={entry.path} depth={depth + 1} />
        )}
      </div>
    );
  };

  function SubDir({ path, depth }: { path: string; depth: number }) {
    const [subEntries, setSubEntries] = useState<FileEntry[]>([]);
    const [loading, setLoading] = useState(false);

    useEffect(() => {
      if (expandedDirs.has(path)) {
        setLoading(true);
        listDir(path)
          .then(setSubEntries)
          .finally(() => setLoading(false));
      }
    }, [path, expandedDirs]);

    if (!expandedDirs.has(path)) return null;
    if (loading) {
      return (
        <div style={{ paddingLeft: (depth) * 16 + 24 }}>
          <span className="file-loading">Loading...</span>
        </div>
      );
    }

    return (
      <>
        {subEntries.map((entry) => renderEntry(entry, depth))}
      </>
    );
  }

  return (
    <div className="file-tree">
      <div className="file-tree-header">
        <span>Files</span>
        <button
          className="file-tree-refresh"
          onClick={() => loadDir(rootPath)}
          title="Refresh"
        >
          <RefreshCw size={14} />
        </button>
      </div>
      {error && <div className="file-tree-error">{error}</div>}
      <div className="file-tree-content">
        {entries.map((entry) => renderEntry(entry, 0))}
      </div>
    </div>
  );
}
