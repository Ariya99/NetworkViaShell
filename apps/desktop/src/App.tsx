import { useEffect, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";

type TunnelProfile = {
  host: string;
  port: number;
  username: string;
  localSocksPort: number;
  identityFile: string | null;
};

type SavedProfile = {
  name: string;
  profile: TunnelProfile;
};

type HostKeyInfo = {
  host: string;
  port: number;
  keyType: string;
  fingerprint: string;
  rawEntry: string;
};

type TunnelHealth =
  | "idle"
  | "connecting"
  | "connected"
  | "connectedInternetFailed"
  | "reconnecting"
  | "stopped";

const emptyProfile = (): TunnelProfile => ({
  host: "",
  port: 22,
  username: "",
  localSocksPort: 1080,
  identityFile: null,
});

function App() {
  const [profile, setProfile] = useState<TunnelProfile>(emptyProfile);
  const [profiles, setProfiles] = useState<SavedProfile[]>([]);
  const [profileName, setProfileName] = useState("");
  const [selectedName, setSelectedName] = useState("");
  const [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [rememberPassword, setRememberPassword] = useState(false);

  const [status, setStatus] = useState<"idle" | "connecting" | "connected" | "stopping">("idle");
  const [health, setHealth] = useState<TunnelHealth>("idle");
  const [endpoint, setEndpoint] = useState("");
  const [error, setError] = useState("");

  // Host Key TOFU state
  const [pendingHostKey, setPendingHostKey] = useState<HostKeyInfo | null>(null);
  const [probingHostKey, setProbingHostKey] = useState(false);

  // Antigravity launcher state
  const [customAntigravityPath, setCustomAntigravityPath] = useState("");
  const [showCustomPath, setShowCustomPath] = useState(false);
  const [launcherFeedback, setLauncherFeedback] = useState<{ message: string; isError: boolean } | null>(null);
  const [launchingAntigravity, setLaunchingAntigravity] = useState(false);

  async function refreshProfiles() {
    try {
      setProfiles(await invoke<SavedProfile[]>("list_saved_profiles"));
    } catch (cause) {
      setError(String(cause));
    }
  }

  useEffect(() => {
    void refreshProfiles();
  }, []);

  // Poll health when connected
  useEffect(() => {
    if (status !== "connected") return;
    const interval = setInterval(async () => {
      try {
        const h = await invoke<TunnelHealth>("get_tunnel_health");
        setHealth(h);
        if (h === "stopped") {
          setStatus("idle");
          setEndpoint("");
        }
      } catch {
        // Ignore health poll errors
      }
    }, 5000);
    return () => clearInterval(interval);
  }, [status]);

  function update<K extends keyof TunnelProfile>(key: K, value: TunnelProfile[K]) {
    setProfile(current => ({ ...current, [key]: value }));
  }

  async function selectProfile(name: string) {
    setSelectedName(name);
    const saved = profiles.find(item => item.name === name);
    if (saved) {
      setProfile(saved.profile);
      setProfileName(saved.name);
      setError("");

      // Retrieve saved credential from Windows Credential Manager if present
      try {
        const savedPwd = await invoke<string | null>("get_saved_password", { profileName: saved.name });
        if (savedPwd) {
          setPassword(savedPwd);
          setRememberPassword(true);
        } else {
          setPassword("");
          setRememberPassword(false);
        }
      } catch {
        setPassword("");
        setRememberPassword(false);
      }
    } else {
      setProfile(emptyProfile());
      setProfileName("");
      setPassword("");
      setRememberPassword(false);
    }
  }

  async function saveProfile() {
    setError("");
    try {
      await invoke("save_profile", { name: profileName, profile });
      setSelectedName(profileName.trim());
      await refreshProfiles();
    } catch (cause) {
      setError(String(cause));
    }
  }

  async function deleteProfile() {
    if (!selectedName) return;
    setError("");
    try {
      await invoke("delete_profile", { name: selectedName });
      setSelectedName("");
      setProfileName("");
      setProfile(emptyProfile());
      setPassword("");
      setRememberPassword(false);
      await refreshProfiles();
    } catch (cause) {
      setError(String(cause));
    }
  }

  async function startTunnel() {
    setError("");
    setStatus("connecting");
    setHealth("connecting");
    try {
      const address = await invoke<string>("start_ssh_tunnel", {
        profile,
        password: password.trim() ? password : null,
        rememberPassword,
        profileName: profileName.trim() ? profileName.trim() : null,
      });

      // Clear password field from frontend state if not remembered
      if (!rememberPassword) {
        setPassword("");
      }

      setEndpoint(address);
      setStatus("connected");
      setHealth("connected");
    } catch (cause) {
      const msg = String(cause);
      if (msg.startsWith("HOST_UNVERIFIED:")) {
        const parts = msg.split(":");
        const host = parts[1];
        const port = Number(parts[2]);
        setProbingHostKey(true);
        setStatus("idle");
        try {
          const keyInfo = await invoke<HostKeyInfo>("probe_host", { host, port });
          setPendingHostKey(keyInfo);
        } catch (probeErr) {
          setError(`Host key verification required, but probe failed: ${probeErr}`);
        } finally {
          setProbingHostKey(false);
        }
      } else {
        setStatus("idle");
        setHealth("idle");
        setError(msg);
      }
    }
  }

  async function connect(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    await startTunnel();
  }

  async function approveAndConnect() {
    if (!pendingHostKey) return;
    try {
      await invoke("approve_host", { rawEntry: pendingHostKey.rawEntry });
      setPendingHostKey(null);
      await startTunnel();
    } catch (cause) {
      setError(`Failed to save host key: ${cause}`);
      setPendingHostKey(null);
    }
  }

  async function disconnect() {
    setStatus("stopping");
    setError("");
    try {
      await invoke("stop_ssh_tunnel");
      setEndpoint("");
      setStatus("idle");
      setHealth("stopped");
    } catch (cause) {
      setStatus("connected");
      setError(String(cause));
    }
  }

  async function launchAntigravity() {
    setLauncherFeedback(null);
    setLaunchingAntigravity(true);
    try {
      await invoke("launch_antigravity", {
        socksPort: profile.localSocksPort,
        customPath: customAntigravityPath.trim() ? customAntigravityPath.trim() : null,
      });
      setLauncherFeedback({
        message: "Antigravity launched successfully through SOCKS5 tunnel with local loopback bypass.",
        isError: false,
      });
    } catch (cause) {
      setLauncherFeedback({
        message: String(cause),
        isError: true,
      });
    } finally {
      setLaunchingAntigravity(false);
    }
  }

  const busy = status === "connecting" || status === "stopping" || probingHostKey;
  const connected = status === "connected";

  const healthBadge = () => {
    if (!connected) return null;
    if (health === "connectedInternetFailed") {
      return <span className="health-badge warning">Internet Check Failed</span>;
    }
    if (health === "reconnecting") {
      return <span className="health-badge warning">Reconnecting…</span>;
    }
    return <span className="health-badge success">Verified & Active</span>;
  };

  return (
    <main className="shell">
      <section className="workspace" aria-labelledby="app-title">
        <header className="topbar">
          <div className="brand">
            <div className="brand-mark" aria-hidden="true">N</div>
            <div>
              <p className="eyebrow">SECURE DESKTOP TUNNELLING</p>
              <h1 id="app-title">NetworkViaShell</h1>
            </div>
          </div>
          <div className="status-container">
            {healthBadge()}
            <div className={"status status-" + status}>
              <span className="status-dot" />
              {connected ? "Connected" : status === "connecting" ? "Connecting" : status === "stopping" ? "Stopping" : "Not connected"}
            </div>
          </div>
        </header>

        <div className="content">
          {/* TOFU Host-Key Approval Modal */}
          {pendingHostKey && (
            <div className="modal-backdrop">
              <div className="modal">
                <h3>Trust New Host Key?</h3>
                <p>
                  This is the first time connecting to <strong>{pendingHostKey.host}:{pendingHostKey.port}</strong>.
                  Verify the server's cryptographic fingerprint before proceeding:
                </p>
                <div className="fingerprint-box">
                  <div><strong>Key Type:</strong> {pendingHostKey.keyType}</div>
                  <div><strong>Fingerprint:</strong> <code>{pendingHostKey.fingerprint}</code></div>
                </div>
                <div className="modal-actions">
                  <button className="button button-primary" onClick={approveAndConnect}>
                    Approve & Connect
                  </button>
                  <button className="button" onClick={() => setPendingHostKey(null)}>
                    Cancel
                  </button>
                </div>
              </div>
            </div>
          )}

          <div className="intro">
            <p className="eyebrow">OPENSSH TUNNEL ENGINE</p>
            <h2>SSH SOCKS5 Gateway</h2>
            <p>Establish a private SOCKS5 proxy and launch applications with isolated loopback bypass.</p>
          </div>

          <div className="field field-wide">
            <label htmlFor="saved-profile">Saved connections</label>
            <select
              id="saved-profile"
              value={selectedName}
              onChange={e => selectProfile(e.target.value)}
              disabled={busy || connected}
            >
              <option value="">New connection…</option>
              {profiles.map(item => (
                <option key={item.name} value={item.name}>
                  {item.name} · {item.profile.username}@{item.profile.host}
                </option>
              ))}
            </select>
          </div>

          <form className="profile-form" onSubmit={connect}>
            <div className="field field-wide">
              <label htmlFor="host">SSH host</label>
              <input
                id="host"
                autoComplete="off"
                placeholder="server.example.com"
                value={profile.host}
                onChange={e => update("host", e.target.value)}
                required
                disabled={busy || connected}
              />
            </div>

            <div className="field">
              <label htmlFor="username">Username</label>
              <input
                id="username"
                autoComplete="username"
                placeholder="user"
                value={profile.username}
                onChange={e => update("username", e.target.value)}
                required
                disabled={busy || connected}
              />
            </div>

            <div className="field">
              <label htmlFor="port">SSH port</label>
              <input
                id="port"
                inputMode="numeric"
                type="number"
                min="1"
                max="65535"
                value={profile.port}
                onChange={e => update("port", Number(e.target.value))}
                required
                disabled={busy || connected}
              />
            </div>

            {/* Password input with show/hide and remember option */}
            <div className="field field-wide">
              <label htmlFor="password">
                Password <span>(or leave empty if using SSH key / agent)</span>
              </label>
              <div className="input-with-button">
                <input
                  id="password"
                  type={showPassword ? "text" : "password"}
                  autoComplete="current-password"
                  placeholder="SSH account password"
                  value={password}
                  onChange={e => setPassword(e.target.value)}
                  disabled={busy || connected}
                />
                <button
                  type="button"
                  className="button button-secondary"
                  onClick={() => setShowPassword(p => !p)}
                  disabled={busy || connected}
                >
                  {showPassword ? "Hide" : "Show"}
                </button>
              </div>
              <div className="checkbox-row">
                <input
                  id="remember-password"
                  type="checkbox"
                  checked={rememberPassword}
                  onChange={e => setRememberPassword(e.target.checked)}
                  disabled={busy || connected}
                />
                <label htmlFor="remember-password">Remember password in Windows Credential Manager</label>
              </div>
            </div>

            <div className="field field-wide">
              <label htmlFor="identity">
                SSH private key path <span>(optional)</span>
              </label>
              <input
                id="identity"
                autoComplete="off"
                placeholder="C:\Users\username\.ssh\id_ed25519"
                value={profile.identityFile ?? ""}
                onChange={e => update("identityFile", e.target.value || null)}
                disabled={busy || connected}
              />
            </div>

            <div className="field">
              <label htmlFor="local-port">Local SOCKS5 port</label>
              <input
                id="local-port"
                inputMode="numeric"
                type="number"
                min="1"
                max="65535"
                value={profile.localSocksPort}
                onChange={e => update("localSocksPort", Number(e.target.value))}
                required
                disabled={busy || connected}
              />
            </div>

            {!connected && (
              <div className="field field-wide">
                <label htmlFor="profile-name">Profile name</label>
                <div className="profile-actions">
                  <input
                    id="profile-name"
                    placeholder="My server"
                    maxLength={48}
                    value={profileName}
                    onChange={e => setProfileName(e.target.value)}
                    disabled={busy}
                  />
                  <button
                    className="button"
                    type="button"
                    onClick={saveProfile}
                    disabled={busy || !profileName.trim() || !profile.host.trim() || !profile.username.trim()}
                  >
                    Save
                  </button>
                  {selectedName && (
                    <button className="button button-danger" type="button" onClick={deleteProfile} disabled={busy}>
                      Delete
                    </button>
                  )}
                </div>
              </div>
            )}

            <div className="form-note">
              <span className="note-icon">i</span>
              <p>
                Credentials are never stored in profile files. When remembered, passwords reside in Windows Credential Manager.
                Unchecking "Remember" immediately removes the credential.
              </p>
            </div>

            {error && <p className="error" role="alert">{error}</p>}

            {connected ? (
              <div className="connected-container">
                <div className="connected-panel">
                  <div>
                    <span className="connected-label">SOCKS5 endpoint</span>
                    <strong>{endpoint}</strong>
                  </div>
                  <button className="button button-danger" type="button" onClick={disconnect} disabled={busy}>
                    Disconnect
                  </button>
                </div>

                {/* App Launcher Section */}
                <div className="launcher-card">
                  <div className="launcher-header">
                    <h4>🚀 App Launcher</h4>
                    <span className="badge">Preset</span>
                  </div>
                  <p className="launcher-desc">
                    Launch Antigravity pre-configured with proxy environment variables and isolated local loopback bypass rules.
                  </p>

                  {launcherFeedback && (
                    <div className={`launcher-alert ${launcherFeedback.isError ? "alert-error" : "alert-success"}`}>
                      {launcherFeedback.message}
                    </div>
                  )}

                  <div className="launcher-actions">
                    <button
                      type="button"
                      className="button button-primary"
                      onClick={launchAntigravity}
                      disabled={launchingAntigravity}
                    >
                      {launchingAntigravity ? "Launching…" : "Launch Antigravity"}
                    </button>
                    <button
                      type="button"
                      className="button button-secondary"
                      onClick={() => setShowCustomPath(v => !v)}
                    >
                      {showCustomPath ? "Hide Custom Path" : "Custom Path"}
                    </button>
                  </div>

                  {showCustomPath && (
                    <div className="custom-path-box">
                      <label htmlFor="custom-antigravity">Antigravity Executable Path</label>
                      <input
                        id="custom-antigravity"
                        placeholder="C:\Users\...\Antigravity.exe"
                        value={customAntigravityPath}
                        onChange={e => setCustomAntigravityPath(e.target.value)}
                      />
                    </div>
                  )}
                </div>
              </div>
            ) : (
              <button
                className="button button-primary"
                type="submit"
                disabled={busy || !profile.host.trim() || !profile.username.trim()}
              >
                {status === "connecting" || probingHostKey ? "Connecting…" : "Connect"}
              </button>
            )}
          </form>
        </div>

        <footer className="footer">
          <span>NetworkViaShell 0.2 · Windows Desktop</span>
          <span>OpenSSH SOCKS5 · App Launcher with Loopback Bypass</span>
        </footer>
      </section>
    </main>
  );
}

export default App;
