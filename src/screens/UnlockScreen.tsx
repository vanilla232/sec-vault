import { useState } from "react";
import { Button } from "../components/Button";
import { Field } from "../components/Field";
import { Input } from "../components/Input";
import { api } from "../lib/api";

interface Props {
  onUnlocked: () => void;
}

export function UnlockScreen({ onUnlocked }: Props) {
  const [password, setPassword] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!password || loading) return;
    setLoading(true);
    setError(null);
    try {
      await api.unlockVault(password);
      onUnlocked();
    } catch (err) {
      const msg = String(err);
      if (msg.includes("decryption failed")) {
        setError("Неверный мастер-пароль");
      } else {
        setError(msg);
      }
      setPassword("");
      await new Promise((r) => setTimeout(r, 1000));
      setLoading(false);
    }
  }

  return (
    <div className="screen-center">
      <form className="screen-card form-stack" onSubmit={handleSubmit}>
        <h1 className="screen-title">Разблокировка</h1>
        <p className="screen-text">
          Введите мастер-пароль для доступа к хранилищу.
        </p>

        <Field label="Мастер-пароль">
          <Input
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            disabled={loading}
            autoFocus
          />
        </Field>

        {error && <p className="error-text">{error}</p>}

        <Button
          type="submit"
          variant="primary"
          className="form-submit"
          disabled={!password || loading}
        >
          {loading ? <span className="spinner" /> : "Разблокировать"}
        </Button>
      </form>
    </div>
  );
}