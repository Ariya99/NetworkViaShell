import { useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";

type TunnelProfile = {
  host: string;
  port: number;
  username: string;
  localSocksPort: number;
  identityFile: string | null;
};

function App() {
  const [host, setHost] = useState("");
  const [port, setPort] = useState("22");
  const [username, setUsername] = useState("");
  const [localPort, setLocalPort] = useState("1080");
  const [identityFile, setIdentityFile] = useState("");
  const [status, setStatus] = useState<"idle" | "connecting" | "connected" | "stopping">("idle");
  const [endpoint, setEndpoint] = useState("");
  const [error, setError] = useState("");

  async function connect(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError("");
    setStatus("connecting");
    const profile: TunnelProfile = {
      host: host.trim(),
      port: Number(port),
      username: username.trim(),
      localSocksPort: Number(localPort),
      identityFile: identityFile.trim() || null,
    };
    try {
      const address = await invoke<string>("start_ssh_tunnel", { profile });
      setEndpoint(address);
      setStatus("connected");
    } catch (cause) {
      setStatus("idle");
      setError(String(cause));
    }
  }

  async function disconnect() {
    setStatus("stopping");
    setError("");
    try {
      await invoke("stop_ssh_tunnel");
      setEndpoint("");
      setStatus("idle");
    } catch (cause) {
      setStatus("connected");
      setError(String(cause));
    }
  }

  const busy = status === "connecting" || status === "stopping";
  return (
    <main className="shell">
      <section className="workspace" aria-labelledby="app-title">
        <header className="topbar">
          <div className="brand">
            <div className="brand-mark" aria-hidden="true">N</div>
            <div><p className="eyebrow">SECURE DESKTOP TUNNELLING</p><h1 id="app-title">NetworkViaShell</h1></div>
          </div>
          <div className={"status status-" + status}><span className="status-dot" />
            {status === "connected" ? "Connected" : status === "connecting" ? "Connecting" : status === "stopping" ? "Stopping" : "Not connected"}
          </div>
        </header>
        <div className="content">
          <div className="intro">
            <p className="eyebrow">FIRST ENGINE · OPENSSH</p>
            <h2>SSH tunnel</h2>
            <p>Connect to an SSH server and expose a private SOCKS5 proxy on this device.</p>
          </div>
          <form className="profile-form" onSubmit={connect}>
            <div className="field field-wide">
              <label htmlFor="host">SSH host</label>
              <input id="host" autoComplete="off" placeholder="server.example.com" value={host} onChange={(e) => setHost(e.target.value)} required disabled={busy || status === "connected"} />
            </div>
            <div className="field">
              <label htmlFor="username">Username</label>
              <input id="username" autoComplete="username" placeholder="user" value={username} onChange={(e) => setUsername(e.target.value)} required disabled={busy || status === "connected"} />
            </div>
            <div className="field">
              <label htmlFor="port">SSH port</label>
              <input id="port" inputMode="numeric" type="number" min="1" max="65535" value={port} onChange={(e) => setPort(e.target.value)} required disabled={busy || status === "connected"} />
            </div>
            <div className="field field-wide">
              <label htmlFor="identity">SSH private key path <span>(optional · or use ssh-agent)</span></label>
              <input id="identity" autoComplete="off" placeholder="/home/user/.ssh/id_ed25519" value={identityFile} onChange={(e) => setIdentityFile(e.target.value)} disabled={busy || status === "connected"} />
            </div>
            <div className="field">
              <label htmlFor="local-port">Local SOCKS5 port</label>
              <input id="local-port" inputMode="numeric" type="number" min="1" max="65535" value={localPort} onChange={(e) => setLocalPort(e.target.value)} required disabled={busy || status === "connected"} />
            </div>
            <div className="form-note"><span className="note-icon">i</span><p>Key authentication only. The server must already be trusted in your OpenSSH known_hosts file. No password is collected or stored.</p></div>
            {error && <p className="error" role="alert">{error}</p>}
            {status === "connected" ? (
              <div className="connected-panel">
                <div><span className="connected-label">SOCKS5 endpoint</span><strong>{endpoint}</strong></div>
                <button className="button button-danger" type="button" onClick={disconnect} disabled={busy}>Disconnect</button>
              </div>
            ) : (
              <button className="button button-primary" type="submit" disabled={busy || !host.trim() || !username.trim()}>
                {status === "connecting" ? "Connecting…" : "Connect"}
              </button>
            )}
          </form>
        </div>
        <footer className="footer"><span>Early development build</span><span>SSH · Local SOCKS5 · No system proxy changes</span></footer>
      </section>
    </main>
  );
}

export default App;
