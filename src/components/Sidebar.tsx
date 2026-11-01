import { Settings, Vault, Wand2, type LucideIcon } from "lucide-react";
import { Tooltip } from "./Tooltip";

export type Tab = "vault" | "generator" | "settings";

interface Props {
  active: Tab;
  onChange: (tab: Tab) => void;
}

const tabs: { key: Tab; label: string; Icon: LucideIcon }[] = [
  { key: "vault", label: "Хранилище", Icon: Vault },
  { key: "generator", label: "Генератор", Icon: Wand2 },
  { key: "settings", label: "Настройки", Icon: Settings },
];

export function Sidebar({ active, onChange }: Props) {
  return (
    <aside className="sidebar">
      {tabs.map(({ key, label, Icon }) => (
        <Tooltip key={key} label={label}>
          <button
            type="button"
            className={`sidebar-btn${active === key ? " active" : ""}`}
            onClick={() => onChange(key)}
            aria-label={label}
          >
            <Icon size={24} strokeWidth={1.75} />
          </button>
        </Tooltip>
      ))}
    </aside>
  );
}