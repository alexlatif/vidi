// Vidi XP - Dashboard Viewer

const API_BASE = '/api/v1';

// Extract dashboard ID from URL
const pathParts = window.location.pathname.split('/');
const dashboardId = pathParts[pathParts.length - 1];

// State
let ws = null;
let wasmModule = null;
let jsDashboard = null;
let updateCount = 0;
let lastSeq = 0;
let reconnectAttempts = 0;
let useLegacyWasm = false; // Track if using legacy single WASM
const MAX_RECONNECT_ATTEMPTS = 5;
const RECONNECT_DELAY = 2000;

// DOM Elements
const dashboardTitle = document.getElementById('dashboard-title');
const loadingOverlay = document.getElementById('loading-overlay');
const errorOverlay = document.getElementById('error-overlay');
const errorMessage = document.getElementById('error-message');
const connectionStatus = document.getElementById('connection-status');
const connectionText = document.getElementById('connection-text');
const updateCountEl = document.getElementById('update-count');
const seqNumberEl = document.getElementById('seq-number');
const btnRefresh = document.getElementById('btn-refresh');
const btnFullscreen = document.getElementById('btn-fullscreen');

// Initialize
document.addEventListener('DOMContentLoaded', async () => {
  setupEventListeners();

  try {
    // Load dashboard metadata first
    const dashboard = await loadDashboardMeta();
    dashboardTitle.textContent = dashboard.xp_name || `Dashboard ${dashboardId.slice(0, 8)}`;

    // Add to recent
    VidiStorage.addRecent({
      id: dashboardId,
      xp_name: dashboard.xp_name
    });

    // Load WASM module
    await loadWasm();

    // Initialize dashboard with data (only needed for legacy WASM)
    // Per-dashboard WASM auto-starts with baked-in config
    if (useLegacyWasm) {
      initDashboard(dashboard.dashboard);
    } else {
      console.log('Per-dashboard WASM auto-started with baked-in config');
    }

    // Connect WebSocket for updates
    connectWebSocket();

    // Hide loading overlay
    loadingOverlay.style.display = 'none';
  } catch (error) {
    showError(error.message);
  }
});

// Setup event listeners
function setupEventListeners() {
  btnRefresh.addEventListener('click', () => {
    if (ws && ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({ type: 'get_state' }));
    } else {
      location.reload();
    }
  });

  btnFullscreen.addEventListener('click', toggleFullscreen);

  // Handle visibility changes for reconnection
  document.addEventListener('visibilitychange', () => {
    if (!document.hidden && (!ws || ws.readyState !== WebSocket.OPEN)) {
      connectWebSocket();
    }
  });
}

// Load dashboard metadata from API
async function loadDashboardMeta() {
  const response = await fetch(`${API_BASE}/dashboards/${dashboardId}`);
  if (!response.ok) {
    if (response.status === 404) {
      throw new Error('Dashboard not found');
    }
    throw new Error('Failed to load dashboard');
  }
  return response.json();
}

// Load WASM module (per-dashboard)
async function loadWasm() {
  try {
    // First check WASM compilation status
    const statusResponse = await fetch(`${API_BASE}/dashboards/${dashboardId}/wasm-status`);
    if (statusResponse.ok) {
      const status = await statusResponse.json();

      if (status.status === 'compiling') {
        // Wait for compilation with polling
        await waitForWasmCompilation();
      } else if (status.status === 'failed') {
        throw new Error(`WASM compilation failed: ${status.error || 'Unknown error'}`);
      } else if (status.status === 'pending') {
        // Trigger compilation and wait
        await fetch(`${API_BASE}/dashboards/${dashboardId}/recompile`, { method: 'POST' });
        await waitForWasmCompilation();
      }
    }

    // Import the per-dashboard WASM module
    const wasmPath = `/wasm/${dashboardId}/vidi.js`;
    wasmModule = await import(wasmPath);

    // For per-dashboard WASM with baked-in config, we just need to init
    // The module auto-starts via wasm_bindgen(start)
    await wasmModule.default();

    useLegacyWasm = false;
    console.log('Per-dashboard WASM module loaded');
  } catch (error) {
    console.error('Failed to load per-dashboard WASM:', error);

    // Fallback to legacy single WASM if per-dashboard not available
    try {
      console.log('Falling back to legacy single WASM...');
      const wasmPath = '/wasm/vidi.js';
      wasmModule = await import(wasmPath);
      await wasmModule.default();
      useLegacyWasm = true;
      console.log('Legacy WASM module loaded');
    } catch (fallbackError) {
      throw new Error('Failed to load WASM module. Make sure WASM files are built.');
    }
  }
}

// Wait for WASM compilation to complete
async function waitForWasmCompilation() {
  const maxAttempts = 120; // 2 minutes max
  const pollInterval = 1000; // 1 second

  for (let i = 0; i < maxAttempts; i++) {
    const response = await fetch(`${API_BASE}/dashboards/${dashboardId}/wasm-status`);
    if (!response.ok) {
      throw new Error('Failed to check WASM status');
    }

    const status = await response.json();

    if (status.status === 'ready' && status.wasm_ready) {
      return; // Compilation complete
    } else if (status.status === 'failed') {
      throw new Error(`WASM compilation failed: ${status.error || 'Unknown error'}`);
    }

    // Update loading message
    const loadingText = document.querySelector('.loading-overlay__text');
    if (loadingText) {
      loadingText.textContent = `Compiling dashboard... (${i + 1}s)`;
    }

    await new Promise(resolve => setTimeout(resolve, pollInterval));
  }

  throw new Error('WASM compilation timed out');
}

// Initialize the dashboard with data
function initDashboard(dashboardData) {
  try {
    const canvasId = 'dashboard-canvas';
    const jsonStr = JSON.stringify(dashboardData);

    // Create JsDashboard instance from WASM
    jsDashboard = new wasmModule.JsDashboard(jsonStr, canvasId);

    // Start the Bevy render loop
    jsDashboard.start();

    console.log('Dashboard initialized and started');
  } catch (error) {
    console.error('Failed to initialize dashboard:', error);
    throw new Error('Failed to initialize dashboard visualization');
  }
}

// Connect to WebSocket for real-time updates
function connectWebSocket() {
  if (ws && ws.readyState === WebSocket.OPEN) {
    return;
  }

  const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
  const wsUrl = `${protocol}//${window.location.host}/ws/v1/dashboards/${dashboardId}`;

  ws = new WebSocket(wsUrl);

  ws.onopen = () => {
    console.log('WebSocket connected');
    reconnectAttempts = 0;
    updateConnectionStatus(true);

    // Request sync if we have previous state
    if (lastSeq > 0) {
      ws.send(JSON.stringify({ type: 'sync', last_seq: lastSeq }));
    }
  };

  ws.onmessage = (event) => {
    try {
      const msg = JSON.parse(event.data);
      handleServerMessage(msg);
    } catch (error) {
      console.error('Failed to parse WebSocket message:', error);
    }
  };

  ws.onclose = () => {
    console.log('WebSocket disconnected');
    updateConnectionStatus(false);

    // Attempt reconnection
    if (reconnectAttempts < MAX_RECONNECT_ATTEMPTS) {
      reconnectAttempts++;
      setTimeout(connectWebSocket, RECONNECT_DELAY * reconnectAttempts);
    }
  };

  ws.onerror = (error) => {
    console.error('WebSocket error:', error);
  };
}

// Handle incoming server messages
function handleServerMessage(msg) {
  // Update sequence number
  if (msg.seq !== undefined) {
    lastSeq = msg.seq;
    seqNumberEl.textContent = lastSeq;
  }

  switch (msg.type) {
    case 'connected':
      console.log('Connected to dashboard:', msg.dashboard_id);
      break;

    case 'append_points':
      if (jsDashboard) {
        jsDashboard.append_points(msg.plot_id, msg.layer_idx, new Float32Array(msg.points));
        incrementUpdateCount();
      }
      break;

    case 'replace_trace':
      if (jsDashboard) {
        jsDashboard.replace_trace(msg.plot_id, msg.layer_idx, new Float32Array(msg.points));
        incrementUpdateCount();
      }
      break;

    case 'update_plot':
      if (jsDashboard) {
        jsDashboard.update_plot(msg.plot_id, JSON.stringify(msg.plot));
        incrementUpdateCount();
      }
      break;

    case 'refresh_all':
      if (jsDashboard) {
        jsDashboard.set_dashboard(JSON.stringify(msg.dashboard));
        incrementUpdateCount();
      }
      break;

    case 'error':
      console.error('Server error:', msg.message);
      break;

    default:
      console.warn('Unknown message type:', msg.type);
  }
}

// Update connection status indicator
function updateConnectionStatus(connected) {
  if (connected) {
    connectionStatus.classList.add('status-dot--connected');
    connectionText.textContent = 'Connected';
  } else {
    connectionStatus.classList.remove('status-dot--connected');
    connectionText.textContent = 'Disconnected';
  }
}

// Increment update counter
function incrementUpdateCount() {
  updateCount++;
  updateCountEl.textContent = `Updates: ${updateCount}`;
}

// Show error overlay
function showError(message) {
  loadingOverlay.style.display = 'none';
  errorMessage.textContent = message;
  errorOverlay.style.display = 'flex';
}

// Toggle fullscreen
function toggleFullscreen() {
  const container = document.querySelector('.dashboard-viewer__canvas-container');

  if (!document.fullscreenElement) {
    container.requestFullscreen().catch(err => {
      console.error('Failed to enter fullscreen:', err);
    });
  } else {
    document.exitFullscreen();
  }
}
