// Placeholder for future conflict resolution dialog

interface ConflictDialogProps {
  filePath: string;
  onResolve: (resolution: string) => void;
  onCancel: () => void;
}

export default function ConflictDialog({ filePath, onResolve, onCancel }: ConflictDialogProps) {
  return (
    <div className="conflict-dialog">
      <h3>File Conflict Detected</h3>
      <p>The file <strong>{filePath}</strong> has been modified in multiple locations.</p>

      <div className="conflict-options">
        <button onClick={() => onResolve("keep_local")}>Keep Local Version</button>
        <button onClick={() => onResolve("keep_gdrive")}>Keep Google Drive Version</button>
        <button onClick={() => onResolve("keep_smb")}>Keep Samba Version</button>
        <button onClick={() => onResolve("keep_both")}>Keep All (Rename)</button>
      </div>

      <button className="cancel-button" onClick={onCancel}>Cancel</button>
    </div>
  );
}
