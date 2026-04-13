import { useState, useEffect, useCallback } from "react";
import Editor from "@monaco-editor/react";
import { FileText, Save, FolderOpen } from "lucide-react";
import { FileTree } from "../components/FileTree";
import { readFile, writeFile } from "../api";

const DEFAULT_ROOT = ".";

function getLanguageFromPath(path: string): string {
  const ext = path.split(".").pop()?.toLowerCase() || "";
  const langMap: Record<string, string> = {
    ts: "typescript",
    tsx: "typescript",
    js: "javascript",
    jsx: "javascript",
    py: "python",
    rs: "rust",
    go: "go",
    java: "java",
    c: "c",
    cpp: "cpp",
    cs: "csharp",
    rb: "ruby",
    php: "php",
    swift: "swift",
    kt: "kotlin",
    scala: "scala",
    md: "markdown",
    json: "json",
    yaml: "yaml",
    yml: "yaml",
    xml: "xml",
    html: "html",
    css: "css",
    scss: "scss",
    less: "less",
    sql: "sql",
    sh: "shell",
    bash: "shell",
    ps1: "powershell",
    toml: "toml",
    ini: "ini",
    conf: "ini",
    log: "plaintext",
    txt: "plaintext",
  };
  return langMap[ext] || "plaintext";
}

export function FilesPage() {
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [content, setContent] = useState("");
  const [originalContent, setOriginalContent] = useState("");
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [rootPath, setRootPath] = useState(DEFAULT_ROOT);
  const [showFolderInput, setShowFolderInput] = useState(false);
  const [folderInput, setFolderInput] = useState(DEFAULT_ROOT);

  useEffect(() => {
    if (selectedPath) {
      setLoading(true);
      setError(null);
      readFile(selectedPath)
        .then((res) => {
          setContent(res.content);
          setOriginalContent(res.content);
        })
        .catch((e) => {
          setError(e instanceof Error ? e.message : "Failed to read file");
        })
        .finally(() => setLoading(false));
    }
  }, [selectedPath]);

  const handleSave = useCallback(async () => {
    if (!selectedPath || content === originalContent) return;
    setSaving(true);
    setError(null);
    try {
      await writeFile(selectedPath, content);
      setOriginalContent(content);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to save file");
    } finally {
      setSaving(false);
    }
  }, [selectedPath, content, originalContent]);

  const handleOpenFolder = () => {
    if (folderInput.trim()) {
      setRootPath(folderInput.trim());
      setSelectedPath(null);
      setContent("");
      setShowFolderInput(false);
    }
  };

  const hasUnsavedChanges = content !== originalContent;

  return (
    <div className="files-page">
      <div className="files-sidebar">
        <div className="files-sidebar-header">
          <button
            className="files-folder-btn"
            onClick={() => setShowFolderInput(true)}
            title="Open Folder"
          >
            <FolderOpen size={14} />
          </button>
          {showFolderInput && (
            <div className="files-folder-input">
              <input
                type="text"
                value={folderInput}
                onChange={(e) => setFolderInput(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleOpenFolder()}
                placeholder="Enter folder path..."
                autoFocus
              />
              <button onClick={handleOpenFolder}>OK</button>
            </div>
          )}
        </div>
        <FileTree
          rootPath={rootPath}
          selectedPath={selectedPath}
          onFileSelect={setSelectedPath}
        />
      </div>
      <div className="files-main">
        {selectedPath ? (
          <>
            <div className="files-editor-header">
              <div className="files-editor-title">
                <FileText size={14} />
                <span>{selectedPath.split(/[/\\]/).pop()}</span>
                {hasUnsavedChanges && <span className="unsaved-indicator">●</span>}
              </div>
              <button
                className="files-save-btn"
                onClick={handleSave}
                disabled={!hasUnsavedChanges || saving}
              >
                <Save size={14} />
                {saving ? "Saving..." : "Save"}
              </button>
            </div>
            {error && <div className="files-error">{error}</div>}
            {loading ? (
              <div className="files-loading">Loading...</div>
            ) : (
              <div className="files-editor">
                <Editor
                  height="100%"
                  language={getLanguageFromPath(selectedPath)}
                  value={content}
                  onChange={(val: string | undefined) => setContent(val || "")}
                  theme="vs-dark"
                  options={{
                    minimap: { enabled: true },
                    fontSize: 14,
                    lineNumbers: "on",
                    scrollBeyondLastLine: false,
                    automaticLayout: true,
                    tabSize: 2,
                    wordWrap: "on",
                  }}
                />
              </div>
            )}
          </>
        ) : (
          <div className="files-empty">
            <FileText size={48} />
            <h2>Select a file to edit</h2>
            <p>Choose a file from the tree view or open a different folder</p>
          </div>
        )}
      </div>
    </div>
  );
}
