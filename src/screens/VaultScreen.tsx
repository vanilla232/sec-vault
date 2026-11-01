import { useCallback, useEffect, useRef, useState } from "react";
import { Check, Copy, Eye, EyeOff, Plus, RefreshCw, Trash2, X } from "lucide-react";
import { api } from "../lib/api";
import type { EntryMeta } from "../lib/types";

export function VaultScreen() {
  const [entries, setEntries] = useState<EntryMeta[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [isCreating, setIsCreating] = useState(false);
  const [newTitle, setNewTitle] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [createError, setCreateError] = useState<string | null>(null);
  const [revealedId, setRevealedId] = useState<string | null>(null);
  const [revealedValue, setRevealedValue] = useState<string>("");
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const revealTimerRef = useRef<number | null>(null);
  const copyTimerRef = useRef<number | null>(null);

  const loadEntries = useCallback(async () => {
    try {
      setLoading(true);
      const data = await api.listEntries();
      setEntries(data);
      setError(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { loadEntries(); }, [loadEntries]);

  useEffect(() => {
    return () => {
      if (revealTimerRef.current) window.clearTimeout(revealTimerRef.current);
      if (copyTimerRef.current) window.clearTimeout(copyTimerRef.current);
    };
  }, []);

  const handleReveal = async (id: string) => {
    if (revealedId === id) {
      setRevealedId(null);
      setRevealedValue("");
      if (revealTimerRef.current) window.clearTimeout(revealTimerRef.current);
      return;
    }
    try {
      const pw = await api.revealEntryPassword(id);
      setRevealedId(id);
      setRevealedValue(pw);
      if (revealTimerRef.current) window.clearTimeout(revealTimerRef.current);
      revealTimerRef.current = window.setTimeout(() => {
        setRevealedId(null);
        setRevealedValue("");
      }, 15000);
    } catch (e) {
      setError(String(e));
    }
  };

  const handleCopy = async (id: string) => {
    try {
      const pw = await api.revealEntryPassword(id);
      await api.copyToClipboard(pw, 30);
      setCopiedId(id);
      if (copyTimerRef.current) window.clearTimeout(copyTimerRef.current);
      copyTimerRef.current = window.setTimeout(() => setCopiedId(null), 1500);
    } catch (e) {
      setError(String(e));
    }
  };

  const handleCreate = async () => {
    if (!newTitle.trim() || !newPassword) return;
    try {
      setCreateError(null);
      await api.createEntry(newTitle, newPassword);
      setNewTitle("");
      setNewPassword("");
      setIsCreating(false);
      await loadEntries();
    } catch (e) {
      setCreateError(String(e));
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await api.deleteEntry(id);
      setDeletingId(null);
      await loadEntries();
    } catch (e) {
      setError(String(e));
    }
  };

  const generateNewPassword = async () => {
    try {
      const pw = await api.generatePassword({
        length: 20, upper: true, lower: true, digits: true, symbols: true, excludeAmbiguous: true,
      });
      setNewPassword(pw);
    } catch (e) {
      setCreateError(String(e));
    }
  };

  const filtered = entries.filter((e) => e.title.toLowerCase().includes(search.toLowerCase()));

  if (loading) {
    return <div className="screen-center"><p className="screen-text">Загрузка...</p></div>;
  }

  return (
    <div className="vault-layout">
      <div className="vault-header">
        <div className="vault-header-left">
          <h1 className="screen-title" style={{ marginRight: "16px" }}>Хранилище</h1>
        </div>
        <div style={{ display: "flex", gap: "8px" }}>
          <input type="text" className="input search-input" placeholder="Поиск..." value={search} onChange={(e) => setSearch(e.target.value)} />
          <button className="primary icon-btn" onClick={() => setIsCreating(true)} title="Создать запись" disabled={isCreating}>
            <Plus size={20} />
          </button>
        </div>
      </div>

      {error && <p className="error-text">{error}</p>}

      {isCreating && (
        <div className="vault-create-row">
          <input className="input vault-create-title" placeholder="Название" value={newTitle} onChange={(e) => setNewTitle(e.target.value)} autoFocus />
          <div style={{ display: "flex", flex: 1, gap: "8px" }}>
            <input className="input vault-create-password" placeholder="Пароль" value={newPassword} onChange={(e) => setNewPassword(e.target.value)} />
            <button className="icon-btn" onClick={generateNewPassword} title="Сгенерировать" style={{ flexShrink: 0 }}><RefreshCw size={18} /></button>
          </div>
          <div style={{ display: "flex", gap: "4px", flexShrink: 0 }}>
            <button className="primary icon-btn" onClick={handleCreate} title="Сохранить" disabled={!newTitle.trim() || !newPassword}><Check size={18} /></button>
            <button className="icon-btn" onClick={() => { setIsCreating(false); setNewTitle(""); setNewPassword(""); setCreateError(null); }} title="Отмена"><X size={18} /></button>
          </div>
          {createError && <p className="error-text" style={{ width: "100%", marginTop: "8px" }}>{createError}</p>}
        </div>
      )}

      <div className="vault-list">
        {filtered.length === 0 && !isCreating ? (
          <div className="empty-state"><p>Нет записей. Нажмите +, чтобы создать первую.</p></div>
        ) : (
          filtered.map((entry) => (
            <div key={entry.id} className="vault-row">
              <span className="vault-title">{entry.title}</span>
              <span className="vault-password">{revealedId === entry.id ? revealedValue : "••••••••••••"}</span>
              <div className="vault-actions">
                <button className="icon-btn" onClick={() => handleReveal(entry.id)} title={revealedId === entry.id ? "Скрыть" : "Показать"}>
                  {revealedId === entry.id ? <EyeOff size={18} /> : <Eye size={18} />}
                </button>
                <button className="icon-btn" onClick={() => handleCopy(entry.id)} title="Копировать">
                  {copiedId === entry.id ? <Check size={18} color="var(--accent)" /> : <Copy size={18} />}
                </button>
                {deletingId === entry.id ? (
                  <div className="delete-confirm">
                    <span style={{ marginRight: "4px" }}>Удалить?</span>
                    <button className="danger icon-btn" style={{ padding: "4px 8px" }} onClick={() => handleDelete(entry.id)}>Да</button>
                    <button className="icon-btn" style={{ padding: "4px 8px" }} onClick={() => setDeletingId(null)}>Нет</button>
                  </div>
                ) : (
                  <button className="icon-btn" onClick={() => setDeletingId(entry.id)} title="Удалить"><Trash2 size={18} /></button>
                )}
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}