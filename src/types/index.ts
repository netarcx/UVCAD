export interface SyncStatus {
  is_syncing: boolean;
  last_sync: string | null;
  files_synced: number;
  files_pending: number;
  conflicts: number;
}

export interface FileInfo {
  path: string;
  size: number;
  modified: string;
  status: string;
}

export interface AppConfig {
  local_path: string | null;
  gdrive_folder_id: string | null;
  smb_share_path: string | null;
}

export interface AuthStatus {
  is_authenticated: boolean;
  provider: string;
  email: string | null;
}
