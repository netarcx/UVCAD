import { FileInfo } from "../types";

interface FileListProps {
  files: FileInfo[];
}

export default function FileList({ files }: FileListProps) {
  return (
    <div className="file-list">
      <h2>Files</h2>
      {files.length === 0 ? (
        <p className="no-files">No files to display</p>
      ) : (
        <table>
          <thead>
            <tr>
              <th>Path</th>
              <th>Size</th>
              <th>Modified</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            {files.map((file, index) => (
              <tr key={index}>
                <td>{file.path}</td>
                <td>{formatSize(file.size)}</td>
                <td>{new Date(file.modified).toLocaleString()}</td>
                <td>
                  <span className={`status-badge status-${file.status}`}>
                    {file.status}
                  </span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}

function formatSize(bytes: number): string {
  const units = ["B", "KB", "MB", "GB"];
  let size = bytes;
  let unitIndex = 0;

  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex++;
  }

  return `${size.toFixed(2)} ${units[unitIndex]}`;
}
