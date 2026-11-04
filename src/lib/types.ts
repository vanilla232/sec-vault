export interface VaultStatus {
  exists: boolean;
  unlocked: boolean;
}

export interface EntryMeta {
  id: string;
  title: string;
  username: string;
  url: string;
  note: string;
  createdAt: string;
  updatedAt: string;
}

export interface GenOptions {
  length: number;
  upper: boolean;
  lower: boolean;
  digits: boolean;
  symbols: boolean;
  excludeAmbiguous: boolean;
}