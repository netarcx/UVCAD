import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { open } from "@tauri-apps/api/dialog";
import { AppConfig, AuthStatus } from "../types";

interface SettingsPanelProps {
  onClose: () => void;
}

export default function SettingsPanel({ onClose }: SettingsPanelProps) {
  const [config, setConfig] = useState<AppConfig>({
    local_path: null,
    gdrive_folder_id: null,
    smb_share_path: null,
  });
  const [authStatus, setAuthStatus] = useState<AuthStatus | null>(null);
  const [clientId, setClientId] = useState("");
  const [clientSecret, setClientSecret] = useState("");
  const [authInProgress, setAuthInProgress] = useState(false);

  useEffect(() => {
    loadConfig();
    loadAuthStatus();
  }, []);

  const loadConfig = async () => {
    try {
      const cfg = await invoke<AppConfig>("get_config");
      console.log("Loaded config from backend:", cfg);
      setConfig(cfg);
    } catch (error) {
      console.error("Failed to load config:", error);
    }
  };

  const loadAuthStatus = async () => {
    try {
      const status = await invoke<AuthStatus>("get_auth_status");
      setAuthStatus(status);
    } catch (error) {
      console.error("Failed to load auth status:", error);
    }
  };

  const handleSelectLocalPath = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
      });
      if (selected && typeof selected === "string") {
        setConfig({ ...config, local_path: selected });
      }
    } catch (error) {
      console.error("Failed to select directory:", error);
    }
  };

  const handleSave = async () => {
    try {
      console.log("Saving config:", config);
      await invoke("update_config", { config });
      console.log("Config saved successfully");
      alert("Configuration saved successfully!");
    } catch (error) {
      console.error("Failed to save config:", error);
      alert("Failed to save configuration: " + error);
    }
  };

  const handleImportCredentials = async () => {
    try {
      const selected = await open({
        filters: [
          {
            name: "JSON",
            extensions: ["json"],
          },
        ],
        multiple: false,
      });

      if (!selected || typeof selected !== "string") {
        return;
      }

      // Read the file
      const { readTextFile } = await import("@tauri-apps/api/fs");
      const content = await readTextFile(selected);

      // Parse the JSON
      const credentials = JSON.parse(content);

      // Extract client_id and client_secret
      // Google credentials JSON can have different formats
      let extractedClientId = "";
      let extractedClientSecret = "";

      if (credentials.installed) {
        extractedClientId = credentials.installed.client_id;
        extractedClientSecret = credentials.installed.client_secret;
      } else if (credentials.web) {
        extractedClientId = credentials.web.client_id;
        extractedClientSecret = credentials.web.client_secret;
      } else if (credentials.client_id && credentials.client_secret) {
        extractedClientId = credentials.client_id;
        extractedClientSecret = credentials.client_secret;
      } else {
        alert("Invalid credentials file format. Please download the correct JSON file from Google Cloud Console.");
        return;
      }

      if (!extractedClientId || !extractedClientSecret) {
        alert("Could not find client_id and client_secret in the file.");
        return;
      }

      setClientId(extractedClientId);
      setClientSecret(extractedClientSecret);
      console.log("Credentials imported successfully");
      alert("Credentials imported successfully!");

    } catch (error) {
      console.error("Failed to import credentials:", error);
      alert("Failed to import credentials: " + error);
    }
  };

  const handleGoogleAuth = async () => {
    if (!clientId || !clientSecret) {
      alert("Please enter both Client ID and Client Secret");
      return;
    }

    try {
      setAuthInProgress(true);

      // Start OAuth flow
      const message = await invoke<string>("start_google_auth", {
        clientId,
        clientSecret
      });

      console.log(message);

      // Wait for user to complete auth in browser, then complete the flow
      setTimeout(async () => {
        try {
          const result = await invoke<string>("complete_google_auth");
          alert(result);
          await loadAuthStatus();
        } catch (error) {
          console.error("Auth completion failed:", error);
          alert("Authentication failed: " + error);
        } finally {
          setAuthInProgress(false);
        }
      }, 3000); // Give user 3 seconds to authorize in browser

    } catch (error) {
      console.error("Google auth failed:", error);
      alert("Authentication failed: " + error);
      setAuthInProgress(false);
    }
  };

  const handleLogout = async () => {
    try {
      await invoke("logout");
      await loadAuthStatus();
      alert("Logged out successfully");
    } catch (error) {
      console.error("Logout failed:", error);
    }
  };

  return (
    <div className="settings-panel">
      <div className="settings-header">
        <h2>Settings</h2>
        <button onClick={onClose}>Close</button>
      </div>

      <div className="settings-content">
        <section className="settings-section">
          <h3>Local Folder</h3>
          <div className="setting-item">
            <input
              type="text"
              value={config.local_path || ""}
              placeholder="Select local folder..."
              readOnly
            />
            <button onClick={handleSelectLocalPath}>Browse</button>
          </div>
        </section>

        <section className="settings-section">
          <h3>Google Drive</h3>
          <div className="setting-item">
            <div className="auth-status">
              Status:{" "}
              {authStatus?.is_authenticated ? (
                <span className="authenticated">Connected</span>
              ) : (
                <span className="not-authenticated">Not Connected</span>
              )}
            </div>
            {authStatus?.is_authenticated && (
              <button onClick={handleLogout}>Logout</button>
            )}
          </div>

          {!authStatus?.is_authenticated && (
            <>
              <div className="setting-item">
                <button
                  onClick={handleImportCredentials}
                  className="import-button"
                  style={{ marginBottom: "1rem" }}
                >
                  📄 Import Credentials JSON
                </button>
                <span style={{ marginLeft: "1rem", fontSize: "0.9rem", color: "#666" }}>
                  Or enter manually:
                </span>
              </div>
              <div className="setting-item">
                <label>Client ID:</label>
                <input
                  type="text"
                  value={clientId}
                  onChange={(e) => setClientId(e.target.value)}
                  placeholder="Enter Google OAuth Client ID"
                />
              </div>
              <div className="setting-item">
                <label>Client Secret:</label>
                <input
                  type="password"
                  value={clientSecret}
                  onChange={(e) => setClientSecret(e.target.value)}
                  placeholder="Enter Google OAuth Client Secret"
                />
              </div>
              <div className="setting-item">
                <button
                  onClick={handleGoogleAuth}
                  disabled={authInProgress || !clientId || !clientSecret}
                >
                  {authInProgress ? "Authenticating..." : "Connect to Google Drive"}
                </button>
              </div>
              <div className="setting-help">
                <p>ℹ️ To get OAuth credentials:</p>
                <ol>
                  <li>Go to <a href="https://console.cloud.google.com/" target="_blank">Google Cloud Console</a></li>
                  <li>Create a project and enable Google Drive API</li>
                  <li>Create OAuth 2.0 credentials (Desktop app)</li>
                  <li>Set redirect URI to: http://127.0.0.1:8080/oauth/callback</li>
                  <li>Download the JSON file or copy Client ID and Client Secret</li>
                </ol>
              </div>
            </>
          )}

          <div className="setting-item">
            <label>Folder ID:</label>
            <input
              type="text"
              value={config.gdrive_folder_id || ""}
              onChange={(e) =>
                setConfig({ ...config, gdrive_folder_id: e.target.value })
              }
              placeholder="Google Drive folder ID"
            />
          </div>
        </section>

        <section className="settings-section">
          <h3>Samba Share</h3>
          <div className="setting-item">
            <label>Share Path:</label>
            <input
              type="text"
              value={config.smb_share_path || ""}
              onChange={(e) =>
                setConfig({ ...config, smb_share_path: e.target.value })
              }
              placeholder="//server/share or /Volumes/share"
            />
          </div>
        </section>

        <section className="settings-section">
          <h3>⚠️ Deletion Safety</h3>
          <div className="setting-help">
            <p>
              <strong>Automatic protection against accidental large deletions:</strong>
            </p>
            <ul style={{ marginLeft: "1.5rem", lineHeight: "1.8" }}>
              <li>
                <strong>Maximum deletion count:</strong> 50 files per sync
              </li>
              <li>
                <strong>Maximum deletion percentage:</strong> 30% of total files
              </li>
            </ul>
            <p style={{ marginTop: "1rem", fontSize: "0.9rem", color: "#666" }}>
              If a sync would exceed these thresholds, it will be automatically aborted
              to prevent accidental data loss from scenarios like:
            </p>
            <ul style={{ marginLeft: "1.5rem", lineHeight: "1.6", fontSize: "0.9rem", color: "#666" }}>
              <li>Drive temporarily unmounted or disconnected</li>
              <li>Folder accidentally emptied</li>
              <li>Permission issues making files appear deleted</li>
            </ul>
          </div>
        </section>

        <div className="settings-actions">
          <button className="save-button" onClick={handleSave}>
            Save Configuration
          </button>
        </div>
      </div>
    </div>
  );
}
