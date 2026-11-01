import { useState } from "react";
import { Button } from "../components/Button";
import { Modal } from "../components/Modal";
import { Field } from "../components/Field";
import { Input } from "../components/Input";
import { api } from "../lib/api";

interface Props {
  onLock: () => void;
}

export function SettingsScreen({ onLock }: Props) {
  const [isDestroyModalOpen, setIsDestroyModalOpen] = useState(false);
  const [destroyPassword, setDestroyPassword] = useState("");
  const [destroyError, setDestroyError] = useState<string | null>(null);
  const [isDestroying, setIsDestroying] = useState(false);

  const handleDestroySubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!destroyPassword || isDestroying) return;

    setIsDestroying(true);
    setDestroyError(null);

    try {
      await api.destroyVault(destroyPassword);
      setIsDestroyModalOpen(false);
      setDestroyPassword("");
      onLock(); 
    } catch (err) {
      const msg = String(err);
      if (msg.includes("decryption failed")) {
        setDestroyError("Неверный мастер-пароль");
      } else {
        setDestroyError(msg);
      }
      setDestroyPassword("");
    } finally {
      setIsDestroying(false);
    }
  };

  const closeModal = () => {
    if (isDestroying) return;
    setIsDestroyModalOpen(false);
    setDestroyPassword("");
    setDestroyError(null);
  };

  return (
    <div className="settings-layout">
      <div>
        <h1 className="screen-title">Настройки</h1>
        <p className="screen-text">Управление безопасностью приложения.</p>
      </div>

      <div className="options-panel">
        <div className="field">
          <label className="field-label">Блокировка хранилища</label>
          <span className="field-hint" style={{ marginBottom: "16px" }}>
            Вы можете вручную заблокировать хранилище в любой момент. Все данные будут удалены из оперативной памяти.
          </span>
          <Button variant="danger" onClick={onLock} style={{ width: "100%" }}>
            Заблокировать хранилище
          </Button>
        </div>

        <div className="danger-zone">
          <label className="field-label" style={{ color: "var(--danger)" }}>Опасная зона</label>
          <span className="field-hint" style={{ marginBottom: "16px", display: "block" }}>
            Полное уничтожение хранилища. Все записи, а также сам файл с мастер-паролем будут удалены навсегда. Приложение вернется к заводским настройкам.
          </span>
          <Button variant="danger" onClick={() => setIsDestroyModalOpen(true)} style={{ width: "100%" }}>
            Удалить хранилище полностью
          </Button>
        </div>
      </div>

      <Modal
        open={isDestroyModalOpen}
        onClose={closeModal}
        title="Уничтожение хранилища"
        actions={
          <>
            <Button onClick={closeModal} disabled={isDestroying}>
              Отмена
            </Button>
            <Button
              variant="danger"
              onClick={handleDestroySubmit}
              disabled={!destroyPassword || isDestroying}
            >
              {isDestroying ? <span className="spinner" /> : "Уничтожить навсегда"}
            </Button>
          </>
        }
      >
        <form onSubmit={handleDestroySubmit} className="form-stack">
          <p className="screen-text">
            Введите мастер-пароль для подтверждения. Это действие удалит файл хранилища с диска, и его нельзя будет отменить.
          </p>
          <Field label="Мастер-пароль" error={destroyError ?? undefined}>
            <Input
              type="password"
              value={destroyPassword}
              onChange={(e) => setDestroyPassword(e.target.value)}
              disabled={isDestroying}
              autoFocus
            />
          </Field>
        </form>
      </Modal>
    </div>
  );
}