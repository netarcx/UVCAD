interface SyncButtonProps {
  onSync: () => void;
  isSyncing: boolean;
}

export default function SyncButton({ onSync, isSyncing }: SyncButtonProps) {
  return (
    <div className="sync-button-container">
      <button
        className="sync-button"
        onClick={onSync}
        disabled={isSyncing}
      >
        {isSyncing ? "Syncing..." : "Start Sync"}
      </button>
    </div>
  );
}
