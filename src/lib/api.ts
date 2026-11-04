import { invoke } from "@tauri-apps/api/core";
import type { EntryMeta, GenOptions, VaultStatus } from "./types";

export const api = {
  vaultStatus: () => invoke<VaultStatus>("vault_status"),

  initializeVault: (masterPassword: string) =>
    invoke<void>("initialize_vault", { masterPassword }),

  unlockVault: (masterPassword: string) =>
    invoke<void>("unlock_vault", { masterPassword }),

  lockVault: () => invoke<void>("lock_vault"),

  listEntries: () => invoke<EntryMeta[]>("list_entries"),

  createEntry: (title: string, username: string, password: string, url: string, note: string) =>
    invoke<EntryMeta>("create_entry", { title, username, password, url, note }),

  updateEntry: (id: string, title?: string, username?: string, password?: string, url?: string, note?: string) =>
    invoke<EntryMeta>("update_entry", { id, title, username, password, url, note }),

  deleteEntry: (id: string) => invoke<void>("delete_entry", { id }),

  revealEntryPassword: (id: string) =>
    invoke<string>("reveal_entry_password", { id }),

  generatePassword: (opts: GenOptions) =>
    invoke<string>("generate_password", { opts }),

  copyToClipboard: (text: string, clearAfterSecs: number = 10) =>
    invoke<void>("copy_to_clipboard", { text, clearAfterSecs }),

  destroyVault: (masterPassword: string) =>
    invoke<void>("destroy_vault", { masterPassword }),
};