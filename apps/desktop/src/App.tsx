import { useEffect, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";

type TunnelProfile = { host: string; port: number; username: string; localSocksPort: number; identityFile: string | null };
type SavedProfile = { name: string; profile: TunnelProfile };
const emptyProfile = (): TunnelProfile => ({ host: "", port: 22, username: "", localSocksPort: 1080, identityFile: null });

function App() {
  const [profile, setProfile] = useState<TunnelProfile>(emptyProfile);
  const [profiles, setProfiles] = useState<SavedProfile[]>([]);
  const [profileName, setProfileName] = useState("");
  const [selectedName, setSelectedName] = useState("");
  const [status, setStatus] = useState<"idle" | "connecting" | "connected" | "stopping">("idle");
  const [endpoint, setEndpoint] = useState("");
  const [error, setError] = useState("");

  async function refreshProfiles() {
    try { setProfiles(await invoke<SavedProfile[]>("list_saved_profiles")); }
    catch (cause) { setError(String(cause)); }
  }
  useEffect(() => { void refreshProfiles(); }, []);

  function update<K extends keyof TunnelProfile>(key: K, value: TunnelProfile[K]) {
    setProfile(current => ({ ...current, [key]: value }));
  }
  function selectProfile(name: string) {
    setSelectedName(name);
    const saved = profiles.find(item => item.name === name);
    if (saved) { setProfile(saved.profile); setProfileName(saved.name); setError(""); }
  }
  async function saveProfile() {
    setError("");
    try {
      await invoke("save_profile", { name: profileName, profile });
      setSelectedName(profileName.trim());
      await refreshProfiles();
    } catch (cause) { setError(String(cause)); }
  }
  async function deleteProfile() {
    if (!selectedName) return;
    setError("");
    try {
      await invoke("delete_profile", { name: selectedName });
      setSelectedName(""); setProfileName(""); setProfile(emptyProfile());
      await refreshProfiles();
    } catch (cause) { setError(String(cause)); }
  }
  async function connect(event: FormEvent<HTMLFormElement>) {
    event.preventDefault(); setError(""); setStatus("connecting");
    try {
      const address = await invoke<string>("start_ssh_tunnel", { profile });
      setEndpoint(address); setStatus("connected");
    } catch (cause) { setStatus("idle"); setError(String(cause)); }
  }
  async function disconnect() {
    setStatus("stopping"); setError("");
    try { await invoke("stop_ssh_tunnel"); setEndpoint(""); setStatus("idle"); }
    catch (cause) { setStatus("connected"); setError(String(cause)); }
  }
  const busy = status === "connecting" || status === "stopping";
  const connected = status === "connected";
  return (
    <main className="shell">
      <section className="workspace" aria-labelledby="app-title">
        <header className="topbar">
          <div className="brand"><div className="brand-mark" aria-hidden="true">N</div>
            <div><p className="eyebrow">SECURE DESKTOP TUNNELLING</p><h1 id="app-title">NetworkViaShell</h1></div>
          </div>
          <div className={"status status-" + status}><span className="status-dot" />
            {connected ? "Connected" : status === "connecting" ? "Connecting" : status === "stopping" ? "Stopping" : "Not connected"}
          </div>
        </header>
        <div className="content">
          <div className="intro"><p className="eyebrow">FIRST ENGINE · OPENSSH</p><h2>SSH tunnel</h2>
            <p>Connect to an SSH server and expose a private SOCKS5 proxy on this device.</p></div>
          <div className="field field-wide">
            <label htmlFor="saved-profile">Saved connections</label>
            <select id="saved-profile" value={selectedName} onChange={e => selectProfile(e.target.value)} disabled={busy || connected}>
              <option value="">New connection…</option>
              {profiles.map(item => <option key={item.name} value={item.name}>{item.name} · {item.profile.username}@{item.profile.host}</option>)}
            </select>
          </div>
          <form className="profile-form" onSubmit={connect}>
            <div className="field field-wide"><label htmlFor="host">SSH host</label>
              <input id="host" autoComplete="off" placeholder="server.example.com" value={profile.host} onChange={e => update("host", e.target.value)} required disabled={busy || connected} /></div>
            <div className="field"><label htmlFor="username">Username</label>
              <input id="username" autoComplete="username" placeholder="user" value={profile.username} onChange={e => update("username", e.target.value)} required disabled={busy || connected} /></div>
            <div className="field"><label htmlFor="port">SSH port</label>
              <input id="port" inputMode="numeric" type="number" min="1" max="65535" value={profile.port} onChange={e => update("port", Number(e.target.value))} required disabled={busy || connected} /></div>
            <div className="field field-wide"><label htmlFor="identity">SSH private key path <span>(optional · or use ssh-agent)</span></label>
              <input id="identity" autoComplete="off" placeholder="/home/user/.ssh/id_ed25519" value={profile.identityFile ?? ""} onChange={e => update("identityFile", e.target.value || null)} disabled={busy || connected} /></div>
            <div className="field"><label htmlFor="local-port">Local SOCKS5 port</label>
              <input id="local-port" inputMode="numeric" type="number" min="1" max="65535" value={profile.localSocksPort} onChange={e => update("localSocksPort", Number(e.target.value))} required disabled={busy || connected} /></div>
            {!connected && <div className="field field-wide"><label htmlFor="profile-name">Profile name</label>
              <div className="profile-actions"><input id="profile-name" placeholder="My server" maxLength={48} value={profileName} onChange={e => setProfileName(e.target.value)} disabled={busy} />
                <button className="button" type="button" onClick={saveProfile} disabled={busy || !profileName.trim() || !profile.host.trim() || !profile.username.trim()}>Save</button>
                {selectedName && <button className="button button-danger" type="button" onClick={deleteProfile} disabled={busy}>Delete</button>}</div></div>}
            <div className="form-note"><span className="note-icon">i</span><p>Only connection details and the key-file path are saved. Private-key contents and passwords are never stored. The server must be trusted in OpenSSH known_hosts.</p></div>
            {error && <p className="error" role="alert">{error}</p>}
            {connected ? <div className="connected-panel"><div><span className="connected-label">SOCKS5 endpoint</span><strong>{endpoint}</strong></div>
              <button className="button button-danger" type="button" onClick={disconnect} disabled={busy}>Disconnect</button></div>
              : <button className="button button-primary" type="submit" disabled={busy || !profile.host.trim() || !profile.username.trim()}>
                {status === "connecting" ? "Connecting…" : "Connect"}</button>}
          </form>
        </div>
        <footer className="footer"><span>NetworkViaShell 0.1 · Linux MVP</span><span>SSH · Local SOCKS5 · No system proxy changes</span></footer>
      </section>
    </main>
  );
}
export default App;
