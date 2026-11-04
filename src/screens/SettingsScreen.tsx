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
  
  const [isImporting, setIsImporting] = useState(false);
  const [isExporting, setIsExporting] = useState(false);
  const [isCsvExportModalOpen, setIsCsvExportModalOpen] = useState(false);

  const handleImport = async () => {
    setIsImporting(true);
    try {
      const count = await api.importCsv();
      if (count > 0) {
        alert(`Успешно импортировано записей: ${count}`);
      }
    } catch (e) {
      alert(`Ошибка импорта: ${e}`);
    } finally {
      setIsImporting(false);
    }
  };

  const handleExportEncrypted = async () => {
    setIsExporting(true);
    try {
      const saved = await api.exportEncrypted();
      if (saved) alert("Зашифрованный бэкап успешно сохранен.");
    } catch (e) {
      alert(`Ошибка экспорта: ${e}`);
    } finally {
      setIsExporting(false);
    }
  };

  const handleExportCsv = async () => {
    setIsCsvExportModalOpen(false);
    setIsExporting(true);
    try {
      const saved = await api.exportCsv();
      if (saved) alert("CSV файл успешно сохранен. Помните, что он не зашифрован!");
    } catch (e) {
      alert(`Ошибка экспорта: ${e}`);
    } finally {
      setIsExporting(false);
    }
  };

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
          <label className="field-label">Импорт данных</label>
          <span className="field-hint mb-4">
            Загрузите CSV файл из Chrome, Edge или Bitwarden. Существующие записи с такими же названиями будут пропущены.
          </span>
          <Button onClick={handleImport} disabled={isImporting || isExporting} className="w-full">
            {isImporting ? "Импорт..." : "Импортировать из CSV"}
          </Button>
        </div>

        <div className="field">
          <label className="field-label">Экспорт данных</label>
          <span className="field-hint mb-4">
            Сохраните резервную копию или перенесите данные в другой менеджер.
          </span>
          <div style={{ display: "flex", gap: "8px" }}>
            <Button onClick={handleExportEncrypted} disabled={isImporting || isExporting} className="w-full">
              {isExporting ? "Сохранение..." : "Зашифрованный бэкап"}
            </Button>
            <Button onClick={() => setIsCsvExportModalOpen(true)} disabled={isImporting || isExporting} className="w-full">
              Экспорт в CSV
            </Button>
          </div>
        </div>

        <div className="field">
          <label className="field-label">Блокировка хранилища</label>
          <span className="field-hint mb-4">
            Вы можете вручную заблокировать хранилище в любой момент.
          </span>
          <Button variant="danger" onClick={onLock} className="w-full">
            Заблокировать хранилище
          </Button>
        </div>

        <div className="danger-zone">
          <label className="field-label danger-zone-label">Опасная зона</label>
          <span className="field-hint danger-zone-text">
            Полное уничтожение хранилища. Все записи будут удалены навсегда.
          </span>
          <Button variant="danger" onClick={() => setIsDestroyModalOpen(true)} className="w-full">
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
            <Button onClick={closeModal} disabled={isDestroying}>Отмена</Button>
            <Button variant="danger" onClick={handleDestroySubmit} disabled={!destroyPassword || isDestroying}>
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

      <Modal
        open={isCsvExportModalOpen}
        onClose={() => setIsCsvExportModalOpen(false)}
        title="Внимание: Опасность!"
        actions={
          <>
            <Button onClick={() => setIsCsvExportModalOpen(false)}>Отмена</Button>
            <Button variant="danger" onClick={handleExportCsv}>Я понимаю, продолжить</Button>
          </>
        }
      >
        <p className="screen-text">
          Вы собираетесь экспортировать пароли в формате CSV. 
          <br /><br />
          <strong>Этот файл не зашифрован.</strong> Любой, кто получит к нему доступ, сможет прочитать ваши пароли в открытом виде. Используйте этот формат только для немедленного переноса в другой менеджер паролей и сразу удаляйте файл после завершения.
        </p>
      </Modal>
    </div>
  );
}