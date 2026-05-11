/**
 * Train Path Review — Detection overlay (T036, US4 / 004-train-detections).
 *
 * Renders detection records returned by `GET /api/detections` on the Leaflet
 * map and exposes a click-to-inspect details panel populated from
 * `PathResult.detection_provenance`.
 *
 *  - Applied punctual detection → filled circle marker placed at the
 *    `intrinsic` parameter along the resolved netelement's polyline.
 *  - Applied linear detection    → semi-transparent polyline along the
 *    full netelement geometry.
 *  - Discarded detection         → muted/dashed marker rendered at the
 *    nearest known anchor (netelement intrinsic, when present); otherwise
 *    skipped on the map.
 *
 * The webapp is read-only with respect to detections (per spec
 * clarification Q2): the user cannot create or edit detections in the UI.
 */

'use strict';

(function () {
  // ---------------------------------------------------------------------------
  // Module state
  // ---------------------------------------------------------------------------

  /** @type {L.LayerGroup|null} */
  let detectionLayerGroup = null;

  /** @type {L.Map|null} */
  let mapRef = null;

  /** @type {Map<string, Array<[number, number]>>} */
  let netGeometries = new Map();

  /** @type {boolean} */
  let detectionsVisible = true;

  // ---------------------------------------------------------------------------
  // Public entry point — called from app.js after the map is ready.
  // ---------------------------------------------------------------------------

  async function initDetections(map, networkData) {
    if (!map) return;
    mapRef = map;

    indexNetworkGeometries(networkData);

    detectionLayerGroup = L.layerGroup().addTo(map);
    wireToggle();

    const data = await fetchDetections();
    if (!data) return;

    renderDetections(data);
  }

  // ---------------------------------------------------------------------------
  // Geometry index
  // ---------------------------------------------------------------------------

  function indexNetworkGeometries(networkData) {
    netGeometries = new Map();
    if (!networkData || !networkData.features) return;
    for (const feature of networkData.features) {
      const id = feature.properties && feature.properties.netelement_id;
      const coords = feature.geometry && feature.geometry.coordinates;
      if (!id || !Array.isArray(coords)) continue;
      // GeoJSON: coords are [lon, lat]; Leaflet wants [lat, lon].
      const latlngs = coords.map(([lon, lat]) => [lat, lon]);
      netGeometries.set(id, latlngs);
    }
  }

  // ---------------------------------------------------------------------------
  // Fetch
  // ---------------------------------------------------------------------------

  async function fetchDetections() {
    try {
      const resp = await fetch('/api/detections');
      if (!resp.ok) return null;
      return await resp.json();
    } catch {
      return null;
    }
  }

  // ---------------------------------------------------------------------------
  // Rendering
  // ---------------------------------------------------------------------------

  function renderDetections(data) {
    if (!detectionLayerGroup) return;
    detectionLayerGroup.clearLayers();

    for (const record of data.punctual || []) {
      addPunctualMarker(record, /* discarded */ false);
    }
    for (const record of data.linear || []) {
      addLinearOverlay(record, /* discarded */ false);
    }
    for (const record of data.discarded || []) {
      if (record.kind === 'linear') {
        addLinearOverlay(record, /* discarded */ true);
      } else {
        addPunctualMarker(record, /* discarded */ true);
      }
    }
  }

  function addPunctualMarker(record, discarded) {
    const netId = appliedNetelementId(record);
    if (!netId) return;
    const coords = netGeometries.get(netId);
    if (!coords || coords.length === 0) return;

    const intrinsic = appliedIntrinsic(record);
    const latlng = interpolateAlong(coords, intrinsic);
    if (!latlng) return;

    const style = discarded
      ? { radius: 5, color: '#9ca3af', fillColor: '#9ca3af', fillOpacity: 0.0, weight: 1, dashArray: '3,3' }
      : { radius: 6, color: '#1d4ed8', fillColor: '#1d4ed8', fillOpacity: 0.8, weight: 2 };

    const marker = L.circleMarker(latlng, style);
    marker.bindTooltip(tooltipFor(record));
    marker.on('click', () => openDetailsPanel(record));
    marker.addTo(detectionLayerGroup);
  }

  function addLinearOverlay(record, discarded) {
    const netId = appliedNetelementId(record);
    if (!netId) return;
    const coords = netGeometries.get(netId);
    if (!coords || coords.length < 2) return;

    const style = discarded
      ? { color: '#9ca3af', weight: 4, opacity: 0.5, dashArray: '6,6' }
      : { color: '#7c3aed', weight: 5, opacity: 0.45 };

    const line = L.polyline(coords, style);
    line.bindTooltip(tooltipFor(record));
    line.on('click', () => openDetailsPanel(record));
    line.addTo(detectionLayerGroup);
  }

  // ---------------------------------------------------------------------------
  // Record accessors (DetectionRecord shape, see tp-core/src/models/detection_record.rs)
  // ---------------------------------------------------------------------------

  function appliedNetelementId(record) {
    const status = record.status || {};
    if (typeof status.netelement_id === 'string') return status.netelement_id;
    return null;
  }

  function appliedIntrinsic(record) {
    const status = record.status || {};
    if (typeof status.intrinsic === 'number') return status.intrinsic;
    return 0.5;
  }

  // ---------------------------------------------------------------------------
  // Geometry helpers
  // ---------------------------------------------------------------------------

  function interpolateAlong(latlngs, intrinsic) {
    if (!Array.isArray(latlngs) || latlngs.length === 0) return null;
    if (latlngs.length === 1) return latlngs[0];

    const clamped = Math.max(0, Math.min(1, intrinsic));
    let totalLength = 0;
    const segLengths = [];
    for (let i = 1; i < latlngs.length; i++) {
      const d = chord(latlngs[i - 1], latlngs[i]);
      segLengths.push(d);
      totalLength += d;
    }
    if (totalLength === 0) return latlngs[0];

    const target = clamped * totalLength;
    let acc = 0;
    for (let i = 0; i < segLengths.length; i++) {
      const next = acc + segLengths[i];
      if (target <= next || i === segLengths.length - 1) {
        const t = segLengths[i] === 0 ? 0 : (target - acc) / segLengths[i];
        return [
          latlngs[i][0] + (latlngs[i + 1][0] - latlngs[i][0]) * t,
          latlngs[i][1] + (latlngs[i + 1][1] - latlngs[i][1]) * t,
        ];
      }
      acc = next;
    }
    return latlngs[latlngs.length - 1];
  }

  function chord(a, b) {
    const dy = a[0] - b[0];
    const dx = a[1] - b[1];
    return Math.sqrt(dx * dx + dy * dy);
  }

  // ---------------------------------------------------------------------------
  // Details panel
  // ---------------------------------------------------------------------------

  function tooltipFor(record) {
    const id = record.id ? `#${record.id}` : '(no id)';
    const kind = record.kind || '?';
    const status = (record.status && record.status.status) || '?';
    return `Detection ${id} — ${kind} / ${status}`;
  }

  function openDetailsPanel(record) {
    const panel = document.getElementById('detection-details');
    if (!panel) return;

    panel.classList.remove('hidden');

    const fmtTimestamp = () => {
      const ts = record.timestamp;
      if (!ts) return '—';
      if (ts.timestamp) return ts.timestamp;
      if (ts.t_from && ts.t_to) return `${ts.t_from} → ${ts.t_to}`;
      return JSON.stringify(ts);
    };

    const status = record.status || {};
    const reason = status.reason || null;

    const rows = [
      ['ID', record.id || '—'],
      ['Source', record.source || '—'],
      ['Source file', record.source_file || '—'],
      ['Source row', record.source_row != null ? String(record.source_row) : '—'],
      ['Kind', record.kind || '—'],
      ['Time', fmtTimestamp()],
      ['Status', status.status || '—'],
    ];

    if (status.netelement_id) rows.push(['Netelement', status.netelement_id]);
    if (typeof status.intrinsic === 'number') rows.push(['Intrinsic', status.intrinsic.toFixed(4)]);
    if (typeof status.distance_m === 'number') rows.push(['Distance (m)', status.distance_m.toFixed(2)]);
    if (reason) {
      rows.push(['Discard reason', reason.kind || '—']);
      if (typeof reason.nearest_distance_m === 'number') {
        rows.push(['Nearest distance (m)', reason.nearest_distance_m.toFixed(2)]);
      }
      if (typeof reason.cutoff_m === 'number') {
        rows.push(['Cutoff (m)', reason.cutoff_m.toFixed(2)]);
      }
    }
    if (typeof record.provenance_index === 'number') {
      rows.push(['Provenance index', String(record.provenance_index)]);
    }

    let html = '<h3>Detection details</h3><table class="detection-details-table">';
    for (const [k, v] of rows) {
      html += `<tr><th>${escapeHtml(k)}</th><td>${escapeHtml(String(v))}</td></tr>`;
    }
    html += '</table>';

    if (record.metadata && typeof record.metadata === 'object') {
      const entries = Object.entries(record.metadata);
      if (entries.length > 0) {
        html += '<h4>Metadata</h4><table class="detection-details-table">';
        for (const [k, v] of entries) {
          html += `<tr><th>${escapeHtml(k)}</th><td>${escapeHtml(JSON.stringify(v))}</td></tr>`;
        }
        html += '</table>';
      }
    }

    html += '<button id="detection-details-close" class="btn btn-secondary">Close</button>';
    panel.innerHTML = html;

    const closeBtn = document.getElementById('detection-details-close');
    if (closeBtn) closeBtn.addEventListener('click', () => panel.classList.add('hidden'));
  }

  function escapeHtml(s) {
    return s
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#39;');
  }

  // ---------------------------------------------------------------------------
  // Toggle wiring
  // ---------------------------------------------------------------------------

  function wireToggle() {
    const toggle = document.getElementById('detections-toggle');
    if (!toggle) return;
    toggle.checked = detectionsVisible;
    toggle.addEventListener('change', (e) => {
      detectionsVisible = !!e.target.checked;
      if (!detectionLayerGroup) return;
      if (detectionsVisible) {
        if (mapRef) detectionLayerGroup.addTo(mapRef);
      } else {
        detectionLayerGroup.remove();
      }
    });
  }

  // ---------------------------------------------------------------------------
  // Export to global scope so app.js can invoke us.
  // ---------------------------------------------------------------------------

  window.TpDetections = { init: initDetections };
})();
