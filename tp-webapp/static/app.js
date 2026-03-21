/**
 * Train Path Review — Single-Page Application (T021, T028, T033)
 *
 * Responsibilities:
 *  - Render the railway network as Leaflet polylines (T021)
 *  - Colour each netelement by its confidence score
 *  - Allow click-to-add / click-to-remove segments (toggle path membership)
 *  - Adapt UI buttons to standalone vs integrated mode (T028)
 *  - Show GNSS positions as orange circle markers when loaded (T033)
 */

'use strict';

// ---------------------------------------------------------------------------
// Globals
// ---------------------------------------------------------------------------
/** @type {L.Map} */
let map;
/** @type {string} — 'standalone' or 'integrated' */
let appMode = 'standalone';
/** @type {Array<{netelement_id: string, layer: L.Polyline}>} */
const netLayers = [];
/** @type {L.TileLayer|null} */
let osmLayer = null;
/** @type {object|null} — last fetched path response */
let pathData = null;

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------
document.addEventListener('DOMContentLoaded', init);

async function init() {
  map = L.map('map');

  osmLayer = L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
    attribution: '© <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>',
    maxZoom: 19,
  }).addTo(map);

  document.getElementById('basemap-toggle').addEventListener('change', (e) => {
    if (e.target.checked) {
      osmLayer.addTo(map);
    } else {
      map.removeLayer(osmLayer);
    }
  });

  document.getElementById('darkmode-toggle').addEventListener('change', (e) => {
    document.body.classList.toggle('dark', e.target.checked);
  });

  // Initialise dark mode from system preference
  const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
  if (prefersDark) {
    document.body.classList.add('dark');
    document.getElementById('darkmode-toggle').checked = true;
  }

  // Load data in parallel
  const [networkData, fetchedPath, gnssData] = await Promise.all([
    apiFetch('/api/network'),
    apiFetch('/api/path'),
    apiFetch('/api/gnss'),
  ]);
  pathData = fetchedPath;

  if (!networkData || !pathData) {
    setStatus('Failed to load data from server.', true);
    return;
  }

  appMode = pathData.mode;
  setupModeUI(pathData);
  renderNetwork(networkData, pathData);
  updateSidebar(pathData);

  if (gnssData && gnssData.features && gnssData.features.length > 0) {
    renderGnss(gnssData);
  }

  // Fit map to network bounds
  const allLayers = netLayers.map((nl) => nl.layer);
  if (allLayers.length > 0) {
    const group = L.featureGroup(allLayers);
    map.fitBounds(group.getBounds(), { padding: [30, 30] });
  }
}

// ---------------------------------------------------------------------------
// Mode-dependent UI (T028)
// ---------------------------------------------------------------------------
function setupModeUI(pathData) {
  const badge = document.getElementById('mode-badge');
  const saveBtn = document.getElementById('save-btn');
  const confirmBtn = document.getElementById('confirm-btn');
  const abortBtn = document.getElementById('abort-btn');

  document.getElementById('close-btn').addEventListener('click', () => window.close());

  if (appMode === 'standalone') {
    badge.textContent = 'Standalone';
    badge.className = 'badge standalone';
    saveBtn.classList.remove('hidden');
    saveBtn.addEventListener('click', onSave);
  } else {
    badge.textContent = 'Review';
    badge.className = 'badge integrated';
    confirmBtn.classList.remove('hidden');
    abortBtn.classList.remove('hidden');
    confirmBtn.addEventListener('click', onConfirm);
    abortBtn.addEventListener('click', onAbort);
  }
}

// ---------------------------------------------------------------------------
// Network rendering (T021)
// ---------------------------------------------------------------------------
function renderNetwork(networkData, pathData) {
  // Build a lookup: netelement_id → segment info
  const pathMap = {};
  for (const seg of pathData.segments) {
    pathMap[seg.netelement_id] = seg;
  }

  for (const feature of networkData.features) {
    const id = feature.properties.netelement_id;
    const inPath = feature.properties.in_path;
    const coords = feature.geometry.coordinates.map(([lon, lat]) => [lat, lon]);

    const seg = pathMap[id];
    const layer = L.polyline(coords, lineStyle(feature.properties, seg));

    layer.bindTooltip(makeTooltip(feature.properties, seg));

    layer.on('click', () => onNetElementClick(id, inPath));
    layer.on('mouseover', () => layer.setStyle({ weight: 6 }));
    layer.on('mouseout', () => layer.setStyle({ weight: inPath ? 4 : 3 }));

    layer.addTo(map);
    netLayers.push({ id, layer });
  }
}

function lineStyle(properties, seg) {
  const inPath = properties.in_path;
  if (!inPath) {
    return { color: '#6b7280', weight: 3, opacity: 0.9 };
  }
  const conf = properties.confidence ?? 0;
  const color = confidenceColor(conf);
  const dashArray = (seg && seg.origin === 'manual') ? '10, 5' : null;
  return { color, weight: 4, opacity: 0.9, dashArray };
}

function confidenceColor(conf) {
  if (conf >= 0.7) return '#16a34a'; // green
  if (conf >= 0.3) return '#ca8a04'; // amber
  return '#dc2626';                  // red
}

/**
 * Escape a value for safe insertion into an HTML context.
 * Converts `&`, `<`, `>`, `"`, and `'` to their HTML entity equivalents.
 * Use this whenever embedding untrusted data (e.g. netelement IDs from a
 * network file) into Leaflet tooltip HTML to prevent XSS injection.
 * @param {*} str - Value to escape (coerced to string).
 * @returns {string}
 */
function escapeHtml(str) {
  return String(str)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function makeTooltip(props, seg) {
  const conf = props.confidence != null ? (props.confidence * 100).toFixed(1) + '%' : '—';
  const origin = escapeHtml(props.origin ?? '—');
  return `<b>${escapeHtml(props.netelement_id)}</b><br>Confidence: ${conf}<br>Origin: ${origin}`;
}

// ---------------------------------------------------------------------------
// Click to add / remove segments (T021)
// ---------------------------------------------------------------------------
async function onNetElementClick(neId, currentlyInPath) {
  let ok;
  if (currentlyInPath) {
    ok = await apiPost('/api/path/remove', { netelement_id: neId });
  } else {
    ok = await apiPost('/api/path/add', { netelement_id: neId });
  }
  if (!ok) return;

  // Refresh map
  pathData = await apiFetch('/api/path');
  const networkData = await apiFetch('/api/network');
  if (!pathData || !networkData) return;

  refreshNetworkLayers(networkData, pathData);
  updateSidebar(pathData);
}

function refreshNetworkLayers(networkData, pathData) {
  // Remove existing layers
  for (const { layer } of netLayers) {
    map.removeLayer(layer);
  }
  netLayers.length = 0;
  renderNetwork(networkData, pathData);
}

// ---------------------------------------------------------------------------
// Sidebar update
// ---------------------------------------------------------------------------
function updateSidebar(pathData) {
  document.getElementById('segment-count-value').textContent = pathData.segments.length;
  const avg = pathData.overall_probability;
  document.getElementById('overall-prob-value').textContent =
    avg != null ? (avg * 100).toFixed(1) + '%' : '—';

  const list = document.getElementById('segment-list');
  list.innerHTML = '';
  pathData.segments.forEach((seg, i) => {
    const li = document.createElement('li');
    li.className = `origin-${seg.origin}`;
    const conf = (seg.probability * 100).toFixed(1);
    li.textContent = `#${i + 1} ${seg.netelement_id} (${conf}%, ${seg.origin})`;
    list.appendChild(li);
  });
}

// ---------------------------------------------------------------------------
// Save / Confirm / Abort handlers
// ---------------------------------------------------------------------------
async function onSave() {
  const pathData = await apiFetch('/api/path');
  if (pathData && pathData.segments.length === 0) {
    setStatus('Cannot save: path is empty.');
    return;
  }
  const result = await apiPost('/api/save');
  if (result && result.ok) {
    setStatus(`Saved to: ${result.path}`);
  }
}

async function onConfirm() {
  const pathData = await apiFetch('/api/path');
  if (pathData && pathData.segments.length === 0) {
    setStatus('Cannot confirm: path is empty.');
    return;
  }
  const result = await apiPost('/api/confirm');
  if (result && result.ok) {
    setStatus('Path confirmed — you may close this window.');
    document.getElementById('confirm-btn').disabled = true;
    document.getElementById('abort-btn').disabled = true;
  }
}

async function onAbort() {
  if (!confirm('Abort the review? The original path will be kept.')) return;
  const result = await apiPost('/api/abort');
  if (result && result.ok) {
    setStatus('Review aborted — you may close this window.');
    document.getElementById('confirm-btn').disabled = true;
    document.getElementById('abort-btn').disabled = true;
  }
}

function setStatus(msg, isError = false) {
  const bar = document.getElementById('status-bar');
  bar.textContent = msg;
  bar.style.color = isError ? '#dc2626' : '#16a34a';
}

// ---------------------------------------------------------------------------
// GNSS overlay (T033)
// ---------------------------------------------------------------------------
function renderGnss(gnssData) {
  for (const feature of gnssData.features) {
    const [lon, lat] = feature.geometry.coordinates;
    L.circleMarker([lat, lon], {
      radius: 4,
      color: '#f97316',
      fillColor: '#f97316',
      fillOpacity: 0.7,
      weight: 1,
    })
      .bindTooltip(`GNSS (${lat.toFixed(5)}, ${lon.toFixed(5)})`)
      .addTo(map);
  }
}

// ---------------------------------------------------------------------------
// Fetch helpers
// ---------------------------------------------------------------------------
async function apiFetch(url) {
  try {
    const resp = await fetch(url);
    if (!resp.ok) return null;
    return await resp.json();
  } catch {
    return null;
  }
}

async function apiPost(url, body = null) {
  try {
    const opts = { method: 'POST' };
    if (body !== null) {
      opts.headers = { 'Content-Type': 'application/json' };
      opts.body = JSON.stringify(body);
    }
    const resp = await fetch(url, opts);
    if (!resp.ok) {
      const body = await resp.json().catch(() => ({}));
      setStatus(body.error ?? `Request to ${url} failed (${resp.status})`, true);
      return null;
    }
    return await resp.json();
  } catch {
    setStatus(`Network error on ${url}`, true);
    return null;
  }
}

async function apiPut(url, body) {
  try {
    const resp = await fetch(url, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    if (!resp.ok) {
      const b = await resp.json().catch(() => ({}));
      setStatus(b.error ?? `PUT ${url} failed (${resp.status})`, true);
      return null;
    }
    return await resp.json();
  } catch {
    setStatus(`Network error on PUT ${url}`, true);
    return null;
  }
}
