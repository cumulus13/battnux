//! Web dashboard — serves a live battery status page and JSON API.
//! Runs as a background Tokio task.
//! Routes:
//!   GET /          → HTML dashboard (auto-refreshes via SSE)
//!   GET /api/data  → JSON snapshot of current state
//!   GET /events    → SSE stream (battery updates every interval)

use crate::monitor::SharedState;
use axum::{
    extract::State,
    response::{Html, IntoResponse, Sse},
    routing::get,
    Json, Router,
};
use log::info;
use std::{convert::Infallible, net::SocketAddr, sync::Arc, time::Duration};
use tokio::time::interval;
use tokio_stream::{wrappers::IntervalStream, StreamExt};

type AppState = Arc<SharedState>;

/// Spawn the web server as a Tokio background task.
pub async fn spawn(shared: SharedState, host: &str, port: u16) {
    let addr: SocketAddr = format!("{}:{}", host, port)
        .parse()
        .expect("Invalid web listen address");

    let state: AppState = Arc::new(shared);

    let app = Router::new()
        .route("/", get(dashboard_handler))
        .route("/api/data", get(api_handler))
        .route("/events", get(sse_handler))
        .with_state(state);

    info!("Web dashboard listening on http://{}", addr);
    println!(
        "  {} http://{}",
        "Web dashboard:".bright_cyan().bold(),
        addr.to_string().bright_white()
    );

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind web server");

    axum::serve(listener, app).await.expect("Web server error");
}

// ─── Handlers ────────────────────────────────────────────────────────────────

async fn api_handler(State(state): State<AppState>) -> impl IntoResponse {
    let data = state.lock().clone();
    Json(data)
}

async fn sse_handler(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<axum::response::sse::Event, Infallible>>> {
    let stream = IntervalStream::new(interval(Duration::from_secs(5))).map(move |_| {
        let data = state.lock().clone();
        let json = serde_json::to_string(&data).unwrap_or_default();
        Ok(axum::response::sse::Event::default().data(json))
    });
    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::default())
}

async fn dashboard_handler() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

// ─── Embedded dashboard HTML ──────────────────────────────────────────────────

const DASHBOARD_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>battnux — Battery Dashboard</title>
<style>
  :root {
    --bg: #0d1117; --surface: #161b22; --border: #30363d;
    --green: #3fb950; --yellow: #d29922; --red: #f85149;
    --cyan: #79c0ff; --text: #c9d1d9; --muted: #6e7681;
    --font: 'SF Mono', 'Fira Code', 'Consolas', monospace;
  }
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body { background: var(--bg); color: var(--text); font-family: var(--font); font-size: 14px; }
  header { padding: 1.5rem 2rem; border-bottom: 1px solid var(--border); display: flex; align-items: center; gap: 1rem; }
  header h1 { font-size: 1.4rem; color: var(--cyan); letter-spacing: 0.05em; }
  header span { color: var(--muted); font-size: 0.85rem; }
  #status-dot { width: 10px; height: 10px; border-radius: 50%; background: var(--green); animation: pulse 2s infinite; }
  @keyframes pulse { 0%,100%{opacity:1} 50%{opacity:0.4} }
  main { max-width: 900px; margin: 2rem auto; padding: 0 1.5rem; }
  .battery-card {
    background: var(--surface); border: 1px solid var(--border);
    border-radius: 8px; padding: 1.5rem; margin-bottom: 1.5rem;
  }
  .card-title { color: var(--cyan); font-size: 1rem; margin-bottom: 1rem; font-weight: bold; }
  .row { display: flex; justify-content: space-between; padding: 0.35rem 0; border-bottom: 1px solid var(--border); }
  .row:last-child { border-bottom: none; }
  .label { color: var(--muted); }
  .value { font-weight: bold; }
  .green { color: var(--green); }
  .yellow { color: var(--yellow); }
  .red { color: var(--red); }
  .cyan { color: var(--cyan); }
  .bar-wrap { background: var(--bg); border-radius: 4px; height: 16px; overflow: hidden; width: 200px; }
  .bar-fill { height: 100%; border-radius: 4px; transition: width 0.8s ease; }
  .warnings { background: #1a0000; border: 1px solid var(--red); border-radius: 8px; padding: 1rem 1.5rem; margin-bottom: 1.5rem; }
  .warnings h3 { color: var(--red); margin-bottom: 0.5rem; }
  .warnings li { color: #ff7b7b; margin-left: 1rem; line-height: 1.8; }
  #meta { color: var(--muted); font-size: 0.8rem; text-align: right; margin-bottom: 1rem; }
  footer { text-align: center; color: var(--muted); padding: 2rem; font-size: 0.8rem; }
</style>
</head>
<body>
<header>
  <div id="status-dot"></div>
  <h1>⚡ battnux</h1>
  <span>Live Battery Dashboard</span>
</header>
<main>
  <div id="meta">Connecting…</div>
  <div id="warnings-section" style="display:none">
    <div class="warnings">
      <h3>⚠ Warnings</h3>
      <ul id="warnings-list"></ul>
    </div>
  </div>
  <div id="batteries"></div>
</main>
<footer>battnux — <a href="https://github.com/cumulus13/battnux" style="color:var(--cyan)">github.com/cumulus13/battnux</a></footer>
<script>
function colorClass(pct) {
  if (pct >= 60) return 'green';
  if (pct >= 30) return 'yellow';
  return 'red';
}
function barColor(pct) {
  if (pct >= 60) return '#3fb950';
  if (pct >= 30) return '#d29922';
  return '#f85149';
}
function stateClass(state) {
  if (state === 'Charging') return 'cyan';
  if (state === 'Full') return 'green';
  if (state === 'Discharging') return 'yellow';
  return '';
}
function fmtMins(mins) {
  if (!mins) return '—';
  const h = Math.floor(mins / 60), m = Math.floor(mins % 60);
  return h > 0 ? `${h}h ${String(m).padStart(2,'0')}m` : `${m}m`;
}
function render(data) {
  document.getElementById('meta').textContent =
    `Refresh #${data.refresh_count} · ${data.last_refresh}`;

  // Warnings
  const ws = document.getElementById('warnings-section');
  const wl = document.getElementById('warnings-list');
  if (data.warnings && data.warnings.length > 0) {
    wl.innerHTML = data.warnings.map(w => `<li>${w}</li>`).join('');
    ws.style.display = 'block';
  } else {
    ws.style.display = 'none';
  }

  // Battery cards
  const container = document.getElementById('batteries');
  container.innerHTML = data.batteries.map(b => {
    const cc = colorClass(b.percentage);
    const sc = stateClass(b.state);
    const barW = Math.round(b.percentage) + '%';
    const barC = barColor(b.percentage);
    const hcc = colorClass(b.health_pct);
    return `
    <div class="battery-card">
      <div class="card-title">Battery #${b.index} — ${b.model || 'Unknown'}</div>
      <div class="row">
        <span class="label">Charge</span>
        <span class="value">
          <div style="display:flex;align-items:center;gap:0.5rem">
            <div class="bar-wrap"><div class="bar-fill" style="width:${barW};background:${barC}"></div></div>
            <span class="${cc}">${b.percentage.toFixed(1)}%</span>
          </div>
        </span>
      </div>
      <div class="row"><span class="label">State</span><span class="value ${sc}">${b.state}</span></div>
      <div class="row"><span class="label">Health</span><span class="value ${hcc}">${b.health_pct.toFixed(1)}%</span></div>
      <div class="row"><span class="label">Power Rate</span><span class="value">${Math.abs(b.power_rate_w).toFixed(2)} W</span></div>
      <div class="row"><span class="label">Time to Empty</span><span class="value yellow">${fmtMins(b.time_to_empty_min)}</span></div>
      <div class="row"><span class="label">Time to Full</span><span class="value cyan">${fmtMins(b.time_to_full_min)}</span></div>
      <div class="row"><span class="label">Energy</span><span class="value">${b.energy_wh.toFixed(2)} / ${b.energy_full_wh.toFixed(2)} Wh</span></div>
      <div class="row"><span class="label">Temperature</span><span class="value">${b.temperature_c != null ? b.temperature_c.toFixed(1) + ' °C' : '—'}</span></div>
      <div class="row"><span class="label">Cycles</span><span class="value">${b.cycle_count ?? '—'}</span></div>
      <div class="row"><span class="label">Technology</span><span class="value">${b.technology}</span></div>
      <div class="row"><span class="label">Vendor</span><span class="value">${b.vendor || '—'}</span></div>
    </div>`;
  }).join('');
}

// SSE live updates
const evtSource = new EventSource('/events');
evtSource.onmessage = e => { try { render(JSON.parse(e.data)); } catch {} };
evtSource.onerror = () => {
  document.getElementById('meta').textContent = 'Connection lost — retrying…';
};

// Also load immediately
fetch('/api/data').then(r => r.json()).then(render).catch(() => {});
</script>
</body>
</html>"#;

use colored::Colorize;
