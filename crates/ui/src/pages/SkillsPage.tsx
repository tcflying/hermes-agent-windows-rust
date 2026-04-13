import { Sparkles, Trash2, Plus, ChevronUp, BarChart3, X } from "lucide-react";
import { useState, useEffect, useCallback } from "react";

const API_BASE = "http://localhost:3848";

interface Skill {
  name: string;
  description: string;
  content: string;
  created_at: string;
}

interface GrowthPoint {
  date: string;
  count: number;
  new_skills: number;
}

export function SkillsPage() {
  const [skills, setSkills] = useState<Skill[]>([]);
  const [growth, setGrowth] = useState<GrowthPoint[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [createForm, setCreateForm] = useState({ name: "", description: "", content: "" });
  const [submitting, setSubmitting] = useState(false);

  const fetchSkills = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE}/api/skills`);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      setSkills(Array.isArray(data) ? data : (data.skills ?? []));
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to fetch skills");
    } finally {
      setLoading(false);
    }
  }, []);

  const fetchGrowth = useCallback(async () => {
    try {
      const res = await fetch(`${API_BASE}/api/skills/growth`);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = await res.json();
      setGrowth(data.skills_over_time ?? []);
    } catch {
      // Growth data is optional — don't block UI
    }
  }, []);

  useEffect(() => {
    setLoading(true);
    Promise.all([fetchSkills(), fetchGrowth()]).finally(() => setLoading(false));
  }, [fetchSkills, fetchGrowth]);

  const handleDelete = async (name: string) => {
    if (!window.confirm(`Delete skill "${name}"?`)) return;
    try {
      const res = await fetch(`${API_BASE}/api/skills/${encodeURIComponent(name)}`, { method: "DELETE" });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      setSkills(prev => prev.filter(s => s.name !== name));
    } catch (e) {
      alert(e instanceof Error ? e.message : "Delete failed");
    }
  };

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!createForm.name.trim()) return;
    setSubmitting(true);
    try {
      const res = await fetch(`${API_BASE}/api/skills/create`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(createForm),
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      setCreateForm({ name: "", description: "", content: "" });
      setShowCreateForm(false);
      await fetchSkills();
      await fetchGrowth();
    } catch (e) {
      alert(e instanceof Error ? e.message : "Create failed");
    } finally {
      setSubmitting(false);
    }
  };

  const autoCount = skills.filter(s => s.name.startsWith("auto-")).length;
  const manualCount = skills.length - autoCount;

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full bg-gray-900 text-gray-400">
        <Sparkles size={24} className="animate-pulse mr-2" /> Loading skills...
      </div>
    );
  }

  return (
    <div className="bg-gray-900 text-white min-h-screen p-6 space-y-6">
      {/* Stats Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold flex items-center gap-2">
            <Sparkles size={24} className="text-purple-400" /> Skills
          </h1>
          <div className="flex gap-4 mt-2 text-sm text-gray-400">
            <span>Total: <span className="text-white font-semibold">{skills.length}</span></span>
            <span>Auto-evolved: <span className="text-purple-400 font-semibold">{autoCount}</span></span>
            <span>Manual: <span className="text-emerald-400 font-semibold">{manualCount}</span></span>
          </div>
        </div>
        <button
          onClick={() => setShowCreateForm(v => !v)}
          className="flex items-center gap-1.5 px-4 py-2 bg-purple-600 hover:bg-purple-500 rounded-lg text-sm font-medium transition-colors"
        >
          {showCreateForm ? <ChevronUp size={16} /> : <Plus size={16} />}
          {showCreateForm ? "Close" : "New Skill"}
        </button>
      </div>

      {/* Error Banner */}
      {error && (
        <div className="bg-red-900/50 border border-red-700 rounded-lg p-3 text-red-300 text-sm flex items-center justify-between">
          <span>{error}</span>
          <button onClick={() => setError(null)}><X size={16} /></button>
        </div>
      )}

      {/* Create Form */}
      {showCreateForm && (
        <form onSubmit={handleCreate} className="bg-gray-800 rounded-xl p-5 space-y-4 border border-gray-700">
          <h2 className="text-lg font-semibold">Create Skill</h2>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <label className="block text-xs text-gray-400 mb-1">Name</label>
              <input
                type="text"
                value={createForm.name}
                onChange={e => setCreateForm(f => ({ ...f, name: e.target.value }))}
                className="w-full bg-gray-900 border border-gray-600 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-purple-500"
                placeholder="my-skill"
                required
              />
            </div>
            <div>
              <label className="block text-xs text-gray-400 mb-1">Description</label>
              <input
                type="text"
                value={createForm.description}
                onChange={e => setCreateForm(f => ({ ...f, description: e.target.value }))}
                className="w-full bg-gray-900 border border-gray-600 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-purple-500"
                placeholder="What this skill does"
              />
            </div>
          </div>
          <div>
            <label className="block text-xs text-gray-400 mb-1">Content</label>
            <textarea
              value={createForm.content}
              onChange={e => setCreateForm(f => ({ ...f, content: e.target.value }))}
              className="w-full bg-gray-900 border border-gray-600 rounded-lg px-3 py-2 text-sm h-32 resize-y focus:outline-none focus:border-purple-500"
              placeholder="Skill content / instructions..."
            />
          </div>
          <button
            type="submit"
            disabled={submitting || !createForm.name.trim()}
            className="px-5 py-2 bg-purple-600 hover:bg-purple-500 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg text-sm font-medium transition-colors"
          >
            {submitting ? "Creating..." : "Create"}
          </button>
        </form>
      )}

      {/* Growth Chart */}
      {growth.length > 0 && (
        <div className="bg-gray-800 rounded-xl p-5 border border-gray-700">
          <h2 className="text-lg font-semibold flex items-center gap-2 mb-4">
            <BarChart3 size={18} className="text-purple-400" /> Growth Over Time
          </h2>
          <div className="space-y-2">
            {growth.map(g => {
              const maxCount = Math.max(...growth.map(x => x.count), 1);
              const widthPct = Math.max((g.count / maxCount) * 100, 2);
              return (
                <div key={g.date} className="flex items-center gap-3 text-sm">
                  <span className="text-gray-400 w-20 shrink-0 text-right font-mono">{g.date}</span>
                  <div className="flex-1 bg-gray-700 rounded-full h-5 overflow-hidden">
                    <div
                      className="h-full bg-gradient-to-r from-purple-600 to-purple-400 rounded-full flex items-center px-2 text-xs font-medium"
                      style={{ width: `${widthPct}%` }}
                    >
                      {g.count}
                    </div>
                  </div>
                  {g.new_skills > 0 && (
                    <span className="text-emerald-400 text-xs shrink-0">+{g.new_skills} new</span>
                  )}
                </div>
              );
            })}
          </div>
        </div>
      )}

      {/* Skills Grid */}
      {skills.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-20 text-gray-500">
          <Sparkles size={48} className="mb-3 opacity-50" />
          <h2 className="text-lg font-medium text-gray-300">No Skills Yet</h2>
          <p className="text-sm mt-1">Create your first skill or wait for auto-evolved skills to appear.</p>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {skills.map(skill => {
            const isAuto = skill.name.startsWith("auto-");
            return (
              <div
                key={skill.name}
                className="bg-gray-800 rounded-xl border border-gray-700 p-4 flex flex-col justify-between hover:border-gray-600 transition-colors"
              >
                <div>
                  <div className="flex items-center gap-2 mb-2">
                    <Sparkles size={16} className={isAuto ? "text-purple-400" : "text-gray-400"} />
                    <h3 className="font-semibold truncate">{skill.name}</h3>
                    {isAuto && (
                      <span className="ml-auto text-[10px] font-bold uppercase tracking-wider px-2 py-0.5 rounded-full bg-purple-600/30 text-purple-300 border border-purple-500/40">
                        Auto-evolved
                      </span>
                    )}
                  </div>
                  {skill.description && (
                    <p className="text-sm text-gray-400 mb-2">{skill.description}</p>
                  )}
                  {skill.content && (
                    <p className="text-xs text-gray-500 line-clamp-2 mb-2">
                      {skill.content.slice(0, 100)}{skill.content.length > 100 ? "..." : ""}
                    </p>
                  )}
                </div>
                <div className="flex items-center justify-between mt-3 pt-3 border-t border-gray-700">
                  <span className="text-[11px] text-gray-500">
                    {skill.created_at ? new Date(skill.created_at).toLocaleDateString() : ""}
                  </span>
                  <button
                    onClick={() => handleDelete(skill.name)}
                    className="flex items-center gap-1 text-xs text-red-400 hover:text-red-300 transition-colors"
                  >
                    <Trash2 size={13} /> Delete
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
