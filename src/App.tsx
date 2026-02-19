import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import SyncButton from "./components/SyncButton";
import FileList from "./components/FileList";
import SettingsPanel from "./components/SettingsPanel";
import { SyncStatus, FileInfo } from "./types";

interface SyncProgress {
  current_file: string;
  total_files: number;
  processed_files: number;
  operation: string;
  percentage: number;
}

function App() {
  const [syncStatus, setSyncStatus] = useState<SyncStatus | null>(null);
  const [files, setFiles] = useState<FileInfo[]>([]);
  const [showSettings, setShowSettings] = useState(false);
  const [syncProgress, setSyncProgress] = useState<SyncProgress | null>(null);
  const [isSyncing, setIsSyncing] = useState(false);

  useEffect(() => {
    loadSyncStatus();
    loadFiles();

    // Listen for sync progress events
    const unlisten = listen<SyncProgress>("sync-progress", (event) => {
      console.log("Sync progress:", event.payload);
      setSyncProgress(event.payload);

      // Clear progress after completion
      if (event.payload.operation === "completed") {
        setTimeout(() => setSyncProgress(null), 3000);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const loadSyncStatus = async () => {
    try {
      const status = await invoke<SyncStatus>("get_sync_status");
      setSyncStatus(status);
    } catch (error) {
      console.error("Failed to load sync status:", error);
    }
  };

  const loadFiles = async () => {
    try {
      const fileList = await invoke<FileInfo[]>("get_file_list");
      setFiles(fileList);
    } catch (error) {
      console.error("Failed to load files:", error);
    }
  };

  const handlePull = async () => {
    setIsSyncing(true);
    try {
      await invoke("pull_from_gdrive");
      await loadSyncStatus();
      await loadFiles();
    } catch (error) {
      console.error("Pull failed:", error);
      const errorMessage = typeof error === 'string' ? error : String(error);
      alert(`Pull failed: ${errorMessage}`);
    } finally {
      setIsSyncing(false);
    }
  };

  const handleSync = async () => {
    setIsSyncing(true);
    try {
      await invoke("start_sync");
      await loadSyncStatus();
      await loadFiles();
    } catch (error) {
      console.error("Sync failed:", error);

      // Display error to user
      const errorMessage = typeof error === 'string' ? error : String(error);

      // Check if it's a deletion safety error
      if (errorMessage.includes("SAFETY CHECK FAILED")) {
        alert(`Sync Aborted - Safety Check Failed\n\n${errorMessage}\n\nNo changes were made to your files.`);
      } else {
        alert(`Sync failed: ${errorMessage}`);
      }
    } finally {
      setIsSyncing(false);
    }
  };

  return (
    <div className="app">
      <header className="app-header">
        <h1>UVCAD - CAD File Sync</h1>
        <button
          className="settings-btn"
          onClick={() => setShowSettings(!showSettings)}
        >
          Settings
        </button>
      </header>

      <main className="app-main">
        {showSettings ? (
          <SettingsPanel onClose={() => setShowSettings(false)} />
        ) : (
          <>
            <div className="status-bar">
              {syncStatus && (
                <>
                  <div className="status-item">
                    <span className="label">Status:</span>
                    <span className={isSyncing ? "syncing" : "idle"}>
                      {isSyncing ? "Syncing..." : "Idle"}
                    </span>
                  </div>
                  <div className="status-item">
                    <span className="label">Files Synced:</span>
                    <span>{syncStatus.files_synced}</span>
                  </div>
                  <div className="status-item">
                    <span className="label">Pending:</span>
                    <span>{syncStatus.files_pending}</span>
                  </div>
                  <div className="status-item">
                    <span className="label">Conflicts:</span>
                    <span className={syncStatus.conflicts > 0 ? "warning" : ""}>
                      {syncStatus.conflicts}
                    </span>
                  </div>
                </>
              )}
            </div>

            {syncProgress && (
              <div className="sync-progress-container">
                <div className="progress-info">
                  <span className="progress-file">{syncProgress.current_file}</span>
                  <span className="progress-stats">
                    {syncProgress.processed_files} / {syncProgress.total_files} files
                  </span>
                </div>
                <div className="progress-bar-container">
                  <div
                    className="progress-bar"
                    style={{ width: `${syncProgress.percentage}%` }}
                  >
                    <span className="progress-text">{syncProgress.percentage.toFixed(0)}%</span>
                  </div>
                </div>
                <div className="progress-operation">{syncProgress.operation}</div>
              </div>
            )}

            <div className="action-buttons">
              <button
                onClick={handlePull}
                disabled={isSyncing}
                className="pull-btn"
              >
                {isSyncing ? "Working..." : "Pull from Drive"}
              </button>
              <SyncButton onSync={handleSync} isSyncing={isSyncing} />
            </div>

            <FileList files={files} />
          </>
        )}
      </main>

      <footer className="app-footer">
        <p>UVCAD v0.2.2 - CAD File Synchronization Tool</p>
      </footer>
    </div>
  );
}

export default App;
