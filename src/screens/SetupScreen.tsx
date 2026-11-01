import { useState } from "react";
import { Button } from "../components/Button";
import { Field } from "../components/Field";
import { Input } from "../components/Input";
import { api } from "../lib/api";

interface Props {
  onInitialized: () => void;
}

export function SetupScreen({ onInitialized }: Props) {
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const lengthOk = password.length >= 12;
  const matchOk = password === confirm && confirm.length > 0;
  const isValid = lengthOk && matchOk;

  const strength = getStrength(password);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!isValid || loading) return;
    setLoading(true);
    setError(null);
    try {
      await api.initializeVault(password);
      onInitialized();
    } catch (err) {
      setError(String(err));
      setLoading(false);
    }
  }

  return (
    <div className="screen-center">
      <form className="screen-card form-stack" onSubmit={handleSubmit}>
        <h1 className="screen-title">Создание хранилища</h1>
        <p className="screen-text">
          Придумайте мастер-пароль для шифрования хранилища. Восстановить его
          невозможно.
        </p>

        <Field label="Мастер-пароль" hint="Минимум 12 символов">
          <Input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            disabled={loading}
            autoFocus
          />
        </Field>

        <div className="strength-meter">
          <div className="strength-bar">
            {[1, 2, 3, 4].map((i) => (
              <div
                key={i}
                className={`strength-segment ${
                  i <= strength.score ? `filled-${strength.level}` : ""
                }`}
              />
            ))}
          </div>
          <span className="strength-text">{strength.label}</span>
        </div>

        <Field
          label="Подтверждение"
          error={!matchOk && confirm.length > 0 ? "Пароли не совпадают" : undefined}
        >
          <Input
            type="password"
            value={confirm}
            onChange={(e) => setConfirm(e.target.value)}
            disabled={loading}
          />
        </Field>

        {error && <p className="error-text">{error}</p>}

        <Button
          type="submit"
          variant="primary"
          className="form-submit"
          disabled={!isValid || loading}
        >
          {loading ? <span className="spinner" /> : "Создать хранилище"}
        </Button>
      </form>
    </div>
  );
}

function getStrength(pw: string) {
  let score = 0;
  if (pw.length >= 8) score++;
  if (pw.length >= 12) score++;
  if (pw.length >= 16) score++;
  if (/[a-z]/.test(pw) && /[A-Z]/.test(pw)) score++;
  if (/[0-9]/.test(pw)) score++;
  if (/[^A-Za-z0-9]/.test(pw)) score++;

  let segments = 0;
  if (score > 0) segments = Math.ceil((score / 6) * 4);

  if (segments <= 1) return { score: segments, level: "weak", label: "Слабый" };
  if (segments <= 3) return { score: segments, level: "medium", label: "Средний" };
  return { score: segments, level: "strong", label: "Надёжный" };
}