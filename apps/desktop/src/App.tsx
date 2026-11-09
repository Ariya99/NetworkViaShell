function App() {
  return (
    <main className="shell">
      <section className="panel" aria-labelledby="app-title">
        <div className="brand-mark" aria-hidden="true">N</div>
        <p className="eyebrow">SECURE DESKTOP TUNNELLING</p>
        <h1 id="app-title">NetworkViaShell</h1>
        <p className="intro">The desktop foundation is ready. SSH connection management is the first implementation milestone.</p>
        <div className="status"><span className="status-dot" /><span>Not connected</span></div>
        <p className="footnote">Early development build · No tunnel is active</p>
      </section>
    </main>
  );
}
export default App;
