import { useCallback, useEffect, useState } from "react";
import { api } from "./lib/api";
import type { VaultStatus } from "./lib/types";
import { Sidebar, type Tab } from "./components/Sidebar";
import { SetupScreen } from "./screens/SetupScreen";
import { UnlockScreen } from "./screens/UnlockScreen";
import { VaultScreen } from "./screens/VaultScreen";
import { GeneratorScreen } from "./screens/GeneratorScreen";
import { SettingsScreen } from "./screens/SettingsScreen";

export default function App() {
  const [status, setStatus] = useState<VaultStatus | null>(null);
  const [tab, setTab] = useState<Tab>("vault");
  const [error, setError] = useState<string | null>(null);

  const refreshStatus = useCallback(async () => {
    try {
      const s = await api.vaultStatus();
      setStatus(s);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const handleLock = useCallback(async () => {
    try {
      await api.lockVault();
      await refreshStatus();
      setTab("vault");
    } catch (e) {
      setError(String(e));
    }
  }, [refreshStatus]);

  useEffect(() => {
    refreshStatus();
  }, [refreshStatus]);

  if (error) {
    return (
      <div className="screen-center">
        <div className="screen-card">
          <h1 className="screen-title">Ошибка</h1>
          <p className="screen-text text-danger">{error}</p>
        </div>
      </div>
    );
  }

  if (!status) {
    return (
      <div className="screen-center">
        <p className="screen-text">Загрузка...</p>
      </div>
    );
  }

  if (!status.exists) return <SetupScreen onInitialized={refreshStatus} />;
  if (!status.unlocked) return <UnlockScreen onUnlocked={refreshStatus} />;

  return (
    <div className="app-shell">
      <Sidebar active={tab} onChange={setTab} />
      <div className="app-content">
        {tab === "vault" ? <VaultScreen /> : null}
        {tab === "generator" ? <GeneratorScreen /> : null}
        {tab === "settings" ? <SettingsScreen onLock={handleLock} /> : null}
      </div>
    </div>
  );
}