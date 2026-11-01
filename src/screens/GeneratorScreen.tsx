import { useEffect, useState, useCallback } from "react";
import { Copy, Check, RefreshCw } from "lucide-react";
import { api } from "../lib/api";
import type { GenOptions } from "../lib/types";

const DEFAULT_OPTIONS: GenOptions = {
  length: 20,
  upper: true,
  lower: true,
  digits: true,
  symbols: true,
  excludeAmbiguous: true,
};

export function GeneratorScreen() {
  const [options, setOptions] = useState<GenOptions>(DEFAULT_OPTIONS);
  const [password, setPassword] = useState("");
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const generate = useCallback(async (opts: GenOptions) => {
    try {
      const pw = await api.generatePassword(opts);
      setPassword(pw);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    generate(options);
  }, [options, generate]);

  const toggleOption = (key: keyof GenOptions) => {
    setOptions((prev) => {
      const next = { ...prev, [key]: !prev[key] };
      const activeClasses = [next.upper, next.lower, next.digits, next.symbols].filter(Boolean).length;
      if (activeClasses < 2) return prev;
      return next;
    });
  };

  const handleCopy = async () => {
    if (!password) return;
    try {
      await api.copyToClipboard(password, 30);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="generator-layout">
      <div>
        <h1 className="screen-title">Генератор паролей</h1>
        <p className="screen-text">Создайте надежный пароль для вашей записи.</p>
      </div>

      <div className="password-display">
        <input type="text" readOnly value={password} className="input password-input" aria-label="Пароль" />
        <button className="icon-btn" onClick={handleCopy} title="Скопировать" disabled={!password}>
          {copied ? <Check size={20} color="var(--accent)" /> : <Copy size={20} />}
        </button>
        <button className="icon-btn" onClick={() => generate(options)} title="Сгенерировать заново">
          <RefreshCw size={20} />
        </button>
      </div>

      <div className="options-panel">
        <div className="range-group">
          <div className="range-header">
            <span className="field-label">Длина пароля</span>
            <span className="range-value">{options.length}</span>
          </div>
          <input type="range" min="12" max="64" value={options.length}
            onChange={(e) => setOptions({ ...options, length: parseInt(e.target.value, 10) })}
            className="range-slider"
          />
        </div>

        <div className="checkbox-group">
          <label className="checkbox-label">
            <input type="checkbox" checked={options.lower} onChange={() => toggleOption("lower")} />
            <span>Строчные (a-z)</span>
          </label>
          <label className="checkbox-label">
            <input type="checkbox" checked={options.upper} onChange={() => toggleOption("upper")} />
            <span>Заглавные (A-Z)</span>
          </label>
          <label className="checkbox-label">
            <input type="checkbox" checked={options.digits} onChange={() => toggleOption("digits")} />
            <span>Цифры (0-9)</span>
          </label>
          <label className="checkbox-label">
            <input type="checkbox" checked={options.symbols} onChange={() => toggleOption("symbols")} />
            <span>Спецсимволы (!@#)</span>
          </label>
        </div>

        <div style={{ borderTop: "1px solid var(--border)", paddingTop: "16px" }}>
          <label className="checkbox-label">
            <input type="checkbox" checked={options.excludeAmbiguous} onChange={() => toggleOption("excludeAmbiguous")} />
            <span>Исключить неоднозначные (0, O, l, I)</span>
          </label>
        </div>
      </div>

      {error && <p className="error-text">{error}</p>}
    </div>
  );
}