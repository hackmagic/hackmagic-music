// HackMagic Music Player - Web Frontend
const OHOS = typeof ohosBridge !== 'undefined';
const ELECTRON = typeof electronAPI !== 'undefined';

// Dynamic API URLs (from Electron or hardcoded)
let API = 'http://127.0.0.1:10280';
let WS_URL = 'ws://127.0.0.1:10280/ws';

// Electron initialization
if (ELECTRON) {
  (async () => {
    API = await electronAPI.getApiUrl();
    WS_URL = await electronAPI.getWsUrl();
    
    // Setup backend command listener
    electronAPI.onBackendCommand((command) => {
      cmd(command);
    });
  })();
}

// ===== Toast Notifications =====
let _toastId = 0;
function showToast(title, msg, icon, duration) {
  const container = document.getElementById('toast-container') || (() => {
    const el = document.createElement('div');
    el.id = 'toast-container';
    document.body.appendChild(el);
    return el;
  })();
  const id = ++_toastId;
  const div = document.createElement('div');
  div.className = 'toast';
  div.innerHTML = `<span class="toast-icon ${icon || 'info'}">${ICONS.info}</span>
    <div class="toast-body"><div class="toast-title">${escHtml(title)}</div>${msg ? `<div class="toast-msg">${escHtml(msg)}</div>` : ''}</div>
    <span class="toast-close" data-toast="${id}">${ICONS.close}</span>`;
  div.querySelector('.toast-close').addEventListener('click', () => removeToast(div));
  container.appendChild(div);
  if (duration !== 0) setTimeout(() => removeToast(div), duration || 4000);
}
function removeToast(el) {
  if (el.classList.contains('removing')) return;
  el.classList.add('removing');
  el.addEventListener('animationend', () => el.remove());
}

let state = {
  playing: false,
  paused: false,
  position: 0,
  duration: 0,
  volume: 80,
  playlist: [],
  currentIndex: null,
  playlistCount: 0,
  repeat: 'loop',
  isDragging: false,
  dragPosition: undefined,
  connected: false,
};
let gLastNotifiedPath = ''; // Track change notification dedup

// ===== API calls =====
async function api(method, path, body) {
  if (OHOS && window._ohosApi) {
    return window._ohosApi(method, path, body);
  }
  try {
    const opts = { method, headers: {} };
    if (body) {
      opts.headers['Content-Type'] = 'application/json';
      opts.body = JSON.stringify(body);
    }
    const res = await fetch(`${API}${path}`, opts);
    if (!res.ok) return null;
    return await res.json();
  } catch (e) {
    return null;
  }
}

// ===== Connection management =====
let connRetryDelay = 500;
const CONN_MAX_RETRY = 30000;

function updateConnectionStatus() {
  const el = document.getElementById('connection-status');
  const dot = document.getElementById('conn-dot');
  const text = document.getElementById('conn-text');
  if (state.connected) {
    el.className = 'connected visible';
    text.textContent = '已连接';
  } else {
    el.className = 'disconnected visible';
    text.textContent = connRetryDelay < CONN_MAX_RETRY
      ? '正在连接后端...'
      : '后端连接失败，请检查播放器是否运行';
  }
}

async function checkBackend() {
  if (OHOS) {
    if (!state.connected) {
      state.connected = true;
      updateConnectionStatus();
      poll();
    }
    return true;
  }
  try {
    const res = await fetch(`${API}/api/health`, { method: 'GET', signal: AbortSignal.timeout(3000) });
    if (res.ok) {
      const data = await res.json();
      if (data.ok) {
        if (!state.connected) {
          state.connected = true;
          connRetryDelay = 500;
          updateConnectionStatus();
          connectWS();
          poll();
        }
        return true;
      }
    }
  } catch (_) {}
  if (state.connected) {
    state.connected = false;
    updateConnectionStatus();
  }
  connRetryDelay = Math.min(connRetryDelay * 1.5, CONN_MAX_RETRY);
  setTimeout(checkBackend, connRetryDelay);
  return false;
}

async function cmd(command) {
  if (OHOS && window.ohosBridge) {
    const parts = command.split(' ');
    const action = parts[0];
    const args = parts.slice(1).join(' ');
    switch (action) {
      case 'play': case 'open': ohosBridge.play(args); break;
      case 'pause': ohosBridge.pause(); break;
      case 'resume': ohosBridge.resume(); break;
      case 'stop': ohosBridge.stop(); break;
      case 'next': ohosBridge.next(); break;
      case 'prev': ohosBridge.prev(); break;
      case 'seek': ohosBridge.seek(parseFloat(args)); break;
      case 'volume': ohosBridge.setVolume(parseInt(args)); break;
      case 'repeat': ohosBridge.setRepeat(args); break;
      case 'play_index': ohosBridge.playAtIndex(parseInt(args)); break;
      case 'remove_index': ohosBridge.removeFromPlaylist(parseInt(args)); break;
      case 'clear': ohosBridge.clearPlaylist(); break;
      default: break;
    }
    return { ok: true };
  }
  return api('POST', '/api/command', { command });
}

// ===== Status Polling =====
let _lastTrackFile = null;

async function fetchStatus() {
  const data = await api('GET', '/api/status');
  if (!data) return;

  state.volume = data.volume;
  state.currentIndex = data.playlist_index;
  state.playlistCount = data.playlist_count;
  state.repeat = data.repeat;

  if (data.state === 'playing') { state.playing = true; state.paused = false; }
  else if (data.state === 'paused') { state.playing = false; state.paused = true; }
  else { state.playing = false; state.paused = false; }

  // Sync playing class for vinyl spin animation
  const albumArt = document.getElementById('album-art');
  if (state.playing) albumArt.classList.add('playing');
  else albumArt.classList.remove('playing');

  if (data.track) {
    state.position = data.track.position_secs;
    state.duration = data.track.duration_secs;
    // Track change detection
    if (data.track.file !== _lastTrackFile) {
      _lastTrackFile = data.track.file;
      const title = data.track.title || data.track.file.split(/[/\\]/).pop();
      showToast(title, data.track.artist || 'Unknown Artist', 'music_note', 3000);
      // Browser Notification API
      if (ELECTRON && electronAPI.showNotification) {
        electronAPI.showNotification({
          title: '正在播放',
          body: `${title} - ${data.track.artist || 'Unknown Artist'}`,
          icon: `${API}/api/cover`
        });
      } else if ('Notification' in window && Notification.permission === 'granted') {
        new Notification('正在播放', { body: `${title} - ${data.track.artist || 'Unknown Artist'}`, icon: `${API}/api/cover` });
      }
    }
    updateTrackInfo(data.track);
  } else {
    _lastTrackFile = null;
    state.position = 0;
    state.duration = 0;
    clearTrackInfo();
  }

  updatePlayButton();
  updateProgress();
  updateVolume();
  updatePlaylistHighlight();
  updateRepeatBadge();
}

async function fetchPlaylist() {
  const data = await api('GET', '/api/playlist');
  if (!data || !data.tracks) return;
  state.playlist = data.tracks;
  state.currentIndex = data.current_index;
  state.playlistCount = data.tracks.length;
  document.getElementById('pl-name').textContent = data.name;
  document.getElementById('track-count').textContent = data.tracks.length;
  // Fetch play queue
  try { state._playQueue = await api('GET', '/api/playlist/queue') || []; } catch { state._playQueue = []; }
  renderPlaylist();
}

// ===== Lyrics =====
async function fetchLyrics() {
  const data = await api('GET', '/api/lyric');
  if (!data) return;
  renderLyrics(data);
}

function renderLyrics(data) {
  const container = document.getElementById('lyric-lines');
  const placeholder = document.getElementById('lyric-placeholder');

  if (!data.has_lyrics || data.lines.length === 0) {
    container.innerHTML = '';
    placeholder.style.display = 'block';
    return;
  }

  placeholder.style.display = 'none';
  let html = '';
  for (const line of data.lines) {
    const cls = line.is_current ? 'current' : line.is_next ? 'next' : '';
    html += `<div class="lyric-line ${cls}" data-time="${line.time_ms}">${escHtml(line.text)}`;
    if (line.translate) {
      html += `<span class="translation">${escHtml(line.translate)}</span>`;
    }
    html += `</div>`;
  }
  container.innerHTML = html;

  // Auto-scroll to current line
  const currentEl = container.querySelector('.lyric-line.current');
  if (currentEl) {
    const lyricContainer = document.getElementById('lyric-container');
    const offset = currentEl.offsetTop - lyricContainer.clientHeight / 2 + currentEl.clientHeight / 2;
    lyricContainer.scrollTo({ top: Math.max(0, offset), behavior: 'smooth' });
  }
}

function escHtml(s) {
  if (!s) return '';
  return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;');
}

// ===== Cover =====
async function fetchCover() {
  const data = await api('GET', '/api/cover');
  const img = document.getElementById('cover-img');
  const placeholder = document.getElementById('cover-placeholder');
  if (data && data.has_cover && data.data) {
    img.src = `data:${data.mime};base64,${data.data}`;
    img.style.display = 'block';
    placeholder.style.display = 'none';
  } else {
    img.style.display = 'none';
    placeholder.style.display = 'block';
  }
}

// ===== Media Session API (OS media keys, lock screen controls) =====
function setupMediaSession() {
  if (!('mediaSession' in navigator)) return;
  navigator.mediaSession.setActionHandler('play', () => cmd('pause'));
  navigator.mediaSession.setActionHandler('pause', () => cmd('pause'));
  navigator.mediaSession.setActionHandler('nexttrack', () => cmd('next'));
  navigator.mediaSession.setActionHandler('previoustrack', () => cmd('prev'));
  navigator.mediaSession.setActionHandler('seekto', (d) => {
    if (d.fastSeek != null && !d.fastSeek) return;
    cmd(`seek ${Math.floor(d.seekTime)}`);
  });
}

function updateMediaSession(track) {
  if (!('mediaSession' in navigator)) return;
  if (!track) {
    navigator.mediaSession.playbackState = 'none';
    return;
  }
  navigator.mediaSession.metadata = new MediaMetadata({
    title: track.title || track.file?.split(/[/\\]/).pop() || '',
    artist: track.artist || 'Unknown Artist',
    album: track.album || '',
    artwork: [{ src: `${API}/api/cover`, sizes: '512x512', type: 'image/jpeg' }],
  });
  navigator.mediaSession.playbackState = state.playing ? 'playing' : 'paused';
}

// ===== UI Updates =====
function updateTrackInfo(track) {
  document.getElementById('track-title').textContent = track.title || track.file.split(/[/\\]/).pop();
  document.getElementById('track-artist').textContent = track.artist || 'Unknown Artist';
  document.getElementById('track-album').textContent = track.album ? `Album: ${track.album}` : '';
  document.title = `${track.title || track.file.split(/[/\\]/).pop()} - ${track.artist || 'Unknown Artist'} - HackMagic Music Player`;
  updateMediaSession(track);
  // Show notification for new track
  if (track.title && track.file_path !== gLastNotifiedPath) {
    gLastNotifiedPath = track.file_path;
    if (ELECTRON && electronAPI.showNotification) {
      electronAPI.showNotification({
        title: track.title,
        body: track.artist || 'Unknown Artist',
        icon: API + '/api/cover'
      });
    } else if ('Notification' in window && Notification.permission === 'granted') {
      try { new Notification(track.title, { body: track.artist || 'Unknown Artist', icon: API + '/api/cover', silent: true }); } catch {}
    }
  }
}

function clearTrackInfo() {
  document.getElementById('track-title').textContent = 'No track';
  document.getElementById('track-artist').textContent = '\u2014';
  document.getElementById('track-album').textContent = '\u2014';
  document.title = '1028 Music Player';
  updateMediaSession(null);
}

function updatePlayButton() {
  const playIcon = document.getElementById('play-icon');
  const pauseIcon = document.getElementById('pause-icon');
  playIcon.style.display = state.playing ? 'none' : 'block';
  pauseIcon.style.display = state.playing ? 'block' : 'none';
}

function updateProgress() {
  const pct = state.duration > 0 ? (state.position / state.duration) * 100 : 0;
  document.getElementById('progress-fill').style.width = `${Math.min(pct, 100)}%`;
  document.getElementById('progress-thumb').style.left = `${Math.min(pct, 100)}%`;
  document.getElementById('time-current').textContent = fmtTime(state.position);
  document.getElementById('time-total').textContent = fmtTime(state.duration);
}

function updateVolume() {
  document.getElementById('vol-slider').value = state.volume;
  document.getElementById('vol-label').textContent = state.volume;
}

function updatePlaylistHighlight() {
  document.querySelectorAll('.pl-item').forEach((el, i) => {
    el.classList.toggle('active', i === state.currentIndex);
  });
}

function updateRepeatBadge() {
  const badge = document.getElementById('repeat-indicator');
  if (!badge) return;
  const labels = { order: '\u25B6 order', shuffle: '\u{1F500} shuffle', random: '\u{1F3B2} random', loop: '\u{1F501} loop', track: '\u{1F501} track', play_track: '\u25B6 once' };
  badge.textContent = labels[state.repeat] || state.repeat;
  badge.title = `Repeat: ${state.repeat}`;

  // Update repeat icon in controls
  const btn = document.getElementById('btn-repeat');
  if (btn) {
    const iconMap = { loop: 'repeat', order: 'playlist_play', shuffle: 'shuffle', track: 'repeat_one', random: 'shuffle' };
    const iconName = iconMap[state.repeat] || 'repeat';
    btn.innerHTML = ICONS[iconName] || ICONS.repeat;
  }
}

function fmtTime(s) {
  if (!s || isNaN(s)) return '0:00';
  const m = Math.floor(s / 60);
  const sec = Math.floor(s % 60);
  return `${m}:${sec.toString().padStart(2, '0')}`;
}

// ===== Playlist Rendering =====
let dragFromIndex = null;

// Playlist sort state
let _plSortMode = 'default';
let _plFilterMode = 'all'; // 'all' | 'favourites'

function renderPlaylist() {
  const container = document.getElementById('playlist');
  const filter = (document.getElementById('pl-search')?.value || '').toLowerCase();

  // Sort tracks
  let tracks = [...state.playlist];
  if (_plSortMode !== 'default') {
    tracks.sort((a, b) => {
      const va = (a[_plSortMode] || '').toLowerCase();
      const vb = (b[_plSortMode] || '').toLowerCase();
      return va.localeCompare(vb);
    });
  }

  // Filter tracks
  if (_plFilterMode === 'favourites') {
    tracks = tracks.filter(t => t.is_favourite);
  }

  let html = '';
  tracks.forEach((track, displayIdx) => {
    const realIdx = state.playlist.indexOf(track);
    const title = track.title || track.file_path.split(/[/\\]/).pop();
    const artist = track.artist || 'Unknown';
    if (filter && !title.toLowerCase().includes(filter) && !artist.toLowerCase().includes(filter) && !(track.album || '').toLowerCase().includes(filter)) return;
    const cueTag = track.is_cue ? '<span class="cue-badge">CUE</span>' : '';
    const favClass = track.is_favourite ? ' pl-fav' : '';
    html += `<div class="pl-item${realIdx === state.currentIndex ? ' active' : ''}${favClass}" draggable="true" data-index="${realIdx}">
      <span class="pl-idx">${displayIdx + 1}</span>
      <div class="pl-info">
        <div class="pl-title">${cueTag}${escHtml(title)}</div>
        <div class="pl-artist">${escHtml(artist)}</div>
      </div>
    </div>`;
  });
  if (!html) html = '<div class="pl-empty">' + (filter ? 'No matching tracks' : (tracks.length === 0 ? 'Playlist is empty' : 'No tracks match filter')) + '</div>';

  // Append play queue if not filtering
  if (!filter && state._playQueue?.length) {
    html += '<div class="pq-header">Play Queue</div>';
    state._playQueue.forEach((item, qi) => {
      const t = item.track;
      html += `<div class="pq-item" data-qi="${qi}" data-pl-index="${item.index}">
        <span class="pq-idx">${qi + 1}</span>
        <span class="pq-title">${escHtml(t.title || '(unknown)')}</span>
        <span class="pq-remove" title="Remove from queue">${ICONS.close}</span>
      </div>`;
    });
  }

  container.innerHTML = html;

  // Bind sort button
  const sortBtn = document.getElementById('pl-sort-btn');
  if (sortBtn) {
    sortBtn.onclick = () => {
      const modes = ['default', 'title', 'artist', 'album'];
      const next = modes[(modes.indexOf(_plSortMode) + 1) % modes.length];
      _plSortMode = next;
      sortBtn.textContent = next === 'default' ? '⇕' : next[0].toUpperCase();
      sortBtn.title = `Sort: ${next}`;
      sortBtn.classList.toggle('active', next !== 'default');
      renderPlaylist();
    };
  }

  // Bind filter button
  const filterBtn = document.getElementById('pl-filter-btn');
  if (filterBtn) {
    filterBtn.onclick = () => {
      _plFilterMode = _plFilterMode === 'all' ? 'favourites' : 'all';
      filterBtn.classList.toggle('active', _plFilterMode === 'favourites');
      filterBtn.title = _plFilterMode === 'favourites' ? 'Show: Favourites' : 'Show: All';
      renderPlaylist();
    };
  }

  // Click to jump
  container.querySelectorAll('.pl-item').forEach(el => {
    el.addEventListener('click', (e) => {
      if (e._dragged) return;
      cmd(`jump ${el.dataset.index}`);
    });
  });

  // Play queue: click to jump, remove
  container.querySelectorAll('.pq-item').forEach(el => {
    el.addEventListener('click', (e) => {
      if (e.target.closest('.pq-remove')) {
        // Remove from queue — we clear the whole queue for simplicity
        state._playQueue = [];
        renderPlaylist();
        return;
      }
      cmd(`jump ${el.dataset.plIndex}`);
    });
  });

  // Drag & drop
  container.querySelectorAll('.pl-item').forEach(el => {
    el.addEventListener('dragstart', (e) => {
      state._dragFrom = parseInt(el.dataset.index);
      el.classList.add('dragging');
      e.dataTransfer.effectAllowed = 'move';
      e.dataTransfer.setData('text/plain', el.dataset.index);
    });
    el.addEventListener('dragend', () => {
      el.classList.remove('dragging');
      container.querySelectorAll('.pl-item').forEach(i => i.classList.remove('drag-over'));
    });
    el.addEventListener('dragover', (e) => {
      e.preventDefault();
      e.dataTransfer.dropEffect = 'move';
      container.querySelectorAll('.pl-item').forEach(i => i.classList.remove('drag-over'));
      el.classList.add('drag-over');
    });
    el.addEventListener('dragleave', () => {
      el.classList.remove('drag-over');
    });
    el.addEventListener('drop', (e) => {
      e.preventDefault();
      el.classList.remove('drag-over');
      const fromIdx = state._dragFrom;
      const toIdx = parseInt(el.dataset.index);
      if (fromIdx === null || fromIdx === undefined || fromIdx === toIdx) return;
      // Optimistically reorder UI
      const items = [...state.playlist];
      const [moved] = items.splice(fromIdx, 1);
      items.splice(toIdx, 0, moved);
      state.playlist = items;
      // Adjust currentIndex
      if (state.currentIndex === fromIdx) {
        state.currentIndex = toIdx;
      } else if (fromIdx < state.currentIndex && toIdx >= state.currentIndex) {
        state.currentIndex--;
      } else if (fromIdx > state.currentIndex && toIdx <= state.currentIndex) {
        state.currentIndex++;
      }
      renderPlaylist();
      updatePlaylistHighlight();
      api('POST', '/api/playlist/reorder', { from: fromIdx, to: toIdx });
    });
  });
}

// ===== Controls =====
function bindControls() {
  // Playback
  byId('btn-play')?.addEventListener('click', () => cmd('pause'));
  byId('btn-next')?.addEventListener('click', () => cmd('next'));
  byId('btn-prev')?.addEventListener('click', () => cmd('prev'));
  byId('btn-stop')?.addEventListener('click', () => cmd('stop'));

  // Function buttons
  byId('btn-repeat')?.addEventListener('click', () => cycleRepeatMode());
  byId('btn-shuffle')?.addEventListener('click', () => cmd('repeat shuffle'));
  byId('btn-favourite')?.addEventListener('click', () => cmd('favourite'));
  byId('btn-lyrics')?.addEventListener('click', toggleLyricsSection);
  byId('btn-equalizer')?.addEventListener('click', openEqualizer);
  byId('btn-ab-repeat')?.addEventListener('click', () => cmd('ab-repeat'));
  byId('btn-mini-mode')?.addEventListener('click', () => sendTauriCommand('minimode'));
  byId('btn-fullscreen')?.addEventListener('click', toggleFullscreen);
  byId('btn-dark-mode')?.addEventListener('click', toggleDarkMode);
  byId('btn-settings')?.addEventListener('click', () => {
    if (window._useDlgSettings) showSettingsDialog();
    else switchTab('settings');
  });
  byId('sidebar-toggle')?.addEventListener('click', toggleSidebarDrawer);

  // Download lyrics
  byId('btn-dl-lyric')?.addEventListener('click', async () => {
    const btn = byId('btn-dl-lyric');
    btn.textContent = '...';
    btn.disabled = true;
    try {
      await api('POST', '/api/lyric/search', {});
      fetchLyrics();
    } catch {}
    setTimeout(() => { btn.textContent = '\u21E9 DL'; btn.disabled = false; }, 2000);
  });

  // Playlist search
  byId('pl-search')?.addEventListener('input', () => renderPlaylist());

  // OS file drag-and-drop
  document.addEventListener('dragover', (e) => { e.preventDefault(); document.body.classList.add('drag-over'); });
  document.addEventListener('dragleave', (e) => {
    if (!e.relatedTarget || !document.body.contains(e.relatedTarget)) document.body.classList.remove('drag-over');
  });
  document.addEventListener('drop', (e) => {
    e.preventDefault();
    document.body.classList.remove('drag-over');
    // In Tauri mode, the backend handles drops. In dev mode, show hint.
    if (!window.__TAURI__ && e.dataTransfer?.files?.length > 0) {
      showToast('In Tauri mode, drag files here to add to playlist', 'info');
    }
  });

  // Title bar
  byId('tb-menu-btn')?.addEventListener('click', () => {
    const firstTrigger = document.querySelector('.menu-trigger');
    if (firstTrigger) toggleMenu(MENU_CONFIG[0].id, firstTrigger);
  });
  byId('tb-minimize')?.addEventListener('click', minimizeWindow);
  byId('tb-maximize')?.addEventListener('click', maximizeWindow);
  byId('tb-close')?.addEventListener('click', closeWindow);

  // Volume
  const volSlider = document.getElementById('vol-slider');
  volSlider.addEventListener('input', () => {
    const v = parseInt(volSlider.value);
    state.volume = v;
    document.getElementById('vol-label').textContent = v;
  });
  volSlider.addEventListener('change', () => {
    cmd(`volume set ${volSlider.value}`);
  });
}

function byId(id) { return document.getElementById(id); }

function cycleRepeatMode() {
  const modes = ['loop', 'order', 'shuffle', 'track'];
  const idx = modes.indexOf(state.repeat);
  const next = modes[(idx + 1) % modes.length];
  cmd(`repeat ${next}`);
}

function toggleLyricsSection() {
  const section = document.getElementById('lyric-section');
  if (section) {
    const hidden = section.style.display === 'none';
    section.style.display = hidden ? '' : 'none';
    byId('btn-lyrics')?.classList.toggle('active', hidden);
  }
}

function toggleDarkMode() {
  const isDark = document.body.classList.toggle('dark-mode');
  byId('btn-dark-mode')?.classList.toggle('active', isDark);
  if (isDark) {
    applyTheme('midnight');
  } else {
    applyTheme(localStorage.getItem('mp_theme') || 'default');
  }
}

// Tauri window controls
function minimizeWindow() {
  if (ELECTRON && electronAPI.windowMinimize) {
    electronAPI.windowMinimize();
  } else if (window.__TAURI__?.window) {
    window.__TAURI__.window.getCurrent().minimize().catch(() => {});
  }
}
function maximizeWindow() {
  if (ELECTRON && electronAPI.windowMaximize) {
    electronAPI.windowMaximize();
  } else if (window.__TAURI__?.window) {
    const w = window.__TAURI__.window.getCurrent();
    w.isMaximized().then(m => m ? w.unmaximize() : w.maximize()).catch(() => {});
  }
}
function closeWindow() {
  if (ELECTRON && electronAPI.windowClose) {
    electronAPI.windowClose();
  } else if (window.__TAURI__?.window) {
    window.__TAURI__.window.getCurrent().close().catch(() => {});
  } else {
    window.close();
  }
}

// Progress bar click & drag to seek
const progressBar = document.getElementById('progress-bar');
progressBar.addEventListener('click', (e) => {
  if (state.isDragging) return; // handled by drag
  if (!state.duration) return;
  const rect = progressBar.getBoundingClientRect();
  const pct = (e.clientX - rect.left) / rect.width;
  const secs = Math.floor(pct * state.duration);
  cmd(`seek ${secs}`);
});

progressBar.addEventListener('mousedown', (e) => {
  if (!state.duration) return;
  e.preventDefault();
  state.isDragging = true;
  progressBar.classList.add('dragging');
  updateProgressFromEvent(e);

  document.addEventListener('mousemove', onProgressDrag);
  document.addEventListener('mouseup', onProgressDragEnd);
});

function updateProgressFromEvent(e) {
  const rect = progressBar.getBoundingClientRect();
  const pct = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
  const secs = pct * state.duration;
  state.dragPosition = secs;
  document.getElementById('progress-fill').style.width = `${pct * 100}%`;
  document.getElementById('progress-thumb').style.left = `${pct * 100}%`;
  document.getElementById('time-current').textContent = fmtTime(secs);
}

function onProgressDrag(e) {
  if (!state.isDragging) return;
  updateProgressFromEvent(e);
}

function onProgressDragEnd(e) {
  if (!state.isDragging) return;
  state.isDragging = false;
  progressBar.classList.remove('dragging');
  document.removeEventListener('mousemove', onProgressDrag);
  document.removeEventListener('mouseup', onProgressDragEnd);

  if (state.dragPosition !== undefined) {
    cmd(`seek ${Math.floor(state.dragPosition)}`);
    state.dragPosition = undefined;
  }
}

// Repeat mode cycle
document.getElementById('repeat-indicator').addEventListener('click', () => {
  const modes = ['loop', 'order', 'shuffle', 'random', 'track'];
  const idx = modes.indexOf(state.repeat);
  const next = modes[(idx + 1) % modes.length];
  cmd(`repeat ${next}`);
});

// ===== Sidebar Tabs =====
document.querySelectorAll('.tab-btn').forEach(btn => {
  btn.addEventListener('click', () => {
    document.querySelectorAll('.tab-btn').forEach(b => b.classList.remove('active'));
    document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
    btn.classList.add('active');
    document.getElementById(`tab-${btn.dataset.tab}`).classList.add('active');
    if (btn.dataset.tab === 'media') loadMediaArtists();
    if (btn.dataset.tab === 'settings') loadSettings();
  });
});

// ===== Keyboard shortcuts =====
document.addEventListener('keydown', (e) => {
  if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;
  // Check custom hotkeys first
  const parts = [];
  if (e.ctrlKey) parts.push('Ctrl');
  if (e.altKey) parts.push('Alt');
  if (e.shiftKey) parts.push('Shift');
  const key = e.code === 'Space' ? 'Space' : e.key.length === 1 ? e.key.toUpperCase() : e.key;
  if (!['Control', 'Alt', 'Shift', 'Meta'].includes(e.key)) parts.push(key);
  const combo = parts.join('+');
  const hotkeys = { ...DEFAULT_HOTKEYS, ...loadHotkeys() };
  const action = hotkeys[combo];
  if (action) {
    e.preventDefault();
    switch (action) {
      case 'pause': cmd('pause'); break;
      case 'prev': cmd('prev'); break;
      case 'next': cmd('next'); break;
      case 'stop': cmd('stop'); break;
      case 'volume_up': cmd(`volume set ${Math.min(state.volume + 5, 100)}`); break;
      case 'volume_down': cmd(`volume set ${Math.max(state.volume - 5, 0)}`); break;
      case 'fullscreen': toggleFullscreen(); break;
      case 'open': cmd('open'); break;
      case 'open_folder': cmd('open folder'); break;
      case 'media_lib': switchTab('media'); break;
      case 'cycle_layout': cycleLayout(); break;
      case 'mini_mode': sendTauriCommand('minimode'); break;
      case 'playlist_save': cmd('playlist save'); break;
      case 'shortcuts': showShortcutsDialog(); break;
    }
    return;
  }
  // Legacy key handling
  switch (e.code) {
    case 'ArrowLeft': cmd('seek -5 --relative'); break;
    case 'ArrowRight': cmd('seek +5 --relative'); break;
  }
});

// ===== Spectrum (Upgraded) =====
const canvas = document.getElementById('spectrum');
const ctx = canvas.getContext('2d');
let _peakHold = new Array(64).fill(0);
let _spectrumIdleTime = 0;

function getSpectrumConfig() {
  const cfg = settingsCache || {};
  const app = cfg.appearance || {};
  return {
    barCount: app.spectrum_columns || 64,
    style: localStorage.getItem('mp_spectrum_visual_style') || 'modern',
    reflection: localStorage.getItem('mp_spectrum_reflection') !== 'false',
    fixedWidth: localStorage.getItem('mp_spectrum_fixed_width') === 'true',
    height: parseInt(localStorage.getItem('mp_spectrum_height') || '80'),
  };
}

function drawSpectrum(data, peaks) {
  const cfg = getSpectrumConfig();
  const w = canvas.width;
  const h = canvas.height;
  ctx.clearRect(0, 0, w, h);

  const count = cfg.barCount;
  if (_peakHold.length !== count) _peakHold = new Array(count).fill(0);

  // Idle animation when no data
  if (!data || data.length === 0) {
    _spectrumIdleTime++;
    for (let i = 0; i < count; i++) {
      const wave = Math.sin(i * 0.3 + _spectrumIdleTime * 0.05) * 0.5 + 0.5;
      const bh = Math.max(wave * h * 0.15, 2);
      const barW = cfg.fixedWidth ? Math.max(2, (w / count) - 1) : w / count;
      const gap = cfg.fixedWidth ? 1 : 0;
      const x = i * (barW + gap);
      const grad = ctx.createLinearGradient(0, h, 0, 0);
      grad.addColorStop(0, 'var(--accent2)');
      grad.addColorStop(1, 'var(--accent)');
      ctx.fillStyle = grad;
      ctx.beginPath();
      const r = Math.min(2, barW / 2);
      ctx.moveTo(x + r, h - bh);
      ctx.lineTo(x + barW - r, h - bh);
      ctx.quadraticCurveTo(x + barW, h - bh, x + barW, h - bh + r);
      ctx.lineTo(x + barW, h);
      ctx.lineTo(x, h);
      ctx.lineTo(x, h - bh + r);
      ctx.quadraticCurveTo(x, h - bh, x + r, h - bh);
      ctx.fill();
    }
    _peakHold.fill(0);
    return;
  }
  _spectrumIdleTime = 0;

  const step = Math.max(Math.floor(data.length / count), 1);
  const barW = cfg.fixedWidth ? Math.max(2, (w / count) - 1) : w / count;
  const gap = cfg.fixedWidth ? 1 : 0;

  // Draw bars
  for (let i = 0; i < count; i++) {
    let val = 0;
    let c = 0;
    for (let j = 0; j < step && i * step + j < data.length; j++) {
      val += data[i * step + j];
      c++;
    }
    val = c > 0 ? val / c : 0;
    const db = Math.min(val * 3, 1);
    const bh = Math.max(db * h, 1);
    const x = i * (barW + gap);

    if (cfg.style === 'modern') {
      // Modern: rounded bars with warm gradient
      const grad = ctx.createLinearGradient(0, h, 0, 0);
      grad.addColorStop(0, '#533483');
      grad.addColorStop(0.5, '#e94560');
      grad.addColorStop(1, '#ff6b6b');
      ctx.fillStyle = grad;
      const radius = Math.min(3, barW / 2);
      ctx.beginPath();
      ctx.moveTo(x + radius, h - bh);
      ctx.lineTo(x + barW - radius, h - bh);
      ctx.quadraticCurveTo(x + barW, h - bh, x + barW, h - bh + radius);
      ctx.lineTo(x + barW, h);
      ctx.lineTo(x, h);
      ctx.lineTo(x, h - bh + radius);
      ctx.quadraticCurveTo(x, h - bh, x + radius, h - bh);
      ctx.fill();
    } else {
      // Classic: flat bars with cool blue gradient
      const grad = ctx.createLinearGradient(0, h, 0, 0);
      grad.addColorStop(0, '#0d47a1');
      grad.addColorStop(0.6, '#00bcd4');
      grad.addColorStop(1, '#00e5ff');
      ctx.fillStyle = grad;
      ctx.fillRect(x, h - bh, barW, bh);
    }

    // Decaying peak hold
    if (db > _peakHold[i]) _peakHold[i] = db;
    else _peakHold[i] = Math.max(0, _peakHold[i] - 0.015);
    const peakY = h - _peakHold[i] * h;
    ctx.fillStyle = 'rgba(255,255,255,0.85)';
    ctx.fillRect(x + 1, peakY - 1, barW - 2, 2);

    // Peak dot from server
    if (peaks && peaks[i] !== undefined) {
      const pY = h - Math.min(peaks[i] * 3, 1) * h;
      ctx.fillStyle = cfg.style === 'modern' ? '#ff6b6b' : '#69f0ae';
      ctx.fillRect(x + 1, pY - 2, barW - 2, 1);
    }
  }

  // Reflection effect (gradient overlay at bottom)
  if (cfg.reflection && cfg.style === 'modern') {
    const reflGrad = ctx.createLinearGradient(0, h * 0.65, 0, h);
    reflGrad.addColorStop(0, 'rgba(83,52,131,0)');
    reflGrad.addColorStop(0.5, 'rgba(233,69,96,0.08)');
    reflGrad.addColorStop(1, 'rgba(255,107,107,0.18)');
    ctx.fillStyle = reflGrad;
    ctx.fillRect(0, h * 0.65, w, h * 0.35);
  } else if (cfg.reflection && cfg.style === 'classic') {
    const reflGrad = ctx.createLinearGradient(0, h * 0.7, 0, h);
    reflGrad.addColorStop(0, 'rgba(13,71,161,0)');
    reflGrad.addColorStop(1, 'rgba(0,229,255,0.12)');
    ctx.fillStyle = reflGrad;
    ctx.fillRect(0, h * 0.7, w, h * 0.3);
  }
}

// ===== WebSocket Spectrum =====
let ws = null;

function connectWS() {
  if (OHOS) return;
  if (ws && ws.readyState === WebSocket.OPEN) return;
  try {
    ws = new WebSocket(WS_URL);
    ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        if (data.spectrum) {
          drawSpectrum(data.spectrum, data.peaks);
        }
      } catch (e) {}
    };
    ws.onclose = () => {
      ws = null;
      setTimeout(connectWS, 2000);
    };
    ws.onerror = () => {
      ws = null;
    };
  } catch (e) {
    setTimeout(connectWS, 2000);
  }
}

// ===== Media Library (11 category tabs) =====
const MEDIA_CATEGORIES = [
  { id: 'all', label: '全部曲目', icon: 'music_note', endpoint: '/api/media/all' },
  { id: 'artists', label: '艺术家', icon: 'artist', endpoint: '/api/media/artists' },
  { id: 'albums', label: '专辑', icon: 'album', endpoint: '/api/media/albums' },
  { id: 'genres', label: '流派', icon: 'lyrics', endpoint: '/api/media/genres' },
  { id: 'years', label: '年份', icon: 'sort', endpoint: '/api/media/years' },
  { id: 'filetypes', label: '文件类型', icon: 'folder', endpoint: '/api/media/filetypes' },
  { id: 'bitrates', label: '比特率', icon: 'equalizer', endpoint: '/api/media/bitrates' },
  { id: 'favourites', label: '收藏', icon: 'favorite', endpoint: '/api/media/favourites' },
  { id: 'recent', label: '最近播放', icon: 'history', endpoint: '/api/media/recent' },
  { id: 'browse', label: '浏览文件夹', icon: 'folder', endpoint: '' },
];

let mediaCategory = 'all';
let mediaBreadcrumb = [];
let folderStack = [];

function renderMediaCategories() {
  const bar = document.getElementById('media-categories');
  if (!bar) return;
  let html = '<div class="media-cat-scroll">';
  for (const cat of MEDIA_CATEGORIES) {
    const active = cat.id === mediaCategory ? ' active' : '';
    const icon = ICONS[cat.icon] || ICONS.music_note;
    html += `<button class="media-cat-btn${active}" data-cat="${cat.id}">${icon}<span>${cat.label}</span></button>`;
  }
  html += '</div>';
  bar.innerHTML = html;

  bar.querySelectorAll('.media-cat-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      mediaCategory = btn.dataset.cat;
      mediaBreadcrumb = [];
      renderMediaCategories();
      loadMediaCategory();
    });
  });
}

async function loadMediaCategory() {
  const searchVal = document.getElementById('media-search')?.value?.trim() || '';
  const container = document.getElementById('media-content');
  const backBtn = document.getElementById('media-back-btn');
  if (!container) return;

  backBtn.style.display = mediaBreadcrumb.length > 0 ? 'inline-block' : 'none';

  const cat = MEDIA_CATEGORIES.find(c => c.id === mediaCategory);
  if (!cat) return;

  // If search box has text and we're on 'all' category, do backend full-text search
  if (searchVal && mediaCategory === 'all' && mediaBreadcrumb.length === 0) {
    try {
      const results = await api('POST', '/api/media/search', { keyword: searchVal });
      if (results && results.length > 0) {
        renderMediaAllTracks(results, container, '');
        return;
      }
    } catch {}
    container.innerHTML = '<div class="media-empty">无匹配结果</div>';
    return;
  }

  const data = await api('GET', cat.endpoint);
  if (!data) { container.innerHTML = '<div class="media-empty">暂无数据</div>'; return; }

  // Render based on category type
  if (cat.id === 'all') renderMediaAllTracks(data, container, searchVal);
  else if (cat.id === 'artists') renderMediaList(data.artists, container, 'artist', searchVal);
  else if (cat.id === 'albums') renderMediaAlbums(data.albums, container, searchVal);
  else if (cat.id === 'genres') renderMediaList(data, container, 'genre', searchVal);
  else if (cat.id === 'years') renderMediaList(data.years, container, 'year', searchVal);
  else if (cat.id === 'filetypes') renderMediaList(data.types, container, 'filetype', searchVal);
  else if (cat.id === 'bitrates') renderMediaList(data.bitrates, container, 'bitrate', searchVal);
  else if (cat.id === 'favourites') renderMediaAllTracks(data, container, searchVal);
  else if (cat.id === 'recent') renderMediaAllTracks(data, container, searchVal);
  else if (cat.id === 'browse') renderFolderBrowser(container);
}

function renderMediaList(items, container, type, search) {
  if (!items || items.length === 0) {
    container.innerHTML = '<div class="media-empty">暂无数据</div>';
    return;
  }
  const filtered = search ? items.filter(i => String(i).toLowerCase().includes(search.toLowerCase())) : items;
  if (filtered.length === 0) {
    container.innerHTML = '<div class="media-empty">无匹配结果</div>';
    return;
  }
  let html = '';
  for (const item of filtered) {
    const label = String(item);
    html += `<div class="media-item" data-type="${type}" data-value="${escHtml(label)}">
      <span class="mi-icon">${ICONS.music_note}</span>
      <span class="mi-name">${escHtml(label)}</span>
      <span class="mi-arrow">${ICONS.skip_next}</span>
    </div>`;
  }
  container.innerHTML = html;

  container.querySelectorAll('.media-item').forEach(el => {
    el.addEventListener('click', () => drillMediaCategory(el.dataset.type, el.dataset.value));
  });
}

function renderMediaAlbums(albums, container, search) {
  if (!albums || albums.length === 0) {
    container.innerHTML = '<div class="media-empty">暂无专辑</div>';
    return;
  }
  let html = '';
  for (const album of albums) {
    if (search && !album.name.toLowerCase().includes(search.toLowerCase())) continue;
    html += `<div class="media-item" data-type="album_artist" data-album="${escHtml(album.name)}">
      <span class="mi-icon">${ICONS.album}</span>
      <span class="mi-name">${escHtml(album.name)}</span>
      <span class="mi-count">${album.track_count || ''}</span>
    </div>`;
  }
  if (!html) { container.innerHTML = '<div class="media-empty">无匹配结果</div>'; return; }
  container.innerHTML = html;

  container.querySelectorAll('.media-item').forEach(el => {
    el.addEventListener('click', () => {
      mediaBreadcrumb.push(mediaCategory);
      mediaCategory = 'tracks_by_album';
      renderMediaCategories();
      loadTracksByAlbum(el.dataset.album);
    });
  });
}

function renderMediaAllTracks(data, container, search) {
  const tracks = data.tracks || data;
  if (!tracks || tracks.length === 0) {
    container.innerHTML = '<div class="media-empty">暂无曲目</div>';
    return;
  }
  const filtered = search ? tracks.filter(t =>
    t.title?.toLowerCase().includes(search) ||
    t.artist?.toLowerCase().includes(search) ||
    t.album?.toLowerCase().includes(search)
  ) : tracks;
  if (filtered.length === 0) {
    container.innerHTML = '<div class="media-empty">无匹配结果</div>';
    return;
  }
  let html = '';
  for (const track of filtered.slice(0, 500)) {
    html += `<div class="media-item" data-action="play" data-file="${escHtml(track.file_path)}">
      <span class="mi-icon">${ICONS.music_note}</span>
      <span class="mi-name">${escHtml(track.title || track.file_path.split(/[/\\]/).pop())}</span>
      <span class="mi-artist">${escHtml(track.artist || '')}</span>
      <span class="mi-count">${fmtTime(track.duration_secs)}</span>
    </div>`;
  }
  container.innerHTML = html;

  container.querySelectorAll('.media-item[data-action="play"]').forEach(el => {
    el.addEventListener('click', () => cmd(`play "${el.dataset.file}"`));
  });
}

function renderDrillData(entries, title) {
  const container = document.getElementById('media-content');
  let html = `<div class="media-item media-title-row"><span class="mi-name" style="color:var(--accent);font-weight:600">${escHtml(title)}</span></div>`;
  for (const e of entries) {
    html += `<div class="media-item" data-action="play" data-file="${escHtml(e.file_path)}">
      <span class="mi-icon">${ICONS.music_note}</span>
      <span class="mi-name">${escHtml(e.title || e.file_path.split(/[/\\]/).pop())}</span>
      <span class="mi-artist">${escHtml(e.artist || '')}</span>
      <span class="mi-count">${fmtTime(e.duration_secs)}</span>
    </div>`;
  }
  container.innerHTML = html;
  container.querySelectorAll('.media-item[data-action="play"]').forEach(el => {
    el.addEventListener('click', () => cmd(`play "${el.dataset.file}"`));
  });
}

async function drillMediaCategory(type, value) {
  mediaBreadcrumb.push(mediaCategory);

  if (type === 'artist') {
    mediaCategory = 'tracks_by_artist';
    renderMediaCategories();
    const data = await api('GET', `/api/media/albums/${encodeURIComponent(value)}`);
    if (data?.albums) {
      let html = `<div class="media-item media-title-row"><span class="mi-name" style="color:var(--accent)">${escHtml(value)}</span></div>`;
      for (const a of data.albums) {
        html += `<div class="media-item" data-type="album_tracks" data-artist="${escHtml(value)}" data-album="${escHtml(a.name)}">
          <span class="mi-icon">${ICONS.album}</span>
          <span class="mi-name">${escHtml(a.name)}</span>
          <span class="mi-count">${a.track_count}</span>
        </div>`;
      }
      document.getElementById('media-content').innerHTML = html;
      document.getElementById('media-content').querySelectorAll('.media-item[data-type="album_tracks"]').forEach(el => {
        el.addEventListener('click', () => loadArtistAlbumTracks(el.dataset.artist, el.dataset.album));
      });
    }
  } else if (type === 'genre') {
    mediaCategory = 'tracks_by_genre';
    renderMediaCategories();
    const data = await api('GET', `/api/media/genre/${encodeURIComponent(value)}`);
    renderDrillData(data?.tracks || [], `流派: ${value}`);
  } else if (type === 'year') {
    mediaCategory = 'tracks_by_year';
    renderMediaCategories();
    const data = await api('GET', `/api/media/year/${value}`);
    renderDrillData(data?.tracks || [], `年份: ${value}`);
  } else if (type === 'filetype') {
    mediaCategory = 'tracks_by_type';
    renderMediaCategories();
    const data = await api('GET', `/api/media/type/${encodeURIComponent(value)}`);
    renderDrillData(data?.tracks || [], `类型: ${value}`);
  } else if (type === 'bitrate') {
    mediaCategory = 'tracks_by_bitrate';
    renderMediaCategories();
    const data = await api('GET', `/api/media/bitrate/${value}`);
    renderDrillData(data?.tracks || [], `比特率: ${value}kbps`);
  }

  document.getElementById('media-back-btn').style.display = 'inline-block';
}

async function loadArtistAlbumTracks(artist, album) {
  mediaBreadcrumb.push(mediaCategory);
  mediaCategory = 'tracks_detail';
  renderMediaCategories();
  const data = await api('GET', `/api/media/tracks/${encodeURIComponent(artist)}/${encodeURIComponent(album)}`);
  renderDrillData(data?.tracks || [], `${artist} - ${album}`);
}

async function loadTracksByAlbum(album) {
  const data = await api('GET', '/api/media/all');
  const tracks = data?.tracks?.filter(t => t.album === album) || [];
  renderDrillData(tracks, `专辑: ${album}`);
  document.getElementById('media-back-btn').style.display = 'inline-block';
}

function initMediaLibrary() {
  renderMediaCategories();
  loadMediaCategory();

  // Media library back button
  document.getElementById('media-back-btn')?.addEventListener('click', () => {
    if (folderStack.length > 0) {
      folderStack.pop();
      renderFolderBrowser(document.getElementById('media-content'));
      return;
    }
    if (mediaBreadcrumb.length > 0) {
      mediaBreadcrumb.pop();
      loadMediaCategory();
    }
  });

  document.getElementById('media-search')?.addEventListener('input', (e) => {
    loadMediaCategory();
  });
}

// ----- Folder Browser -----
let folderCurrentPath = '';

async function renderFolderBrowser(container) {
  const path = folderStack.length > 0 ? folderStack[folderStack.length - 1] : '';
  folderCurrentPath = path;
  const backBtn = document.getElementById('media-back-btn');
  backBtn.style.display = folderStack.length > 0 ? 'inline-block' : 'none';

  // Breadcrumb
  let breadcrumbHtml = '';
  if (folderStack.length > 0) {
    const parts = [{ name: 'Root', path: '' }];
    for (let i = 0; i < folderStack.length; i++) {
      parts.push({ name: folderStack[i].split(/[/\\]/).filter(Boolean).pop() || folderStack[i], path: folderStack[i] });
    }
    breadcrumbHtml = '<div class="fb-breadcrumb">';
    breadcrumbHtml += parts.map((p, idx) =>
      `<span class="fb-crumb${idx === parts.length - 1 ? ' active' : ''}" data-crumb="${idx}">${idx > 0 ? ' › ' : ''}${escHtml(p.name)}</span>`
    ).join('');
    breadcrumbHtml += '</div>';
  }

  if (!path) {
    // Show root drives/volumes
    let drives = [];
    if (navigator.userAgent.includes('Windows')) {
      drives = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split('').map(l => l + ':\\').filter(d => {
        try { return true; } catch { return false; }
      });
    }
    if (drives.length === 0) {
      try {
        const result = await api('POST', '/api/media/browse', { path: '/' });
        if (result) {
          container.innerHTML = breadcrumbHtml + renderFolderTree(result, true);
          bindFolderEvents(container);
          return;
        }
      } catch {}
      drives = ['C:\\', 'D:\\', 'E:\\'];
    }
    const fake = drives.map(d => ({ name: d, path: d, is_dir: true, is_audio: false, size: 0 }));
    container.innerHTML = breadcrumbHtml + renderFolderTree(fake, true);
    bindFolderEvents(container);
    return;
  }

  try {
    const result = await api('POST', '/api/media/browse', { path });
    if (result) {
      container.innerHTML = breadcrumbHtml + renderFolderTree(result, false);
      bindFolderEvents(container);
    } else {
      container.innerHTML = breadcrumbHtml + '<div class="media-empty">Cannot read directory</div>';
    }
  } catch {
    container.innerHTML = breadcrumbHtml + '<div class="media-empty">Error reading directory</div>';
  }

  // Bind breadcrumb clicks
  container.querySelectorAll('.fb-crumb').forEach(el => {
    el.addEventListener('click', () => {
      const idx = parseInt(el.dataset.crumb);
      if (idx === 0) {
        folderStack = [];
      } else {
        folderStack = folderStack.slice(0, idx);
      }
      renderFolderBrowser(container);
    });
  });
}

function renderFolderTree(entries, isRoot) {
  const dirs = entries.filter(e => e.is_dir);
  const files = entries.filter(e => e.is_audio);
  let html = '';
  for (const d of dirs) {
    html += `<div class="fb-item fb-dir" data-path="${escHtml(d.path)}">
      <span class="fb-icon">${ICONS.folder}</span>
      <span class="fb-name">${escHtml(d.name)}</span>
      <span class="fb-arrow">${ICONS.skip_next}</span>
    </div>`;
  }
  for (const f of files) {
    const sizeStr = f.size > 1048576 ? (f.size/1048576).toFixed(1)+'MB' : f.size > 1024 ? (f.size/1024).toFixed(1)+'KB' : f.size+'B';
    html += `<div class="fb-item fb-file" data-path="${escHtml(f.path)}">
      <span class="fb-icon">${ICONS.music_note}</span>
      <span class="fb-name">${escHtml(f.name)}</span>
      <span class="fb-size">${sizeStr}</span>
    </div>`;
  }
  if (!html) html = '<div class="media-empty">(empty)</div>';
  return html;
}

function bindFolderEvents(container) {
  container.querySelectorAll('.fb-dir').forEach(el => {
    el.addEventListener('click', () => {
      folderStack.push(el.dataset.path);
      renderFolderBrowser(document.getElementById('media-content'));
    });
  });
  container.querySelectorAll('.fb-file').forEach(el => {
    el.addEventListener('click', () => {
      cmd(`open "${el.dataset.path}"`);
    });
  });
}

// ===== Themes / Skins =====
// Desktop lyric color presets
const LYRIC_COLORS = {
  default: { color1: '#e94560', color2: 'rgba(255,255,255,0.35)', name: '经典红' },
  blue: { color1: '#4fc3f7', color2: 'rgba(255,255,255,0.35)', name: '天空蓝' },
  green: { color1: '#66bb6a', color2: 'rgba(255,255,255,0.35)', name: '翠绿' },
  purple: { color1: '#bb86fc', color2: 'rgba(255,255,255,0.35)', name: '淡紫' },
  orange: { color1: '#ff7043', color2: 'rgba(255,255,255,0.35)', name: '暖橙' },
};

function applyLyricColor(scheme) {
  const colors = LYRIC_COLORS[scheme];
  if (!colors) return;
  localStorage.setItem('mp_lyric_color', scheme);
  // Apply to mini.html via localStorage (read by mini.html on poll)
}

const THEMES = {
  default: {
    '--bg': '#1a1a2e', '--bg2': '#16213e', '--bg3': '#0f3460',
    '--accent': '#e94560', '--accent2': '#533483',
    '--text': '#eee', '--text2': '#999', '--text3': '#666',
    '--border': '#2a2a4e', '--hover': '#1f2f5f',
    '--success': '#2ecc71', '--warning': '#f39c12',
    name: 'Default Dark',
  },
  ocean: {
    '--bg': '#0a1628', '--bg2': '#0f2740', '--bg3': '#1a3a5c',
    '--accent': '#4fc3f7', '--accent2': '#0288d1',
    '--text': '#e0f7fa', '--text2': '#80deea', '--text3': '#546e7a',
    '--border': '#1a3a5c', '--hover': '#1a3050',
    '--success': '#2ecc71', '--warning': '#f39c12',
    name: 'Ocean Blue',
  },
  forest: {
    '--bg': '#1a2e1a', '--bg2': '#1e3a1e', '--bg3': '#2a5c2a',
    '--accent': '#66bb6a', '--accent2': '#2e7d32',
    '--text': '#e8f5e9', '--text2': '#a5d6a7', '--text3': '#6b8e6b',
    '--border': '#2a5c2a', '--hover': '#234a23',
    '--success': '#2ecc71', '--warning': '#f39c12',
    name: 'Forest Green',
  },
  sunset: {
    '--bg': '#2e1a1a', '--bg2': '#3a1e1a', '--bg3': '#5c2a1a',
    '--accent': '#ff7043', '--accent2': '#d84315',
    '--text': '#fbe9e7', '--text2': '#ffab91', '--text3': '#8d6e63',
    '--border': '#5c2a1a', '--hover': '#4a2318',
    '--success': '#2ecc71', '--warning': '#f39c12',
    name: 'Sunset Orange',
  },
  midnight: {
    '--bg': '#0d0d0d', '--bg2': '#1a1a1a', '--bg3': '#2d2d2d',
    '--accent': '#bb86fc', '--accent2': '#6200ee',
    '--text': '#e0e0e0', '--text2': '#9e9e9e', '--text3': '#616161',
    '--border': '#333333', '--hover': '#252525',
    '--success': '#2ecc71', '--warning': '#f39c12',
    name: 'Midnight Purple',
  },
  cherry_blossom: {
    '--bg': '#2e1a2a', '--bg2': '#3a1e35', '--bg3': '#5c2a4a',
    '--accent': '#ff80ab', '--accent2': '#ff4081',
    '--text': '#fce4ec', '--text2': '#f48fb1', '--text3': '#9c6b7a',
    '--border': '#5c2a4a', '--hover': '#4a2340',
    '--success': '#2ecc71', '--warning': '#f39c12',
    name: 'Cherry Blossom',
  },
  nord: {
    '--bg': '#2e3440', '--bg2': '#3b4252', '--bg3': '#434c5e',
    '--accent': '#88c0d0', '--accent2': '#5e81ac',
    '--text': '#eceff4', '--text2': '#d8dee9', '--text3': '#616e88',
    '--border': '#4c566a', '--hover': '#3b4252',
    '--success': '#a3be8c', '--warning': '#d08770',
    name: 'Nord',
  },
  tokyo_night: {
    '--bg': '#1a1b2e', '--bg2': '#232540', '--bg3': '#2f3158',
    '--accent': '#7aa2f7', '--accent2': '#bb9af7',
    '--text': '#c0caf5', '--text2': '#a9b1d6', '--text3': '#565f89',
    '--border': '#363b6e', '--hover': '#292d4a',
    '--success': '#9ece6a', '--warning': '#e0af68',
    name: 'Tokyo Night',
  },
};

const THEME_VARS = ['--bg','--bg2','--bg3','--accent','--accent2','--text','--text2','--text3','--border','--hover','--success','--warning'];
var customThemes = {};

(function loadCustomThemes() {
  try { customThemes = JSON.parse(localStorage.getItem('mp_custom_themes')) || {}; } catch { customThemes = {}; }
  // Inject custom themes into THEMES
  for (const [k, v] of Object.entries(customThemes)) THEMES['custom_' + k] = v;
})();

function applyTheme(themeName) {
  let theme = THEMES[themeName];
  if (!theme) {
    // Try custom without prefix
    theme = THEMES['custom_' + themeName];
    if (!theme) return;
    themeName = 'custom_' + themeName;
  }
  const root = document.documentElement;
  for (const [key, val] of Object.entries(theme)) {
    if (key === 'name') continue;
    root.style.setProperty(key, val);
  }
  localStorage.setItem('mp_theme', themeName);
  // Persist to backend
  api('POST', '/api/config', { key: 'appearance.theme', value: themeName });
  // Update settings UI if visible
  const sel = document.getElementById('set-theme');
  if (sel) {
    // Ensure custom theme has an option
    if (themeName.startsWith('custom_') && !sel.querySelector(`option[value="${themeName}"]`)) {
      const opt = document.createElement('option');
      opt.value = themeName; opt.textContent = theme.name;
      sel.appendChild(opt);
    }
    sel.value = themeName;
  }
}

async function loadTheme() {
  // Try loading theme from backend config first
  try {
    const res = await api('GET', '/api/config');
    if (res && res.config && res.config.appearance && res.config.appearance.theme) {
      const t = res.config.appearance.theme;
      if (THEMES[t] || THEMES['custom_' + t]) {
        applyTheme(THEMES[t] ? t : 'custom_' + t);
        return;
      }
    }
  } catch (_) {}
  // Fallback to localStorage if backend not available yet
  const saved = localStorage.getItem('mp_theme') || 'default';
  applyTheme(saved);
  // Glass mode
  if (localStorage.getItem('mp_glass') === 'true') {
    document.body.classList.add('glass');
  }
  // Blur intensity
  const blur = localStorage.getItem('mp_blur');
  if (blur) {
    document.documentElement.style.setProperty('--glass-blur', blur + 'px');
  }
  // Panel opacity
  const opacity = localStorage.getItem('mp_opacity');
  if (opacity) {
    document.documentElement.style.setProperty('--panel-opacity', opacity);
  }
}

// ----- Theme Editor -----
const THEME_VAR_LABELS = {
  '--bg': '背景', '--bg2': '背景2', '--bg3': '背景3',
  '--accent': '强调色', '--accent2': '次要强调色',
  '--text': '文字色', '--text2': '文字色2', '--text3': '文字色3',
  '--border': '边框色', '--hover': '悬停色',
  '--success': '成功色', '--warning': '警告色',
};

function showThemeEditor() {
  const currentName = localStorage.getItem('mp_theme') || 'default';
  const currentTheme = THEMES[currentName];
  const vars = currentTheme ? THEME_VARS.filter(v => v in currentTheme) : THEME_VARS;
  const inputs = vars.map(v => `
    <div style="display:flex;align-items:center;gap:8px;margin:4px 0">
      <label style="width:80px;font-size:11px;color:var(--text3)">${THEME_VAR_LABELS[v] || v}</label>
      <input type="color" id="te_${v.replace(/-/g,'_')}" value="${currentTheme?.[v] || '#000'}" style="width:36px;height:28px;padding:0;border:1px solid var(--border);background:none;cursor:pointer">
      <input type="text" id="te_txt_${v.replace(/-/g,'_')}" value="${currentTheme?.[v] || '#000'}" style="flex:1;font-size:11px;padding:2px 4px;background:var(--bg2);color:var(--text);border:1px solid var(--border);border-radius:3px">
    </div>
  `).join('');

  showDialog({
    title: '自定义主题编辑器',
    width: '480px',
    onOpen: function(box) {
      vars.forEach(v => {
        const id = v.replace(/-/g,'_');
        const colorInput = box.querySelector(`#te_${id}`);
        const txtInput = box.querySelector(`#te_txt_${id}`);
        if (colorInput && txtInput) {
          colorInput.addEventListener('input', () => {
            txtInput.value = colorInput.value;
            document.documentElement.style.setProperty(v, colorInput.value);
          });
          txtInput.addEventListener('input', () => {
            if (/^#[0-9a-f]{6}$/i.test(txtInput.value)) {
              colorInput.value = txtInput.value;
              document.documentElement.style.setProperty(v, txtInput.value);
            }
          });
        }
      });
    },
    body: `<div style="max-height:400px;overflow-y:auto">${inputs}</div>`,
    footer: `
      <button id="te-save">保存为...</button>
      <button id="te-reset">重置为当前主题</button>
      <button data-dlg-close>关闭</button>
    `
  });

  setTimeout(() => {
    const saveBtn = document.getElementById('te-save');
    const resetBtn = document.getElementById('te-reset');
    if (saveBtn) saveBtn.onclick = () => {
      const name = prompt('主题名称：');
      if (!name) return;
      const theme = { name };
      for (const v of THEME_VARS) {
        const el = document.getElementById(`te_txt_${v.replace(/-/g,'_')}`);
        if (el) theme[v] = el.value;
      }
      customThemes[name] = theme;
      localStorage.setItem('mp_custom_themes', JSON.stringify(customThemes));
      const key = 'custom_' + name;
      THEMES[key] = theme;
      applyTheme(key);
      const sel = document.getElementById('set-theme');
      if (sel) {
        const opt = document.createElement('option');
        opt.value = key; opt.textContent = name;
        sel.appendChild(opt);
        sel.value = key;
      }
    };
    if (resetBtn) resetBtn.onclick = () => {
      const currentName = localStorage.getItem('mp_theme') || 'default';
      const theme = THEMES[currentName];
      if (!theme) return;
      for (const v of THEME_VARS) {
        if (v in theme) {
          const el = document.getElementById(`te_txt_${v.replace(/-/g,'_')}`);
          const colorEl = document.getElementById(`te_${v.replace(/-/g,'_')}`);
          if (el) el.value = theme[v];
          if (colorEl) colorEl.value = theme[v];
          document.documentElement.style.setProperty(v, theme[v]);
        }
      }
    };
  }, 50);
}

// ----- Theme Editor end -----

// Fullscreen toggle
document.addEventListener('dblclick', (e) => {
  const main = document.getElementById('main');
  if (main.contains(e.target) && !e.target.closest('input,select,button,.ctrl-btn')) {
    toggleFullscreen();
  }
});

function toggleFullscreen() {
  if (!document.fullscreenElement) {
    document.documentElement.requestFullscreen().catch(() => {});
  } else {
    document.exitFullscreen().catch(() => {});
  }
}
let settingsCache = null;

async function loadSettings() {
  const data = await api('GET', '/api/config');
  if (!data || !data.config) return;
  settingsCache = data.config;
  renderSettings(data.config, data.status);
  // Fetch EQ + reverb state
  fetchEqState();
  fetchReverbState();
}

function renderSettings(config, status) {
  const container = document.getElementById('settings-content');
  const eqState = window._eqState || { enabled: false, bands: [] };
  const reverbState = window._reverbState || { enabled: false, mix: 50, time: 100 };
  const currSpeed = status?.speed || 1;
  const currPitch = status?.pitch || 0;
  const html = `
    <div class="settings-group">
      <h3>Playback</h3>
      <div class="setting-row">
        <label>Engine</label>
        <select id="set-engine">
          <option value="bass" ${config.play?.engine === 'bass' ? 'selected' : ''}>BASS</option>
          <option value="ffmpeg" ${config.play?.engine === 'ffmpeg' ? 'selected' : ''}>FFmpeg</option>
        </select>
      </div>
      <div class="setting-row">
        <label>Default Volume</label>
        <input type="number" id="set-volume" value="${config.play?.default_volume ?? 80}" min="0" max="100" />
      </div>
      <div class="setting-row">
        <label>Fade Effect</label>
        <div class="toggle ${config.play?.fade_effect ? 'on' : ''}" id="set-fade" data-key="play.fade_effect"></div>
      </div>
      <div class="setting-row">
        <label>Auto Play on Start</label>
        <div class="toggle ${config.play?.auto_play_when_start ? 'on' : ''}" id="set-auto-play" data-key="play.auto_play_when_start"></div>
      </div>
      <div class="setting-row">
        <label>Merge Same Songs</label>
        <div class="toggle ${config.play?.merge_song_different_versions ? 'on' : ''}" id="set-merge" data-key="play.merge_song_different_versions"></div>
      </div>
      <div class="setting-row">
        <label>Fade Duration (ms)</label>
        <input type="number" id="set-fade-time" value="${config.play?.fade_time ?? 500}" min="0" max="5000" step="100" />
      </div>
      <div class="setting-row">
        <label>Always on Top</label>
        <div class="toggle ${config.play?.always_on_top ? 'on' : ''}" id="set-always-top" data-key="play.always_on_top"></div>
      </div>
      <div class="setting-row">
        <label>Stop on Error</label>
        <div class="toggle ${config.play?.stop_when_error !== false ? 'on' : ''}" id="set-stop-error" data-key="play.stop_when_error"></div>
      </div>
      <div class="setting-row">
        <label>Audio Output</label>
        <select id="set-output-device">
          <option value="-1">Default Device</option>
        </select>
      </div>
      <div class="setting-row">
        <label>Output Mode</label>
        <select id="set-output-mode">
          <option value="directsound" ${config.play?.output_mode === 'directsound' ? 'selected' : ''}>DirectSound</option>
          <option value="wasapi" ${config.play?.output_mode === 'wasapi' ? 'selected' : ''}>WASAPI (Shared)</option>
          <option value="wasapi_exclusive" ${config.play?.output_mode === 'wasapi_exclusive' ? 'selected' : ''}>WASAPI (Exclusive)</option>
        </select>
      </div>
      <div class="setting-row">
        <label>ReplayGain</label>
        <select id="set-replaygain">
          <option value="off" ${config.play?.replaygain === 'off' ? 'selected' : ''}>Off</option>
          <option value="track" ${config.play?.replaygain === 'track' ? 'selected' : ''}>Track Gain</option>
          <option value="album" ${config.play?.replaygain === 'album' ? 'selected' : ''}>Album Gain</option>
        </select>
      </div>
    </div>
    <div class="settings-group">
      <h3>Appearance</h3>
      <div class="setting-row">
        <label>Theme / Skin</label>
        <select id="set-theme">
          ${Object.entries(THEMES).map(([k, v]) => `<option value="${k}" ${(localStorage.getItem('mp_theme') || 'default') === k ? 'selected' : ''}>${v.name}</option>`).join('')}
        </select>
      </div>
      <div class="setting-row">
        <label>Dark Mode</label>
        <div class="toggle ${config.appearance?.dark_mode !== false ? 'on' : ''}" id="set-dark" data-key="appearance.dark_mode"></div>
      </div>
      <div class="setting-row">
        <label>Glass Blur</label>
        <div class="toggle ${localStorage.getItem('mp_glass') === 'true' ? 'on' : ''}" id="set-glass"></div>
      </div>
      <div class="setting-row" id="glass-intensity-row" style="${localStorage.getItem('mp_glass') !== 'true' ? 'display:none' : ''}">
        <label>Blur Intensity</label>
        <input type="range" id="set-blur" min="3" max="30" step="1" value="${localStorage.getItem('mp_blur') || '15'}">
      </div>
      <div class="setting-row">
        <label>Panel Opacity</label>
        <input type="range" id="set-opacity" min="0.3" max="1" step="0.05" value="${localStorage.getItem('mp_opacity') || '1'}">
      </div>
      <div class="setting-row">
        <label>Spectrum Columns</label>
        <select id="set-spectrum-col">
          <option value="16" ${config.appearance?.spectrum_columns == 16 ? 'selected' : ''}>16</option>
          <option value="32" ${config.appearance?.spectrum_columns == 32 ? 'selected' : ''}>32</option>
          <option value="64" ${config.appearance?.spectrum_columns == 64 ? 'selected' : ''}>64</option>
          <option value="128" ${config.appearance?.spectrum_columns == 128 ? 'selected' : ''} ${!config.appearance?.spectrum_columns || config.appearance?.spectrum_columns == 0 ? 'selected' : ''}>128</option>
        </select>
      </div>
      <div class="setting-row">
        <label>Spectrum Style</label>
        <select id="set-spectrum-style">
          <option value="log" ${config.appearance?.spectrum_style !== 'linear' ? 'selected' : ''}>Logarithmic</option>
          <option value="linear" ${config.appearance?.spectrum_style === 'linear' ? 'selected' : ''}>Linear</option>
        </select>
      </div>
      <div class="setting-row">
        <label>Visual Style</label>
        <select id="set-spectrum-visual">
          <option value="modern" ${(localStorage.getItem('mp_spectrum_visual_style')||'modern')==='modern'?'selected':''}>Modern</option>
          <option value="classic" ${localStorage.getItem('mp_spectrum_visual_style')==='classic'?'selected':''}>Classic</option>
        </select>
      </div>
      <div class="setting-row">
        <label>Reflection</label>
        <div class="toggle ${localStorage.getItem('mp_spectrum_reflection')!=='false'?'on':''}" id="set-spectrum-reflection"></div>
      </div>
      <div class="setting-row">
        <label>Fixed Width Bars</label>
        <div class="toggle ${localStorage.getItem('mp_spectrum_fixed_width')==='true'?'on':''}" id="set-spectrum-fixed"></div>
      </div>
      <div class="setting-row">
        <label>Spectrum Height</label>
        <input type="range" id="set-spectrum-height" min="40" max="200" step="5" value="${localStorage.getItem('mp_spectrum_height')||'80'}">
      </div>
      <div class="setting-row">
        <label>FFT Size</label>
        <select id="set-fft-size">
          <option value="256" ${config.appearance?.fft_size == 256 ? 'selected' : ''}>256</option>
          <option value="512" ${config.appearance?.fft_size == 512 ? 'selected' : ''}>512</option>
          <option value="1024" ${config.appearance?.fft_size == 1024 ? 'selected' : ''} ${!config.appearance?.fft_size || config.appearance?.fft_size == 0 ? 'selected' : ''}>1024</option>
          <option value="2048" ${config.appearance?.fft_size == 2048 ? 'selected' : ''}>2048</option>
        </select>
      </div>
    </div>
    <div class="settings-group">
      <h3>Lyrics</h3>
      <div class="setting-row">
        <label>Show Translation</label>
        <div class="toggle ${config.lyric?.show_translate ? 'on' : ''}" id="set-translate" data-key="lyric.show_translate"></div>
      </div>
      <div class="setting-row">
        <label>Fuzzy Match</label>
        <div class="toggle ${config.lyric?.fuzzy_match ? 'on' : ''}" id="set-fuzzy" data-key="lyric.fuzzy_match"></div>
      </div>
      <h3 style="margin-top:12px">Desktop Lyrics</h3>
      <div class="setting-row">
        <label>Color Scheme</label>
        <select id="set-lyric-color">
          ${Object.entries(LYRIC_COLORS).map(([k, v]) => `<option value="${k}" ${(localStorage.getItem('mp_lyric_color')||'default')===k?'selected':''}>${v.name}</option>`).join('')}
        </select>
      </div>
      <div class="setting-row">
        <label>Tri-Color Gradient</label>
        <select id="set-lyric-tricolor">
          <option value="">Off</option>
          <option value="sunset" ${localStorage.getItem('mp_lyric_tricolor')==='sunset'?'selected':''}>Sunset</option>
          <option value="ocean" ${localStorage.getItem('mp_lyric_tricolor')==='ocean'?'selected':''}>Ocean</option>
          <option value="forest" ${localStorage.getItem('mp_lyric_tricolor')==='forest'?'selected':''}>Forest</option>
          <option value="fire" ${localStorage.getItem('mp_lyric_tricolor')==='fire'?'selected':''}>Fire</option>
          <option value="neon" ${localStorage.getItem('mp_lyric_tricolor')==='neon'?'selected':''}>Neon</option>
        </select>
      </div>
      <div class="setting-row">
        <label>Panel Opacity</label>
        <input type="range" id="set-lyric-opacity" min="0.2" max="1" step="0.05" value="${localStorage.getItem('mp_lyric_opacity')||'0.55'}">
      </div>
      <div class="setting-row">
        <label>Text Align</label>
        <select id="set-lyric-align">
          <option value="center" ${(localStorage.getItem('mp_lyric_align')||'center')==='center'?'selected':''}>居中</option>
          <option value="left" ${localStorage.getItem('mp_lyric_align')==='left'?'selected':''}>左对齐</option>
          <option value="right" ${localStorage.getItem('mp_lyric_align')==='right'?'selected':''}>右对齐</option>
        </select>
      </div>
      <div class="setting-row">
        <label>Font Size</label>
        <input type="range" id="set-lyric-size" min="10" max="24" step="1" value="${localStorage.getItem('mp_lyric_size')||'14'}">
      </div>
      <div class="setting-row">
        <label>Line Height</label>
        <input type="range" id="set-lyric-height" min="1" max="3" step="0.1" value="${localStorage.getItem('mp_lyric_height')||'1.5'}">
      </div>
    </div>
    <div class="settings-group">
      <h3>EQ &amp; Effects</h3>
      <div class="setting-row">
        <label>Equalizer</label>
        <div class="toggle ${eqState.enabled ? 'on' : ''}" id="set-eq-enable"></div>
      </div>
      <div class="setting-row">
        <label>Preset</label>
        <select id="set-eq-preset">
          <option value="none">None</option>
          <option value="classical">Classical</option>
          <option value="pop">Pop</option>
          <option value="jazz">Jazz</option>
          <option value="rock">Rock</option>
          <option value="soft">Soft</option>
          <option value="bass">Bass</option>
          <option value="nobass">No Bass</option>
          <option value="nohigh">No High</option>
        </select>
        <button id="btn-eq-save-preset" title="Save current settings as preset" style="margin-left:4px;font-size:10px;padding:2px 6px">Save</button>
        <button id="btn-eq-del-preset" title="Delete selected user preset" style="margin-left:2px;font-size:10px;padding:2px 6px">Del</button>
        <button id="btn-eq-import-preset" title="Import presets from file" style="margin-left:2px;font-size:10px;padding:2px 6px">Import</button>
        <button id="btn-eq-export-preset" title="Export presets to file" style="margin-left:2px;font-size:10px;padding:2px 6px">Export</button>
        <input type="file" id="eq-import-input" accept=".json" style="display:none" />
      </div>
      <div id="eq-bands" style="display:${eqState.enabled?'grid':'none'};grid-template-columns:1fr 1fr;gap:4px;padding:4px 0;">
        ${eqState.bands.length ? eqState.bands.map((g, i) => {
          const freqs = ['31','62','125','250','500','1k','2k','4k','8k','16k'];
          return `<div class="eq-band-row"><label>${freqs[i]}Hz</label><input type="range" min="-15" max="15" value="${g}" step="1" data-band="${i}"><span class="eq-val">${g > 0 ? '+' : ''}${g}dB</span></div>`;
        }).join('') : '<span style="color:var(--text3);font-size:11px">Loading...</span>'}
      </div>
      <div class="setting-row" style="margin-top:8px">
        <label>Reverb</label>
        <div class="toggle ${reverbState.enabled ? 'on' : ''}" id="set-reverb-enable"></div>
      </div>
      <div id="reverb-controls" style="display:${reverbState.enabled?'block':'none'}">
        <div class="setting-row">
          <label>Mix</label>
          <input type="range" id="set-reverb-mix" min="0" max="100" step="1" value="${reverbState.mix}">
        </div>
        <div class="setting-row">
          <label>Time (ms)</label>
          <input type="range" id="set-reverb-time" min="10" max="3000" step="10" value="${reverbState.time}">
        </div>
      </div>
      <div class="setting-row">
        <label>Speed</label>
        <input type="range" id="set-speed" min="0.5" max="2" step="0.05" value="${currSpeed || 1}">
        <span class="eq-val" id="speed-val">${currSpeed || 1}x</span>
      </div>
      <div class="setting-row">
        <label>Pitch (semitones)</label>
        <input type="range" id="set-pitch" min="-12" max="12" step="1" value="${currPitch || 0}">
        <span class="eq-val" id="pitch-val">${currPitch > 0 ? '+' : ''}${currPitch || 0}</span>
      </div>
    </div>
    <div class="settings-group">
      <h3>Media Library</h3>
      <div class="setting-row">
        <label>Auto Scan</label>
        <div class="toggle ${config.media_lib?.auto_scan ? 'on' : ''}" id="set-auto-scan" data-key="media_lib.auto_scan"></div>
      </div>
      <div class="setting-row">
        <label>Min Duration (sec)</label>
        <input type="number" id="set-min-dur" value="${config.media_lib?.min_duration_secs ?? 0}" min="0" />
      </div>
      <div class="setting-row">
        <label>Scan Directories</label>
        <div style="flex:1;display:flex;flex-direction:column;gap:4px">
          <div id="media-dirs-list" style="font-size:11px;color:var(--text2)"></div>
          <div style="display:flex;gap:4px">
            <input type="text" id="set-media-dir-input" placeholder="Add directory path..." style="flex:1;padding:3px 6px;border:1px solid var(--border);border-radius:4px;background:var(--bg2);color:var(--text);font-size:11px">
            <button id="set-media-dir-add" style="padding:3px 8px;font-size:11px;border:1px solid var(--border);border-radius:4px;background:var(--bg2);color:var(--text);cursor:pointer">Add</button>
          </div>
        </div>
      </div>
      <div class="setting-row">
        <label>Rescan Library</label>
        <button id="set-rescan" style="padding:4px 12px;font-size:11px;border:1px solid var(--border);border-radius:4px;background:var(--bg2);color:var(--text);cursor:pointer">Scan Now</button>
      </div>
    </div>
    <div class="settings-group">
      <h3>General</h3>
      <div class="setting-row">
        <label>Language</label>
        <select id="set-language">
          <option value="zh-CN" ${config.general?.language === 'zh-CN' ? 'selected' : ''}>中文</option>
          <option value="en" ${config.general?.language === 'en' ? 'selected' : ''}>English</option>
          <option value="ja" ${config.general?.language === 'ja' ? 'selected' : ''}>日本語</option>
        </select>
      </div>
      <div class="setting-row">
        <label>Auto-Download Lyrics</label>
        <div class="toggle ${config.general?.auto_download_lyric ? 'on' : ''}" id="set-dl-lyric" data-key="general.auto_download_lyric"></div>
      </div>
      <div class="setting-row">
        <label>Auto-Download Cover</label>
        <div class="toggle ${config.general?.auto_download_album_cover !== false ? 'on' : ''}" id="set-dl-cover" data-key="general.auto_download_album_cover"></div>
      </div>
      <div class="setting-row">
        <label>Check Updates on Start</label>
        <div class="toggle ${config.general?.check_update_when_start !== false ? 'on' : ''}" id="set-check-update" data-key="general.check_update_when_start"></div>
      </div>
      <div class="setting-row">
        <label>Minimize to Tray</label>
        <div class="toggle ${config.general?.minimize_to_notify_icon ? 'on' : ''}" id="set-min-tray" data-key="general.minimize_to_notify_icon"></div>
      </div>
    </div>
    <div class="settings-group">
      <h3>Last.fm Scrobbling</h3>
      <div class="setting-row">
        <label>Enable</label>
        <div class="toggle ${config.lastfm?.enabled ? 'on' : ''}" id="set-lastfm-enable" data-key="lastfm.enabled"></div>
      </div>
      <div class="setting-row">
        <label>Username</label>
        <input type="text" id="set-lastfm-user" value="${config.lastfm?.username || ''}" />
      </div>
      <div class="setting-row">
        <label>Password</label>
        <input type="password" id="set-lastfm-pass" value="${config.lastfm?.password || ''}" />
      </div>
      <div class="setting-row">
        <label>API Key</label>
        <input type="text" id="set-lastfm-key" value="${config.lastfm?.api_key || ''}" />
      </div>
      <div class="setting-row">
        <label>Auto Scrobble</label>
        <div class="toggle ${config.lastfm?.auto_scrobble !== false ? 'on' : ''}" id="set-lastfm-scrobble" data-key="lastfm.auto_scrobble"></div>
      </div>
    </div>
  `;
  container.innerHTML = html;

  // Bind toggle events
  container.querySelectorAll('.toggle').forEach(el => {
    el.addEventListener('click', () => {
      const key = el.dataset.key;
      const newVal = el.classList.contains('on') ? 'false' : 'true';
      el.classList.toggle('on');
      api('POST', '/api/config', { key, value: newVal });
    });
  });

  // Theme selector
  const themeSel = document.getElementById('set-theme');
  if (themeSel) {
    themeSel.addEventListener('change', () => applyTheme(themeSel.value));
  }

  // Desktop lyrics settings
  const lycSel = document.getElementById('set-lyric-color');
  if (lycSel) {
    lycSel.addEventListener('change', () => applyLyricColor(lycSel.value));
  }
  const lycOp = document.getElementById('set-lyric-opacity');
  if (lycOp) {
    lycOp.addEventListener('input', () => {
      localStorage.setItem('mp_lyric_opacity', lycOp.value);
    });
  }
  const lycAlign = document.getElementById('set-lyric-align');
  if (lycAlign) {
    lycAlign.addEventListener('change', () => {
      localStorage.setItem('mp_lyric_align', lycAlign.value);
    });
  }
  const lycSize = document.getElementById('set-lyric-size');
  if (lycSize) {
    lycSize.addEventListener('input', () => {
      localStorage.setItem('mp_lyric_size', lycSize.value);
    });
  }
  const lycHeight = document.getElementById('set-lyric-height');
  if (lycHeight) {
    lycHeight.addEventListener('input', () => {
      localStorage.setItem('mp_lyric_height', lycHeight.value);
    });
  }

  // Tri-color gradient
  const triSel = document.getElementById('set-lyric-tricolor');
  if (triSel) {
    triSel.addEventListener('change', () => {
      localStorage.setItem('mp_lyric_tricolor', triSel.value);
    });
  }

  // Glass mode toggle
  const glassToggle = document.getElementById('set-glass');
  if (glassToggle) {
    glassToggle.addEventListener('click', () => {
      const on = glassToggle.classList.toggle('on');
      localStorage.setItem('mp_glass', on ? 'true' : 'false');
      document.body.classList.toggle('glass', on);
      const row = document.getElementById('glass-intensity-row');
      if (row) row.style.display = on ? '' : 'none';
    });
  }

  // Blur intensity slider
  const blurSlider = document.getElementById('set-blur');
  if (blurSlider) {
    blurSlider.addEventListener('input', () => {
      const val = blurSlider.value;
      localStorage.setItem('mp_blur', val);
      document.documentElement.style.setProperty('--glass-blur', val + 'px');
    });
  }

  // Fade Duration
  const fadeTime = document.getElementById('set-fade-time');
  if (fadeTime) {
    fadeTime.addEventListener('change', () => {
      api('POST', '/api/config', { key: 'play.fade_time', value: fadeTime.value });
    });
  }

  // Media library dirs management
  const renderMediaDirs = () => {
    const list = document.getElementById('media-dirs-list');
    const dirs = settingsCache?.media_lib?.media_dirs || [];
    if (dirs.length === 0) { list.textContent = '(none)'; return; }
    list.innerHTML = dirs.map(d => '<span style="display:inline-block;margin:2px 4px 2px 0;padding:1px 6px;background:var(--bg3);border-radius:3px">' + escHtml(d) + ' <span data-rmdir="' + escHtml(d) + '" style="cursor:pointer;color:var(--accent)">×</span></span>').join('');
    list.querySelectorAll('[data-rmdir]').forEach(el => {
      el.addEventListener('click', async () => {
        const dir = el.dataset.rmdir;
        const dirs2 = (settingsCache?.media_lib?.media_dirs || []).filter(d => d !== dir);
        await api('POST', '/api/config', { key: 'media_lib.media_dirs', value: JSON.stringify(dirs2) });
        if (settingsCache) settingsCache.media_lib.media_dirs = dirs2;
        renderMediaDirs();
      });
    });
  };
  const addDirBtn = document.getElementById('set-media-dir-add');
  if (addDirBtn) {
    addDirBtn.addEventListener('click', async () => {
      const input = document.getElementById('set-media-dir-input');
      const dir = input.value.trim();
      if (!dir) return;
      const dirs = settingsCache?.media_lib?.media_dirs || [];
      if (dirs.includes(dir)) return;
      dirs.push(dir);
      await api('POST', '/api/config', { key: 'media_lib.media_dirs', value: JSON.stringify(dirs) });
      if (settingsCache) settingsCache.media_lib.media_dirs = dirs;
      renderMediaDirs();
      input.value = '';
    });
  }
  renderMediaDirs();

  // Last.fm text fields
  ['set-lastfm-user', 'set-lastfm-pass', 'set-lastfm-key'].forEach(id => {
    const el = document.getElementById(id);
    if (el) el.addEventListener('change', () => {
      const key = id === 'set-lastfm-user' ? 'lastfm.username' : id === 'set-lastfm-pass' ? 'lastfm.password' : 'lastfm.api_key';
      api('POST', '/api/config', { key, value: el.value });
    });
  });

  // FFT size
  const fftSel = document.getElementById('set-fft-size');
  if (fftSel) fftSel.addEventListener('change', () => api('POST', '/api/config', { key: 'appearance.fft_size', value: fftSel.value }));

  // Spectrum visual style
  const visSel = document.getElementById('set-spectrum-visual');
  if (visSel) visSel.addEventListener('change', () => {
    localStorage.setItem('mp_spectrum_visual_style', visSel.value);
    resizeCanvas();
  });

  // Spectrum reflection toggle
  const reflToggle = document.getElementById('set-spectrum-reflection');
  if (reflToggle) reflToggle.addEventListener('click', () => {
    const on = reflToggle.classList.toggle('on');
    localStorage.setItem('mp_spectrum_reflection', on ? 'true' : 'false');
  });

  // Spectrum fixed width toggle
  const fixedToggle = document.getElementById('set-spectrum-fixed');
  if (fixedToggle) fixedToggle.addEventListener('click', () => {
    const on = fixedToggle.classList.toggle('on');
    localStorage.setItem('mp_spectrum_fixed_width', on ? 'true' : 'false');
  });

  // Spectrum height slider
  const heightSlider = document.getElementById('set-spectrum-height');
  if (heightSlider) heightSlider.addEventListener('change', () => {
    localStorage.setItem('mp_spectrum_height', heightSlider.value);
    resizeCanvas();
  });

  // Load audio output devices
  (async () => {
    const devSel = document.getElementById('set-output-device');
    if (!devSel) return;
    try {
      const devices = await api('GET', '/api/audio/devices');
      if (devices) {
      const currDev = String(config.play?.output_device ?? '-1');
      devices.forEach(d => {
        const opt = document.createElement('option');
        opt.value = String(d.id);
        opt.textContent = d.name;
        if (String(d.id) === currDev) opt.selected = true;
        devSel.appendChild(opt);
      });
      }
    } catch {}
    devSel.addEventListener('change', () => {
      api('POST', '/api/config', { key: 'play.output_device', value: devSel.value });
    });
  })();

  // Output mode change
  const modeSel = document.getElementById('set-output-mode');
  if (modeSel) {
    modeSel.addEventListener('change', () => {
      api('POST', '/api/config', { key: 'play.output_mode', value: modeSel.value });
      // Show a note about restart
      setTimeout(() => {
        const note = document.getElementById('mode-restart-note') || (() => {
          const n = document.createElement('div');
          n.id = 'mode-restart-note';
          n.style.cssText = 'position:fixed;bottom:20px;left:50%;transform:translateX(-50%);background:var(--accent);color:#fff;padding:8px 16px;border-radius:6px;font-size:12px;z-index:9999';
          n.textContent = 'Output mode will take effect after restart';
          document.body.appendChild(n);
          setTimeout(() => n.remove(), 4000);
          return n;
        })();
      }, 200);
    });
  }

  // ReplayGain mode change
  const rgSel = document.getElementById('set-replaygain');
  if (rgSel) {
    rgSel.addEventListener('change', () => {
      api('POST', '/api/config', { key: 'play.replaygain', value: rgSel.value });
    });
  }

  // Rescan
  const rescanBtn = document.getElementById('set-rescan');
  if (rescanBtn) {
    rescanBtn.addEventListener('click', () => {
      rescanBtn.textContent = 'Scanning...';
      api('POST', '/api/command', { command: 'media scan' }).then(() => {
        rescanBtn.textContent = 'Scan Now';
      }).catch(() => { rescanBtn.textContent = 'Scan Now'; });
    });
  }

  // EQ enable toggle
  const eqEnable = document.getElementById('set-eq-enable');
  if (eqEnable) {
    eqEnable.addEventListener('click', () => {
      const on = eqEnable.classList.toggle('on');
      api('POST', '/api/command', { command: on ? 'eq enable' : 'eq disable' });
      const bands = document.getElementById('eq-bands');
      if (bands) bands.style.display = on ? 'grid' : 'none';
    });
  }
  // EQ preset
  const eqPreset = document.getElementById('set-eq-preset');
  if (eqPreset) {
    eqPreset.addEventListener('change', () => {
      const val = eqPreset.value;
      // Check if it's a user preset
      const userPresets = loadUserEqPresets();
      if (userPresets[val]) {
        // Apply user preset band-by-band
        const bands = userPresets[val];
        bands.forEach((g, i) => {
          api('POST', '/api/command', { command: `eq set ${i} ${g}` });
        });
        setTimeout(fetchEqState, 300);
      } else {
        api('POST', '/api/command', { command: `eq preset ${val}` });
        setTimeout(fetchEqState, 200);
      }
    });
  }
  // Save user EQ preset
  const btnSave = document.getElementById('btn-eq-save-preset');
  if (btnSave) {
    btnSave.addEventListener('click', () => {
      const name = prompt('Save EQ as preset name:');
      if (!name) return;
      const bands = window._eqState?.bands;
      if (!bands) return;
      const presets = loadUserEqPresets();
      presets[name] = bands;
      saveUserEqPresets(presets);
      fetchEqState();
    });
  }
  // Delete user EQ preset
  const btnDel = document.getElementById('btn-eq-del-preset');
  if (btnDel) {
    btnDel.addEventListener('click', () => {
      const sel = document.getElementById('set-eq-preset');
      const val = sel?.value;
      if (!val) return;
      const presets = loadUserEqPresets();
      if (!(val in presets)) { alert('Not a user preset'); return; }
      if (!confirm(`Delete preset "${val}"?`)) return;
      delete presets[val];
      saveUserEqPresets(presets);
      sel.value = 'none';
      fetchEqState();
    });
  }
  // Import EQ presets from file
  const btnImp = document.getElementById('btn-eq-import-preset');
  const importInput = document.getElementById('eq-import-input');
  if (btnImp) btnImp.addEventListener('click', () => importInput?.click());
  if (importInput) {
    importInput.addEventListener('change', () => {
      const file = importInput.files?.[0];
      if (!file) return;
      const reader = new FileReader();
      reader.onload = (e) => {
        try {
          const imported = JSON.parse(e.target.result);
          const presets = loadUserEqPresets();
          let count = 0;
          for (const [name, bands] of Object.entries(imported)) {
            if (Array.isArray(bands) && bands.length === 10) {
              presets[name] = bands;
              count++;
            }
          }
          saveUserEqPresets(presets);
          fetchEqState();
          showToast('EQ 预设导入', `成功导入 ${count} 个预设`, 'info');
        } catch { showToast('导入失败', '无效的 JSON 文件', 'info', 3000); }
      };
      reader.readAsText(file);
      importInput.value = '';
    });
  }
  // Export EQ presets to file
  const btnExp = document.getElementById('btn-eq-export-preset');
  if (btnExp) {
    btnExp.addEventListener('click', () => {
      const presets = loadUserEqPresets();
      const keys = Object.keys(presets);
      if (!keys.length) { showToast('导出失败', '没有用户预设可导出', 'info', 3000); return; }
      const blob = new Blob([JSON.stringify(presets, null, 2)], { type: 'application/json' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url; a.download = 'eq_presets.json'; a.click();
      URL.revokeObjectURL(url);
      showToast('EQ 预设导出', `已导出 ${keys.length} 个预设`, 'info');
    });
  }
  // EQ band sliders (delegated in fetchEqState)
  // Reverb enable toggle
  const revEnable = document.getElementById('set-reverb-enable');
  if (revEnable) {
    revEnable.addEventListener('click', () => {
      const on = revEnable.classList.toggle('on');
      api('POST', '/api/command', { command: on ? 'reverb enable' : 'reverb disable' });
      const ctrl = document.getElementById('reverb-controls');
      if (ctrl) ctrl.style.display = on ? 'block' : 'none';
      if (!on) setTimeout(fetchReverbState, 200);
    });
  }
  // Reverb controls
  const revMix = document.getElementById('set-reverb-mix');
  if (revMix) {
    revMix.addEventListener('change', () => {
      api('POST', '/api/command', { command: `reverb mix ${revMix.value}` });
    });
  }
  const revTime = document.getElementById('set-reverb-time');
  if (revTime) {
    revTime.addEventListener('change', () => {
      api('POST', '/api/command', { command: `reverb time ${revTime.value}` });
    });
  }
  // Speed slider
  const speedSlider = document.getElementById('set-speed');
  if (speedSlider) {
    speedSlider.addEventListener('input', () => {
      const val = parseFloat(speedSlider.value).toFixed(2);
      document.getElementById('speed-val').textContent = val + 'x';
    });
    speedSlider.addEventListener('change', () => {
      const val = parseFloat(speedSlider.value).toFixed(2);
      api('POST', '/api/command', { command: `speed set ${val}` });
    });
  }
  // Pitch slider
  const pitchSlider = document.getElementById('set-pitch');
  if (pitchSlider) {
    pitchSlider.addEventListener('input', () => {
      const val = parseInt(pitchSlider.value);
      document.getElementById('pitch-val').textContent = (val > 0 ? '+' : '') + val;
    });
    pitchSlider.addEventListener('change', () => {
      const val = parseInt(pitchSlider.value);
      api('POST', '/api/command', { command: `pitch set ${val}` });
    });
  }

  // Panel opacity slider
  const opacitySlider = document.getElementById('set-opacity');
  if (opacitySlider) {
    opacitySlider.addEventListener('input', () => {
      const val = opacitySlider.value;
      localStorage.setItem('mp_opacity', val);
      document.documentElement.style.setProperty('--panel-opacity', val);
    });
  }

  // Bind select/input events
  container.querySelectorAll('select').forEach(el => {
    el.addEventListener('change', () => {
      const keyMap = {
        'set-engine': 'play.engine',
        'set-spectrum-col': 'appearance.spectrum_columns',
        'set-spectrum-style': 'appearance.spectrum_style',
        'set-language': 'general.language',
      };
      const key = keyMap[el.id];
      if (key) api('POST', '/api/config', { key, value: el.value });
    });
  });

  container.querySelectorAll('input:not([type="range"])').forEach(el => {
    el.addEventListener('change', () => {
      const keyMap = {
        'set-volume': 'play.default_volume',
        'set-min-dur': 'media_lib.min_duration_secs',
        'set-fade-time': 'play.fade_time',
        'set-lastfm-user': 'lastfm.username',
        'set-lastfm-pass': 'lastfm.password',
        'set-lastfm-key': 'lastfm.api_key',
      };
      const key = keyMap[el.id];
      if (key) api('POST', '/api/config', { key, value: el.value });
    });
  });
}

// ===== Polling loop =====
let pollCount = 0;
let pollTimer = null;

async function poll() {
  if (!state.connected) return;
  await fetchStatus();
  pollCount++;
  if (pollCount % 3 === 0) {
    fetchLyrics();
    fetchCover();
  }
  if (pollCount % 6 === 0) {
    fetchPlaylist();
  }
  pollTimer = setTimeout(poll, 500);
}

// ===== Layout modes =====
const LAYOUTS = ['big', 'narrow', 'small'];
let currentLayout = 'big';

function setLayout(mode) {
  if (!LAYOUTS.includes(mode)) return;
  currentLayout = mode;
  document.body.classList.remove('layout-big', 'layout-narrow', 'layout-small');
  document.body.classList.add(`layout-${mode}`);
  document.body.classList.add('layout-locked');
  localStorage.setItem('mp_layout', mode);

  // Update sidebar drawer visibility for narrow mode
  if (mode === 'narrow' || mode === 'small') {
    document.getElementById('sidebar')?.classList.remove('drawer-open');
  }

  // Update menu checks
  document.querySelectorAll('.menu-item[data-id^="layout_"]').forEach(el => {
    el.classList.toggle('checked', el.dataset.id === `layout_${mode}`);
  });
}

function cycleLayout() {
  const idx = LAYOUTS.indexOf(currentLayout);
  setLayout(LAYOUTS[(idx + 1) % LAYOUTS.length]);
}

function toggleSidebarDrawer() {
  if (currentLayout === 'big') return;
  document.getElementById('sidebar')?.classList.toggle('drawer-open');
}

function loadLayout() {
  const saved = localStorage.getItem('mp_layout') || 'big';
  setLayout(saved);
  // Allow auto-responsive if no explicit lock
  if (!localStorage.getItem('mp_layout')) {
    document.body.classList.remove('layout-locked');
  }
}

// Responsive layout observer
let layoutObserver = null;
function initLayoutObserver() {
  layoutObserver = new ResizeObserver(entries => {
    for (const entry of entries) {
      const w = entry.contentRect.width;
      if (document.body.classList.contains('layout-locked')) return;
      // Auto-switch based on available width
      if (w < 480) {
        document.body.classList.add('layout-small-narrow');
      } else {
        document.body.classList.remove('layout-small-narrow');
      }
    }
  });
  const main = document.getElementById('main');
  if (main) layoutObserver.observe(main);
}

// ===== Context menu support =====
document.addEventListener('contextmenu', (e) => {
  // Playlist item context menu
  const plItem = e.target.closest('.pl-item');
  if (plItem) {
    e.preventDefault();
    contextMenuIndex = parseInt(plItem.dataset.index);
    window.ContextMenu.showContextMenu(CONTEXT_MENUS.playlist, e.clientX, e.clientY);
    return;
  }
  // Main area context menu
  const main = document.getElementById('main');
  if (main.contains(e.target)) {
    e.preventDefault();
    window.ContextMenu.showContextMenu(CONTEXT_MENUS.main, e.clientX, e.clientY);
    return;
  }
});

// ===== Title Bar =====
function renderTitleBar() {
  const bar = document.getElementById('title-bar');
  if (!bar) return;
  if (OHOS) {
    bar.style.display = 'none';
    return;
  }
  bar.innerHTML = `
    <div id="tb-left">
      <button id="tb-menu-btn" class="tb-btn" title="菜单">${ICONS.menu}</button>
    </div>
    <div id="tb-title" data-tauri-drag-region>1028 Music Player</div>
    <div id="tb-right">
      <button id="tb-minimize" class="tb-btn" title="最小化">${ICONS.minimize}</button>
      <button id="tb-maximize" class="tb-btn" title="最大化">${ICONS.maximize}</button>
      <button id="tb-close" class="tb-btn tb-close" title="关闭">${ICONS.close}</button>
    </div>`;
}

// ===== Controls =====
function renderControls() {
  const prim = document.getElementById('controls-primary');
  if (prim) {
    prim.innerHTML = `
      <button id="btn-prev" class="ctrl-btn" title="上一曲 (Ctrl+Left)">${ICONS.skip_previous}</button>
      <button id="btn-play" class="ctrl-btn ctrl-primary" title="播放/暂停 (Space)">
        <span id="play-icon">${ICONS.play_arrow}</span>
        <span id="pause-icon" style="display:none">${ICONS.pause}</span>
      </button>
      <button id="btn-stop" class="ctrl-btn" title="停止">${ICONS.stop}</button>
      <button id="btn-next" class="ctrl-btn" title="下一曲 (Ctrl+Right)">${ICONS.skip_next}</button>`;
  }

  const sec = document.getElementById('controls-secondary');
  if (sec) {
    sec.innerHTML = `
      <button id="btn-repeat" class="ctrl-btn fn-btn" title="循环模式">${ICONS.repeat}</button>
      <button id="btn-shuffle" class="ctrl-btn fn-btn" title="随机播放">${ICONS.shuffle}</button>
      <span class="fn-sep"></span>
      <button id="btn-favourite" class="ctrl-btn fn-btn" title="收藏">${ICONS.favorite_border}</button>
      <button id="btn-lyrics" class="ctrl-btn fn-btn" title="歌词">${ICONS.lyrics}</button>
      <button id="btn-equalizer" class="ctrl-btn fn-btn" title="均衡器">${ICONS.equalizer}</button>
      <button id="btn-ab-repeat" class="ctrl-btn fn-btn" title="AB 复读">${ICONS.ab_repeat}</button>
      <span class="fn-sep"></span>
      <button id="btn-mini-mode" class="ctrl-btn fn-btn" title="迷你模式 (Ctrl+Alt+M)">${ICONS.mini_mode}</button>
      <button id="btn-fullscreen" class="ctrl-btn fn-btn" title="全屏 (F11)">${ICONS.fullscreen}</button>
      <button id="btn-dark-mode" class="ctrl-btn fn-btn" title="深色模式">${ICONS.dark_mode}</button>
      <button id="btn-settings" class="ctrl-btn fn-btn" title="设置">${ICONS.settings}</button>`;
  }

  // Sidebar toggle icon
  const st = document.getElementById('sidebar-toggle');
  if (st) st.innerHTML = ICONS.menu;

  const volIcon = document.getElementById('vol-icon-wrap');
  if (volIcon) volIcon.innerHTML = ICONS.volume_up;
}

// ----- Tag Editor Dialog -----
function showTagEditor(index) {
  const track = index != null ? state.playlist[index] : gTrack;
  if (!track) {
    showDialog({ title: 'Tag Editor', body: '<span style="color:var(--text3)">No track selected</span>' });
    return;
  }
  const filePath = track.file_path;
  showDialog({
    title: 'Tag Editor',
    width: '480px',
    onOpen: async (box) => {
      const statusEl = box.querySelector('#tag-status');
      const saveBtn = box.querySelector('#tag-save');
      const form = box.querySelector('#tag-form');
      // Load current tags
      try {
        const r = await api('POST', '/api/tag/read', { file: filePath });
        if (r) {
          form.querySelector('[name="title"]').value = r.title || '';
          form.querySelector('[name="artist"]').value = r.artist || '';
          form.querySelector('[name="album"]').value = r.album || '';
          form.querySelector('[name="genre"]').value = r.genre || '';
          form.querySelector('[name="year"]').value = r.year || '';
          form.querySelector('[name="track"]').value = r.track || '';
          statusEl.textContent = 'Tags loaded';
        } else {
          statusEl.textContent = 'Cannot read tags';
        }
      } catch { statusEl.textContent = 'Error loading tags'; }

      saveBtn.addEventListener('click', async () => {
        const fields = ['title', 'artist', 'album', 'genre', 'year', 'track'];
        let success = 0, failed = 0;
        for (const field of fields) {
          const val = form.querySelector(`[name="${field}"]`).value.trim();
          if (!val) continue;
          const cmdStr = `tag set "${filePath}" ${field} "${val}"`;
          try {
            const res = await api('POST', '/api/command', { command: cmdStr });
            if (res?.success) success++; else failed++;
          } catch { failed++; }
        }
        statusEl.textContent = `Saved: ${success} fields, failed: ${failed}`;
        statusEl.style.color = failed ? '#e74c3c' : 'var(--accent)';
      });
    },
    body: `
      <div style="color:var(--text3);font-size:11px;margin-bottom:8px;word-break:break-all">${escHtml(filePath)}</div>
      <div id="tag-form" class="prop-grid">
        <span class="prop-label">Title</span><input name="title" class="tag-input">
        <span class="prop-label">Artist</span><input name="artist" class="tag-input">
        <span class="prop-label">Album</span><input name="album" class="tag-input">
        <span class="prop-label">Genre</span><input name="genre" class="tag-input">
        <span class="prop-label">Year</span><input name="year" type="number" class="tag-input" style="width:80px">
        <span class="prop-label">Track</span><input name="track" type="number" class="tag-input" style="width:80px">
      </div>
      <div id="tag-status" style="margin-top:8px;font-size:11px;color:var(--text3)"></div>`,
    footer: `<button id="tag-save" class="primary">Save</button><button data-dlg-close>Close</button>`
  });
}

// ----- Hotkey Manager -----
const DEFAULT_HOTKEYS = {
  'Space': 'pause', 'Ctrl+Left': 'prev', 'Ctrl+Right': 'next',
  'Up': 'volume_up', 'Down': 'volume_down',
  'F11': 'fullscreen', 'Ctrl+O': 'open', 'Ctrl+F': 'open_folder',
  'Ctrl+M': 'media_lib', 'Ctrl+Shift+L': 'cycle_layout',
  'Ctrl+Alt+M': 'mini_mode', 'Ctrl+S': 'playlist_save',
  '?': 'shortcuts',
};

function loadHotkeys() {
  try { return JSON.parse(localStorage.getItem('mp_hotkeys')) || {}; } catch { return {}; }
}
function saveHotkeys(map) { localStorage.setItem('mp_hotkeys', JSON.stringify(map)); }

function showHotkeyDialog() {
  const userHotkeys = loadHotkeys();
  const allHotkeys = { ...DEFAULT_HOTKEYS, ...userHotkeys };
  // Build reverse lookup
  const actionNames = {
    pause: 'Play/Pause', prev: 'Previous Track', next: 'Next Track',
    stop: 'Stop', volume_up: 'Volume +5', volume_down: 'Volume -5',
    fullscreen: 'Toggle Fullscreen', open: 'Open File', open_folder: 'Open Folder',
    media_lib: 'Media Library', cycle_layout: 'Cycle Layout',
    mini_mode: 'Mini Mode', playlist_save: 'Save Playlist',
    shortcuts: 'Shortcut Help',
  };

  showDialog({
    title: 'Hotkey Settings',
    width: '480px',
    onOpen: (box) => {
      const listEl = box.querySelector('#hk-list');
      const captureEl = box.querySelector('#hk-capture');
      const resetBtn = box.querySelector('#hk-reset');
      let rebinding = null;

      const render = () => {
        const current = { ...DEFAULT_HOTKEYS, ...loadHotkeys() };
        listEl.innerHTML = Object.entries(actionNames).map(([action, label]) => {
          const key = Object.entries(current).find(([, a]) => a === action)?.[0] || '(unset)';
          return `<div class="hk-item" data-action="${action}">
            <span class="hk-label">${label}</span>
            <span class="hk-key">${escHtml(key)}</span>
          </div>`;
        }).join('');
        listEl.querySelectorAll('.hk-item').forEach(el => {
          el.addEventListener('click', () => {
            const action = el.dataset.action;
            rebinding = action;
            el.classList.add('hk-recording');
            captureEl.style.display = 'block';
            captureEl.textContent = `Press a key for: ${actionNames[action] || action}`;
          });
        });
      };

      const handleKey = (e) => {
        if (!rebinding) return;
        e.preventDefault();
        const parts = [];
        if (e.ctrlKey) parts.push('Ctrl');
        if (e.altKey) parts.push('Alt');
        if (e.shiftKey) parts.push('Shift');
        if (e.metaKey) parts.push('Meta');
        const key = e.key === ' ' ? 'Space' : e.key.length === 1 ? e.key.toUpperCase() : e.key;
        if (key.startsWith('F') && key.length <= 3) parts.push(key);
        else if (!e.ctrlKey && !e.altKey && !e.shiftKey && !e.metaKey) parts.push(key);
        else if (e.ctrlKey || e.altKey || e.shiftKey || e.metaKey) parts.push(key);

        const combo = parts.join('+');
        if (combo === 'Escape') { rebinding = null; captureEl.style.display = 'none'; render(); return; }

        const hotkeys = loadHotkeys();
        hotkeys[combo] = rebinding;
        saveHotkeys(hotkeys);
        rebinding = null;
        captureEl.style.display = 'none';
        render();
      };

      document.addEventListener('keydown', handleKey);
      box.addEventListener('remove', () => document.removeEventListener('keydown', handleKey));

      resetBtn.addEventListener('click', () => {
        if (confirm('Reset all hotkeys to defaults?')) {
          localStorage.removeItem('mp_hotkeys');
          render();
        }
      });

      render();
    },
    body: `
      <div id="hk-capture" style="display:none;padding:8px;background:var(--bg3);border-radius:6px;margin-bottom:8px;text-align:center;font-size:13px;color:var(--accent)"></div>
      <div id="hk-list"></div>`,
    footer: `<button id="hk-reset">Reset to Defaults</button><button data-dlg-close>Close</button>`
  });
}
// ===== Notification/Toast =====
function showToast(msg, type) {
  let container = document.getElementById('toast-container');
  if (!container) {
    container = document.createElement('div');
    container.id = 'toast-container';
    container.style.cssText = 'position:fixed;bottom:20px;left:50%;transform:translateX(-50%);z-index:9999;display:flex;flex-direction:column;gap:6px;align-items:center;pointer-events:none';
    document.body.appendChild(container);
  }
  const el = document.createElement('div');
  el.textContent = msg;
  el.style.cssText = 'padding:8px 18px;border-radius:8px;font-size:12px;background:var(--bg2);color:var(--text);border:1px solid var(--border);box-shadow:0 4px 16px rgba(0,0,0,0.3);animation:fadeIn 0.15s ease;max-width:400px;text-align:center';
  if (type === 'error') el.style.borderColor = '#e74c3c';
  if (type === 'success') el.style.borderColor = '#2ecc71';
  container.appendChild(el);
  setTimeout(() => { el.style.opacity = '0'; el.style.transition = 'opacity 0.3s'; setTimeout(() => el.remove(), 300); }, 2500);
}
// Wrap API with error toast
const _origApi = window._origApi || api;
window._origApi = _origApi;
api = async function(method, path, body) {
  try {
    const result = await _origApi(method, path, body);
    if (result === null && method !== 'GET') showToast('Request failed', 'error');
    return result;
  } catch {
    if (method !== 'GET') showToast('Connection error', 'error');
    return null;
  }
};

function showDialog(opts) {
  const overlay = document.createElement('div');
  overlay.className = 'dialog-overlay';
  overlay.innerHTML = `
    <div class="dialog-box" style="${opts.width ? 'width:'+opts.width : ''}">
      <div class="dialog-header">
        <h2>${escHtml(opts.title || 'Dialog')}</h2>
        <button class="dlg-close" data-dlg-close>${ICONS.close}</button>
      </div>
      <div class="dialog-body">${opts.body || ''}</div>
      ${opts.footer !== false ? `<div class="dialog-footer">${opts.footer || '<button data-dlg-close>Close</button>'}</div>` : ''}
    </div>`;
  document.body.appendChild(overlay);
  const box = overlay.querySelector('.dialog-box');
  overlay.querySelectorAll('[data-dlg-close]').forEach(el => {
    el.addEventListener('click', () => overlay.remove());
  });
  overlay.addEventListener('click', (e) => {
    if (e.target === overlay) overlay.remove();
  });
  if (opts.onOpen) opts.onOpen(box, overlay);
  return overlay;
}

function hideDialog() {
  const el = document.querySelector('.dialog-overlay');
  if (el) el.remove();
}

// ----- About Dialog -----
function showAboutDialog() {
  showDialog({
    title: 'About 1028 Music Player',
    width: '420px',
    footer: '<button class="primary" data-dlg-close>OK</button>',
    body: `
      <div style="text-align:center;padding:16px 0">
        <div style="font-size:32px;margin-bottom:8px">🎵</div>
        <div style="font-size:18px;font-weight:600">1028 Music Player</div>
        <div style="color:var(--text3);font-size:12px">v1.0.0</div>
        <div style="margin:12px 0;font-size:12px;color:var(--text2);line-height:1.8">
          A full-featured music player for Windows / macOS / Linux<br>
          Based on Rust + Tauri + BASS/FFmpeg engines<br><br>
          Original project: MusicPlayer2 (MFC/C++)<br>
          Open source under GPL-3.0
        </div>
      </div>`
  });
}

// ----- Shortcuts Dialog -----
function showShortcutsDialog() {
  showDialog({
    title: 'Keyboard Shortcuts',
    width: '420px',
    body: `
      <div class="prop-grid">
        <span class="prop-label">Space</span><span class="prop-value">Play / Pause</span>
        <span class="prop-label">← / →</span><span class="prop-value">Seek -5s / +5s</span>
        <span class="prop-label">↑ / ↓</span><span class="prop-value">Volume +5 / -5</span>
        <span class="prop-label">F11</span><span class="prop-value">Toggle Fullscreen</span>
        <span class="prop-label">Ctrl+O</span><span class="prop-value">Open File</span>
        <span class="prop-label">Ctrl+F</span><span class="prop-value">Open Folder</span>
        <span class="prop-label">Ctrl+M</span><span class="prop-value">Media Library</span>
        <span class="prop-label">Ctrl+Shift+L</span><span class="prop-value">Cycle Layout</span>
        <span class="prop-label">Alt+F4</span><span class="prop-value">Exit</span>
        <span class="prop-label">?</span><span class="prop-value">This Help</span>
      </div>`
  });
}

// ----- File Properties Dialog -----
function showFileProperties() {
  if (!gTrack) {
    showDialog({ title: 'File Properties', body: '<span style="color:var(--text3)">No track loaded</span>' });
    return;
  }
  const t = gTrack;
  const fmtDur = (s) => { const m = Math.floor(s/60); return m+':'+String(s%60).padStart(2,'0'); };
  showDialog({
    title: 'File Properties',
    width: '460px',
    body: `
      <div class="prop-grid">
        <span class="prop-label">Title</span><span class="prop-value">${escHtml(t.title || '(unknown)')}</span>
        <span class="prop-label">Artist</span><span class="prop-value">${escHtml(t.artist || '(unknown)')}</span>
        <span class="prop-label">Album</span><span class="prop-value">${escHtml(t.album || '(unknown)')}</span>
        <span class="prop-label">Duration</span><span class="prop-value">${fmtDur(t.duration_secs)}</span>
        <span class="prop-label">Current Position</span><span class="prop-value">${fmtDur(t.position_secs)}</span>
        <span class="prop-label">File</span><span class="prop-value" style="font-size:11px;word-break:break-all">${escHtml(t.file)}</span>
        <span class="prop-label">Favourite</span><span class="prop-value">${t.is_favourite ? '★ Yes' : '☆ No'}</span>
        <span class="prop-label">Rating</span><span class="prop-value">${'★'.repeat(t.rating || 0) + '☆'.repeat(5 - (t.rating || 0))}</span>
      </div>`
  });
}

// ----- Sleep Timer Dialog -----
let sleepTimerHandle = null;
let sleepEndTime = null;

function showSleepTimerDialog() {
  let remaining = '';
  if (sleepEndTime) {
    const diff = Math.max(0, Math.floor((sleepEndTime - Date.now()) / 1000));
    remaining = ` (${Math.floor(diff/60)}m ${diff%60}s remaining)`;
  }
  showDialog({
    title: 'Sleep Timer' + remaining,
    width: '380px',
    onOpen: (box) => {
      const radioGroup = box.querySelector('[name="sleep-mode"]');
      const minsInput = box.querySelector('#sleep-mins');
      const statusEl = box.querySelector('#sleep-status');
      const startBtn = box.querySelector('#sleep-start');
      const stopBtn = box.querySelector('#sleep-stop');
      const radioVal = () => document.querySelector('input[name="sleep-mode"]:checked')?.value;
      const updateStatus = () => {
        if (sleepEndTime) {
          const diff = Math.max(0, Math.floor((sleepEndTime - Date.now()) / 1000));
          statusEl.textContent = `Timer active: ${Math.floor(diff/60)}m ${diff%60}s remaining`;
          statusEl.style.color = 'var(--accent)';
        } else {
          statusEl.textContent = 'Timer inactive';
          statusEl.style.color = 'var(--text3)';
        }
      };
      startBtn.addEventListener('click', () => {
        const mode = radioVal();
        let secs = 0;
        if (mode === 'off') { clearSleepTimer(); updateStatus(); return; }
        if (mode === 'custom') secs = parseInt(minsInput.value) * 60 || 0;
        else secs = { '15m':900, '30m':1800, '45m':2700, '1h':3600, '2h':7200 }[mode] || 0;
        if (secs <= 0) return;
        clearSleepTimer();
        sleepEndTime = Date.now() + secs * 1000;
        sleepTimerHandle = setTimeout(() => { cmd('stop'); clearSleepTimer(); }, secs * 1000);
        updateStatus();
      });
      stopBtn.addEventListener('click', () => { clearSleepTimer(); updateStatus(); });
      updateStatus();
      if (sleepEndTime) {
        const interval = setInterval(() => {
          if (!sleepEndTime) { clearInterval(interval); return; }
          const diff = Math.max(0, Math.floor((sleepEndTime - Date.now()) / 1000));
          statusEl.textContent = `Timer active: ${Math.floor(diff/60)}m ${diff%60}s remaining`;
          if (diff <= 0) { clearSleepTimer(); clearInterval(interval); }
        }, 1000);
        box.addEventListener('remove', () => clearInterval(interval));
      }
    },
    body: `
      <div class="sleep-options">
        <label><input type="radio" name="sleep-mode" value="off" checked> Disabled</label>
        <label><input type="radio" name="sleep-mode" value="15m"> 15 minutes</label>
        <label><input type="radio" name="sleep-mode" value="30m"> 30 minutes</label>
        <label><input type="radio" name="sleep-mode" value="45m"> 45 minutes</label>
        <label><input type="radio" name="sleep-mode" value="1h"> 1 hour</label>
        <label><input type="radio" name="sleep-mode" value="2h"> 2 hours</label>
        <label><input type="radio" name="sleep-mode" value="custom"> Custom:</label>
        <div class="sleep-custom-row">
          <input type="number" id="sleep-mins" value="10" min="1">
          <span style="color:var(--text3);font-size:12px">minutes</span>
        </div>
      </div>
      <div id="sleep-status" style="margin-top:12px;font-size:12px;color:var(--text3)"></div>`,
    footer: `<button id="sleep-start" class="primary">Start</button><button id="sleep-stop">Cancel</button><button data-dlg-close>Close</button>`
  });
}

function clearSleepTimer() {
  if (sleepTimerHandle) { clearTimeout(sleepTimerHandle); sleepTimerHandle = null; }
  sleepEndTime = null;
}

// ----- Play Stats Dialog -----
function showPlayStats() {
  showDialog({
    title: 'Play Stats',
    width: '520px',
    onOpen: async (box) => {
      const bodyEl = box.querySelector('.dlg-body-custom');
      bodyEl.innerHTML = '<div style="text-align:center;padding:20px;color:var(--text3)">Loading...</div>';
      try {
        const s = await api('GET', '/api/stats');
        if (!s) { bodyEl.innerHTML = '<div style="text-align:center;padding:20px;color:var(--text3)">Failed to load stats</div>'; return; }
        function fmtHms(secs) {
          const h = Math.floor(secs / 3600), m = Math.floor((secs % 3600) / 60), sec = secs % 60;
          return `${h}h ${m}m ${sec}s`;
        }
        let html = `<div class="stats-grid">
          <div class="stat-card"><span class="stat-val">${fmtHms(s.total_listen_secs)}</span><span class="stat-lbl">Total Time</span></div>
          <div class="stat-card"><span class="stat-val">${s.total_play_count}</span><span class="stat-lbl">Plays</span></div>
          <div class="stat-card"><span class="stat-val">${s.total_track_count}</span><span class="stat-lbl">Tracks</span></div>
          <div class="stat-card"><span class="stat-val">${fmtHms(s.day_secs)}</span><span class="stat-lbl">Today</span></div>
          <div class="stat-card"><span class="stat-val">${fmtHms(s.week_secs)}</span><span class="stat-lbl">This Week</span></div>
          <div class="stat-card"><span class="stat-val">${fmtHms(s.month_secs)}</span><span class="stat-lbl">This Month</span></div>
        </div>`;
        if (s.top_tracks?.length) {
          html += '<h4 style="margin:12px 0 6px;font-size:12px;color:var(--text2)">Top Tracks</h4>';
          html += s.top_tracks.map((t, i) =>
            `<div class="stat-row"><span class="stat-rank">${i+1}</span><span class="stat-name">${escHtml(t.title || t.path.split(/[/\\]/).pop())}</span><span class="stat-count">${t.play_count} plays</span></div>`
          ).join('');
        }
        if (s.top_artists?.length) {
          html += '<h4 style="margin:12px 0 6px;font-size:12px;color:var(--text2)">Top Artists</h4>';
          html += s.top_artists.map((a, i) =>
            `<div class="stat-row"><span class="stat-rank">${i+1}</span><span class="stat-name">${escHtml(a.artist || '(unknown)')}</span><span class="stat-count">${a.play_count} plays</span></div>`
          ).join('');
        }
        bodyEl.innerHTML = html;
      } catch { bodyEl.innerHTML = '<div style="text-align:center;padding:20px;color:#e74c3c">Error loading stats</div>'; }
    },
    body: '<div class="dlg-body-custom"></div>',
    footer: '<button data-dlg-close>Close</button>'
  });
}

// ----- MusicBrainz Auto-Tag Dialog -----
function showMusicBrainzDialog() {
  showDialog({
    title: 'MusicBrainz Auto-Tag',
    width: '420px',
    onOpen: async (box) => {
      const statusEl = box.querySelector('#mb-status');
      const applyBtn = box.querySelector('#mb-apply');
      statusEl.textContent = 'Looking up current track on MusicBrainz...';
      try {
        const r = await api('POST', '/api/musicbrainz', { auto: true });
        if (r?.success) {
          statusEl.textContent = r.message || 'Tag updated from MusicBrainz';
          applyBtn.disabled = true;
        } else {
          statusEl.textContent = 'MusicBrainz: ' + (r?.error || 'no match');
        }
      } catch { statusEl.textContent = 'MusicBrainz lookup failed'; }
    },
    body: '<p style="color:var(--text3);font-size:12px">Looks up the current track on MusicBrainz.org and automatically writes title/artist/album/year/track tags.</p><div id="mb-status" style="margin-top:8px;font-size:12px;color:var(--text3)"></div>',
    footer: '<button id="mb-apply" class="primary">Auto-Tag</button><button data-dlg-close>Close</button>'
  });
}

// ----- Lyrics Editor Dialog -----
function showLyricEditor() {
  showDialog({
    title: 'Lyrics Editor',
    width: '520px',
    onOpen: async (box) => {
      const textarea = box.querySelector('.lyric-edit-area');
      const saveBtn = box.querySelector('#lyric-save');
      const status = box.querySelector('#lyric-edit-status');
      // Load current lyrics
      try {
        const r = await fetch('/api/lyric');
        const data = await r.json();
        textarea.value = data.lyric || '(no lyrics)';
      } catch { textarea.value = '(failed to load lyrics)'; }
      saveBtn.addEventListener('click', async () => {
        const lyric = textarea.value;
        // Send via command - backend handles writing
        status.textContent = 'Saving...';
        status.style.color = 'var(--text2)';
        try {
          const r = await cmd('lyric edit');
          status.textContent = 'Saved (lyric edit command sent)';
          status.style.color = 'var(--accent)';
        } catch {
          status.textContent = 'Save failed';
          status.style.color = '#e74c3c';
        }
      });
    },
    body: `
      <textarea class="lyric-edit-area" placeholder="Edit lyrics here..."></textarea>
      <div id="lyric-edit-status" style="margin-top:8px;font-size:11px;color:var(--text3)">Current lyrics loaded</div>`,
    footer: `<button id="lyric-save" class="primary">Save</button><button data-dlg-close>Close</button>`
  });
}

// ----- Playlist Manager Dialog -----
function showPlaylistManager() {
  showDialog({
    title: 'Playlist Manager',
    width: '480px',
    onOpen: async (box) => {
      const listEl = box.querySelector('#plm-list');
      const statusEl = box.querySelector('#plm-status');
      const newBtn = box.querySelector('#plm-new');
      const renameBtn = box.querySelector('#plm-rename');
      const delBtn = box.querySelector('#plm-delete');
      const refreshList = async () => {
        try {
          const r = await fetch('/api/playlist/list');
          const data = await r.json();
          const active = data.find(p => p.is_active);
          listEl.innerHTML = data.map(p =>
            `<div class="plm-item ${p.is_active ? 'active' : ''}" data-name="${escHtml(p.name)}">
              <span class="plm-name">${escHtml(p.name)}</span>
              <span class="plm-count">${p.track_count} tracks</span>
              ${!p.is_active ? '<span class="plm-load" title="Load">' + ICONS.play_arrow + '</span>' : ''}
            </div>`
          ).join('');
          // Load click
          listEl.querySelectorAll('.plm-load').forEach(el => {
            el.addEventListener('click', async (e) => {
              e.stopPropagation();
              const name = el.parentElement.dataset.name;
              await api('POST', '/api/command', { command: `playlist load ${name}` });
              refreshList();
            });
          });
          // Item click to switch
          listEl.querySelectorAll('.plm-item:not(.active)').forEach(el => {
            el.addEventListener('click', async () => {
              if (confirm(`Switch to "${el.dataset.name}"?`)) {
                await api('POST', '/api/command', { command: `playlist load ${el.dataset.name}` });
                refreshList();
              }
            });
          });
          statusEl.textContent = `${data.length} playlists`;
        } catch { statusEl.textContent = 'Failed to load'; }
      };

      newBtn.addEventListener('click', async () => {
        const name = prompt('Playlist name:');
        if (!name) return;
        await api('POST', '/api/command', { command: `playlist new ${name}` });
        refreshList();
      });
      renameBtn.addEventListener('click', async () => {
        const sel = listEl.querySelector('.active') || listEl.querySelector('.selected');
        if (!sel) { statusEl.textContent = 'Select a playlist first'; return; }
        const name = prompt('New name:', sel.dataset.name);
        if (!name) return;
        await api('POST', '/api/command', { command: `playlist rename ${sel.dataset.name} ${name}` });
        refreshList();
      });
      delBtn.addEventListener('click', async () => {
        const sel = listEl.querySelector('.selected');
        if (!sel) { statusEl.textContent = 'Select a playlist first'; return; }
        if (!confirm(`Delete "${sel.dataset.name}"?`)) return;
        await api('POST', '/api/command', { command: `playlist delete ${sel.dataset.name}` });
        refreshList();
      });

      // Export / Import
      box.querySelector('#plm-export')?.addEventListener('click', async () => {
        try {
          const r = await api('GET', '/api/playlist/export');
          if (!r) return;
          const blob = new Blob([r.m3u], { type: 'text/plain' });
          const url = URL.createObjectURL(blob);
          const a = document.createElement('a');
          a.href = url; a.download = 'playlist.m3u'; a.click();
          URL.revokeObjectURL(url);
          statusEl.textContent = `Exported ${r.count} tracks`;
        } catch { statusEl.textContent = 'Export failed'; }
      });
      box.querySelector('#plm-import')?.addEventListener('click', () => {
        const input = document.createElement('input');
        input.type = 'file'; input.accept = '.m3u,.m3u8,.txt,.playlist';
        input.addEventListener('change', async () => {
          if (!input.files?.[0]) return;
          const text = await input.files[0].text();
          const r = await api('POST', '/api/playlist/import', { m3u: text });
          if (r?.success) statusEl.textContent = 'Import successful';
          else statusEl.textContent = 'Import failed';
          refreshList();
        });
        input.click();
      });

      await refreshList();
    },
    body: `
      <div id="plm-list" style="max-height:300px;overflow-y:auto;border:1px solid var(--border);border-radius:4px;margin-bottom:8px"></div>
      <div id="plm-status" style="font-size:11px;color:var(--text3);margin-bottom:4px"></div>`,
    footer: `
      <button id="plm-new" class="primary">New</button>
      <button id="plm-rename">Rename</button>
      <button id="plm-delete">Delete</button>
      <button id="plm-export">Export</button>
      <button id="plm-import">Import</button>
      <button data-dlg-close>Close</button>`
  });
}

// ----- Batch Tag Edit Dialog -----
function showBatchTagEditor() {
  showDialog({
    title: 'Batch Tag Edit',
    width: '500px',
    onOpen: async (box) => {
      const listEl = box.querySelector('#bt-list');
      const statusEl = box.querySelector('#bt-status');
      const saveBtn = box.querySelector('#bt-save');
      let selected = new Set();

      const load = async () => {
        try {
          const r = await fetch('/api/playlist');
          const data = await r.json();
          listEl.innerHTML = data.tracks.map((t, i) =>
            `<div class="bt-item ${selected.has(i) ? 'selected' : ''}" data-index="${i}">
              <span class="bt-check">${selected.has(i) ? '☑' : '☐'}</span>
              <span class="bt-title">${escHtml(t.title || '(unknown)')}</span>
              <span class="bt-artist">${escHtml(t.artist || '')}</span>
            </div>`
          ).join('');
          listEl.querySelectorAll('.bt-item').forEach(el => {
            el.addEventListener('click', () => {
              const idx = parseInt(el.dataset.index);
              if (selected.has(idx)) selected.delete(idx);
              else selected.add(idx);
              el.classList.toggle('selected');
              el.querySelector('.bt-check').textContent = selected.has(idx) ? '☑' : '☐';
            });
          });
          statusEl.textContent = `Total: ${data.tracks.length} tracks, ${selected.size} selected`;
        } catch { statusEl.textContent = 'Failed to load playlist'; }
      };

      saveBtn.addEventListener('click', async () => {
        if (selected.size === 0) { statusEl.textContent = 'Select at least one track'; return; }
        try {
          const r = await fetch('/api/playlist');
          const data = await r.json();
          const indices = [...selected].sort((a, b) => a - b);
          const fields = ['title', 'artist', 'album', 'genre', 'year', 'track'];
          const values = {};
          for (const f of fields) {
            const inp = box.querySelector(`#bt-${f}`);
            const cb = box.querySelector(`#bt-cb-${f}`);
            if (cb.checked && inp.value.trim()) values[f] = inp.value.trim();
          }
          if (Object.keys(values).length === 0) {
            statusEl.textContent = 'No fields selected to edit';
            return;
          }
          let ok = 0, fail = 0;
          for (const idx of indices) {
            const track = data.tracks[idx];
            statusEl.textContent = `Editing ${track.title || track.file_path}...`;
            for (const [field, val] of Object.entries(values)) {
              const cmd = `tag set "${track.file_path}" ${field} "${val}"`;
              try {
                const res = await api('POST', '/api/command', { command: cmd });
                if (res?.success) ok++; else fail++;
              } catch { fail++; }
            }
          }
          statusEl.textContent = `Done: ${ok} fields written, ${fail} failed`;
        } catch { statusEl.textContent = 'Error'; }
      });

      await load();
    },
    body: `
      <div id="bt-fields">
        <div class="bt-field"><label><input type="checkbox" id="bt-cb-title" checked> Title</label><input type="text" id="bt-title" class="tag-input" placeholder="leave empty to skip"></div>
        <div class="bt-field"><label><input type="checkbox" id="bt-cb-artist" checked> Artist</label><input type="text" id="bt-artist" class="tag-input" placeholder="leave empty to skip"></div>
        <div class="bt-field"><label><input type="checkbox" id="bt-cb-album" checked> Album</label><input type="text" id="bt-album" class="tag-input" placeholder="leave empty to skip"></div>
        <div class="bt-field"><label><input type="checkbox" id="bt-cb-genre" checked> Genre</label><input type="text" id="bt-genre" class="tag-input" placeholder="leave empty to skip"></div>
        <div class="bt-field"><label><input type="checkbox" id="bt-cb-year" checked> Year</label><input type="number" id="bt-year" class="tag-input" style="width:100px" placeholder="leave empty to skip"></div>
        <div class="bt-field"><label><input type="checkbox" id="bt-cb-track" checked> Track</label><input type="number" id="bt-track" class="tag-input" style="width:100px" placeholder="leave empty to skip"></div>
      </div>
      <div id="bt-list" style="max-height:200px;overflow-y:auto;margin:8px 0;border:1px solid var(--border);border-radius:4px"></div>
      <div id="bt-status" style="font-size:11px;color:var(--text3)"></div>`,
    footer: `<button id="bt-save" class="primary">Apply to Selected</button><button data-dlg-close>Close</button>`
  });
}

// ----- Format Converter Dialog -----
function showFormatConverter() {
  showDialog({
    title: 'Format Converter',
    width: '520px',
    onOpen: async (box) => {
      const listEl = box.querySelector('#fc-list');
      const fmtSel = box.querySelector('#fc-format');
      const modeSel = box.querySelector('#fc-mode');
      const qualityRow = box.querySelector('#fc-quality-row');
      const bitrateRow = box.querySelector('#fc-bitrate-row');
      const qualityInp = box.querySelector('#fc-quality');
      const bitrateInp = box.querySelector('#fc-bitrate');
      const statusEl = box.querySelector('#fc-status');
      const convertBtn = box.querySelector('#fc-convert');
      let selectedIndices = new Set();

      // Load playlist tracks
      const loadTracks = async () => {
        try {
          const r = await fetch('/api/playlist');
          const data = await r.json();
          listEl.innerHTML = data.tracks.map((t, i) =>
            `<div class="fc-item ${selectedIndices.has(i) ? 'selected' : ''}" data-index="${i}">
              <span class="fc-check">${selectedIndices.has(i) ? '☑' : '☐'}</span>
              <span class="fc-title">${escHtml(t.title || '(unknown)')}</span>
              <span class="fc-artist">${escHtml(t.artist || '')}</span>
              <span class="fc-dur">${Math.floor(t.duration_secs/60)}:${String(t.duration_secs%60).padStart(2,'0')}</span>
            </div>`
          ).join('');
          listEl.querySelectorAll('.fc-item').forEach(el => {
            el.addEventListener('click', () => {
              const idx = parseInt(el.dataset.index);
              if (selectedIndices.has(idx)) selectedIndices.delete(idx);
              else selectedIndices.add(idx);
              el.classList.toggle('selected');
              el.querySelector('.fc-check').textContent = selectedIndices.has(idx) ? '☑' : '☐';
            });
          });
          statusEl.textContent = `Total: ${data.tracks.length} tracks, ${selectedIndices.size} selected`;
        } catch { statusEl.textContent = 'Failed to load playlist'; }
      };

      // Mode change -> show/hide quality/bitrate
      modeSel.addEventListener('change', () => {
        const mode = modeSel.value;
        qualityRow.style.display = mode === 'vbr' ? '' : 'none';
        bitrateRow.style.display = mode !== 'vbr' ? '' : 'none';
      });
      qualityRow.style.display = modeSel.value === 'vbr' ? '' : 'none';
      bitrateRow.style.display = modeSel.value !== 'vbr' ? '' : 'none';

      // Convert
      convertBtn.addEventListener('click', async () => {
        if (selectedIndices.size === 0) { statusEl.textContent = 'Select at least one track'; return; }
        try {
          const r = await fetch('/api/playlist');
          const data = await r.json();
          const fmt = fmtSel.value;
          const mode = modeSel.value;
          const quality = qualityInp.value;
          const bitrate = bitrateInp.value;
          const indices = [...selectedIndices].sort((a,b) => a-b);
          let converted = 0, failed = 0;
          for (const idx of indices) {
            const track = data.tracks[idx];
            const src = track.file_path;
            const dest = src.replace(/\.\w+$/, '.' + fmt);
            statusEl.textContent = `Converting (${converted + failed + 1}/${indices.length}): ${track.title || src}...`;
            const cmdStr = `tag format "${src}" "${dest}" --format ${fmt} --mode ${mode} --quality ${quality} --bitrate ${bitrate}`;
            const res = await api('POST', '/api/command', { command: cmdStr });
            if (res?.success) converted++; else failed++;
          }
          statusEl.textContent = `Done: ${converted} converted, ${failed} failed`;
        } catch { statusEl.textContent = 'Conversion error'; }
      });

      await loadTracks();
    },
    body: `
      <div style="margin-bottom:8px">
        <label>Format: </label>
        <select id="fc-format" style="margin-right:12px">
          <option value="mp3">MP3</option>
          <option value="flac">FLAC</option>
          <option value="wav">WAV</option>
          <option value="ogg">OGG</option>
          <option value="opus">Opus</option>
          <option value="m4a">M4A (AAC)</option>
          <option value="wma">WMA</option>
        </select>
        <label>Mode: </label>
        <select id="fc-mode">
          <option value="vbr">VBR</option>
          <option value="cbr">CBR</option>
          <option value="abr">ABR</option>
        </select>
      </div>
      <div id="fc-quality-row" style="margin-bottom:8px;display:none">
        <label>Quality (0-9, 0=best): </label>
        <input type="number" id="fc-quality" value="2" min="0" max="9" style="width:60px">
      </div>
      <div id="fc-bitrate-row" style="margin-bottom:8px;display:none">
        <label>Bitrate (kbps): </label>
        <input type="number" id="fc-bitrate" value="320" min="64" max="320" step="32" style="width:80px">
      </div>
      <div id="fc-list" style="max-height:250px;overflow-y:auto;border:1px solid var(--border);border-radius:6px;"></div>
      <div id="fc-status" style="margin-top:8px;font-size:11px;color:var(--text3)"></div>`,
    footer: `<button id="fc-convert" class="primary">Convert Selected</button><button data-dlg-close>Close</button>`
  });
}

// ===== Settings Dialog (7 tabs) =====
function showSettingsDialog() {
  showDialog({
    title: '设置',
    width: '680px',
    height: '520px',
    body: `
      <div id="settings-dlg" style="display:flex;gap:0;height:100%">
        <!-- Sidebar tabs -->
        <div id="settings-dlg-tabs" style="width:120px;flex-shrink:0;border-right:1px solid var(--border);padding:8px 0">
          <div class="settings-tab active" data-stab="appearance">外观</div>
          <div class="settings-tab" data-stab="playback">播放</div>
          <div class="settings-tab" data-stab="lyrics">歌词</div>
          <div class="settings-tab" data-stab="hotkeys">快捷键</div>
          <div class="settings-tab" data-stab="media">媒体库</div>
          <div class="settings-tab" data-stab="general">常规</div>
          <div class="settings-tab" data-stab="about">关于</div>
        </div>
        <!-- Content area -->
        <div id="settings-dlg-content" style="flex:1;overflow-y:auto;padding:12px 16px"></div>
      </div>
      <style>
        .settings-tab {
          padding: 8px 14px; font-size: 12px; cursor: pointer; color: var(--text2);
          border-left: 3px solid transparent; transition: all var(--transition);
        }
        .settings-tab:hover { color: var(--text); background: var(--hover); }
        .settings-tab.active { color: var(--accent); border-left-color: var(--accent); background: rgba(233,69,96,0.06); font-weight: 600; }
        .settings-dlg-group { margin-bottom: 16px; }
        .settings-dlg-group h3 { font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 1px; color: var(--text2); margin: 0 0 8px; padding-bottom: 4px; border-bottom: 1px solid var(--border); }
        .settings-dlg-row { display: flex; align-items: center; gap: 8px; padding: 5px 0; }
        .settings-dlg-row label { font-size: 12px; color: var(--text); min-width: 80px; flex-shrink: 0; }
        .settings-dlg-row select, .settings-dlg-row input[type="number"] { flex:1; padding:4px 6px; border:1px solid var(--border); border-radius:4px; background:var(--bg2); color:var(--text); font-size:12px; }
        .settings-dlg-row input[type="range"] { flex:1; accent-color: var(--accent); }
        .settings-dlg-row .eq-val { font-size:11px; color:var(--text2); width:36px; text-align:right; font-family:var(--font-mono); }
        .settings-dlg-about { text-align:center; padding:20px; }
        .settings-dlg-about h2 { margin:0 0 4px; font-size:18px; }
        .settings-dlg-about p { font-size:12px; color:var(--text2); margin:2px 0; }
      </style>
    `,
    onOpen: async () => {
      // Fetch config
      let config = {}, status = {};
      try {
        const data = await api('GET', '/api/config');
        if (data) config = data.config || {};
        const st = await api('GET', '/api/status');
        if (st) status = st;
      } catch {}

      const eqState = window._eqState || { enabled: false, bands: [] };
      const reverbState = window._reverbState || { enabled: false, mix: 50, time: 100 };

      const content = document.getElementById('settings-dlg-content');

      function renderTab(tabId) {
        let html = '';
        if (tabId === 'appearance') {
          const bright = config.appearance?.brightness || 'dark';
          html = \`
            <div class="settings-dlg-group">
              <h3>Theme</h3>
              <div class="settings-dlg-row"><label>Theme</label><select id="sd-theme">\${Object.entries(THEMES||{}).map(([k,v]) => '<option value="'+k+'" '+((localStorage.getItem('mp_theme')||'default')===k?'selected':'')+'>'+v.name+'</option>').join('')}</select></div>
              <div class="settings-dlg-row"><label>Dark Mode</label><div class="toggle \${config.appearance?.dark_mode !== false ? 'on' : ''}" id="sd-dark"></div></div>
              <div class="settings-dlg-row"><label>Panel Opacity</label><input type="range" id="sd-opacity" min="0.3" max="1" step="0.05" value="\${localStorage.getItem('mp_opacity')||'1'}"><span class="eq-val" id="sd-opacity-val">\${localStorage.getItem('mp_opacity')||'1'}</span></div>
            </div>
            <div class="settings-dlg-group">
              <h3>Spectrum</h3>
              <div class="settings-dlg-row"><label>Columns</label><select id="sd-spectrum-col"><option value="16">16</option><option value="32">32</option><option value="64">64</option><option value="128">128</option></select></div>
              <div class="settings-dlg-row"><label>Style</label><select id="sd-spectrum-style"><option value="log">Log</option><option value="linear">Linear</option></select></div>
              <div class="settings-dlg-row"><label>Visual</label><select id="sd-spectrum-visual"><option value="modern">Modern</option><option value="classic">Classic</option></select></div>
              <div class="settings-dlg-row"><label>Reflection</label><div class="toggle on" id="sd-spectrum-reflection"></div></div>
              <div class="settings-dlg-row"><label>Height</label><input type="range" id="sd-spectrum-height" min="40" max="200" step="5" value="\${localStorage.getItem('mp_spectrum_height')||'80'}"><span class="eq-val" id="sd-spectrum-height-val">\${localStorage.getItem('mp_spectrum_height')||'80'}</span></div>
            </div>\`;
        } else if (tabId === 'playback') {
          html = \`
            <div class="settings-dlg-group">
              <h3>Playback</h3>
              <div class="settings-dlg-row"><label>Engine</label><select id="sd-engine"><option value="bass" \${config.play?.engine==='bass'?'selected':''}>BASS</option><option value="ffmpeg" \${config.play?.engine==='ffmpeg'?'selected':''}>FFmpeg</option></select></div>
              <div class="settings-dlg-row"><label>Default Vol</label><input type="number" id="sd-volume" value="\${config.play?.default_volume??80}" min="0" max="100"></div>
              <div class="settings-dlg-row"><label>Fade Effect</label><div class="toggle \${config.play?.fade_effect?'on':''}" id="sd-fade"></div></div>
              <div class="settings-dlg-row"><label>Fade Time</label><input type="number" id="sd-fade-time" value="\${config.play?.fade_time??500}" min="0" max="5000" step="100"></div>
              <div class="settings-dlg-row"><label>Auto Play</label><div class="toggle \${config.play?.auto_play_when_start?'on':''}" id="sd-auto-play"></div></div>
              <div class="settings-dlg-row"><label>Output Device</label><select id="sd-output-dev"><option value="default">Default</option></select></div>
            </div>
            <div class="settings-dlg-group">
              <h3>Speed / Pitch</h3>
              <div class="settings-dlg-row"><label>Speed</label><input type="range" id="sd-speed" min="0.5" max="2" step="0.05" value="\${status.speed||1}"><span class="eq-val" id="sd-speed-val">\${(status.speed||1).toFixed(2)}x</span></div>
              <div class="settings-dlg-row"><label>Pitch</label><input type="range" id="sd-pitch" min="-12" max="12" step="1" value="\${status.pitch||0}"><span class="eq-val" id="sd-pitch-val">\${status.pitch>0?'+':''}\${status.pitch||0}</span></div>
            </div>\`;
        } else if (tabId === 'lyrics') {
          html = \`
            <div class="settings-dlg-group">
              <h3>Lyrics</h3>
              <div class="settings-dlg-row"><label>Show Translation</label><div class="toggle \${config.lyric?.show_translate?'on':''}" id="sd-translate"></div></div>
              <div class="settings-dlg-row"><label>Fuzzy Match</label><div class="toggle \${config.lyric?.fuzzy_match?'on':''}" id="sd-fuzzy"></div></div>
            </div>
            <div class="settings-dlg-group">
              <h3>Desktop Lyrics</h3>
              <div class="settings-dlg-row"><label>Color Scheme</label><select id="sd-lyric-color">\${Object.entries(LYRIC_COLORS||{}).map(([k,v]) => '<option value="'+k+'" '+(localStorage.getItem('mp_lyric_color')||'default')===k?'selected':'')+'>'+v.name+'</option>').join('')}</select></div>
              <div class="settings-dlg-row"><label>Tri-Color</label><select id="sd-lyric-tricolor"><option value="">Off</option><option value="sunset">Sunset</option><option value="ocean">Ocean</option><option value="forest">Forest</option><option value="fire">Fire</option><option value="neon">Neon</option></select></div>
              <div class="settings-dlg-row"><label>Opacity</label><input type="range" id="sd-lyric-opacity" min="0.2" max="1" step="0.05" value="\${localStorage.getItem('mp_lyric_opacity')||'0.55'}"><span class="eq-val" id="sd-lyric-opacity-val">\${localStorage.getItem('mp_lyric_opacity')||'0.55'}</span></div>
              <div class="settings-dlg-row"><label>Align</label><select id="sd-lyric-align"><option value="center">Center</option><option value="left">Left</option><option value="right">Right</option></select></div>
              <div class="settings-dlg-row"><label>Font Size</label><input type="range" id="sd-lyric-size" min="10" max="24" step="1" value="\${localStorage.getItem('mp_lyric_size')||'14'}"><span class="eq-val" id="sd-lyric-size-val">\${localStorage.getItem('mp_lyric_size')||'14'}px</span></div>
              <div class="settings-dlg-row"><label>Line Height</label><input type="range" id="sd-lyric-height" min="1" max="3" step="0.1" value="\${localStorage.getItem('mp_lyric_height')||'1.5'}"><span class="eq-val" id="sd-lyric-height-val">\${localStorage.getItem('mp_lyric_height')||'1.5'}</span></div>
            </div>\`;
        } else if (tabId === 'hotkeys') {
          const hotkeys = loadHotkeys ? loadHotkeys() : {};
          const entries = Object.entries(hotkeys).length ? Object.entries(hotkeys) : [['Space','pause'],['Ctrl+Left','prev'],['Ctrl+Right','next'],['Ctrl+Up','volume_up'],['Ctrl+Down','volume_down'],['F11','fullscreen'],['Ctrl+O','open'],['Ctrl+M','media_lib'],['Ctrl+Alt+M','mini_mode'],['?','shortcuts']];
          html = \`<div class="settings-dlg-group"><h3>Keyboard Shortcuts</h3><div style="font-size:11px;color:var(--text3);margin-bottom:8px">Click a shortcut to record a new key combination.</div>\`;
          entries.forEach(([key, action]) => {
            const label = (ACTION_LABELS||{})[action] || action;
            html += \`<div class="settings-dlg-row" style="cursor:pointer" data-hk-action="\${action}"><label style="flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis">\${label}</label><span style="font-size:11px;padding:2px 8px;border:1px solid var(--border);border-radius:4px;color:var(--accent);font-family:var(--font-mono)">\${key}</span></div>\`;
          });
          html += \`<button id="sd-hotkey-reset" style="margin-top:8px;padding:4px 12px;font-size:11px;border:1px solid var(--border);border-radius:4px;background:var(--bg2);color:var(--text);cursor:pointer">Reset Defaults</button></div>\`;
        } else if (tabId === 'media') {
          html = \`
            <div class="settings-dlg-group">
              <h3>Media Library</h3>
              <div class="settings-dlg-row"><label>Auto Scan</label><div class="toggle \${config.media_lib?.auto_scan?'on':''}" id="sd-auto-scan"></div></div>
              <div class="settings-dlg-row"><label>Min Duration</label><input type="number" id="sd-min-dur" value="\${config.media_lib?.min_duration_secs??0}" min="0"></div>
              <div class="settings-dlg-row"><label>Rescan</label><button id="sd-rescan" style="padding:4px 12px;font-size:11px;border:1px solid var(--border);border-radius:4px;background:var(--bg2);color:var(--text);cursor:pointer">Scan Now</button></div>
            </div>
            <div class="settings-dlg-group">
              <h3>Last.fm</h3>
              <div class="settings-dlg-row"><label>Enable</label><div class="toggle \${config.lastfm?.enabled?'on':''}" id="sd-lastfm-enable"></div></div>
              <div class="settings-dlg-row"><label>Username</label><input type="text" id="sd-lastfm-user" value="\${config.lastfm?.username||''}" style="flex:1;padding:4px 6px;border:1px solid var(--border);border-radius:4px;background:var(--bg2);color:var(--text);font-size:12px"></div>
            </div>\`;
        } else if (tabId === 'general') {
          html = \`
            <div class="settings-dlg-group">
              <h3>General</h3>
              <div class="settings-dlg-row"><label>Language</label><select id="sd-language"><option value="zh-CN" \${config.general?.language==='zh-CN'?'selected':''}>中文</option><option value="en" \${config.general?.language==='en'?'selected':''}>English</option><option value="ja" \${config.general?.language==='ja'?'selected':''}>日本語</option></select></div>
              <div class="settings-dlg-row"><label>Auto DL Lyrics</label><div class="toggle \${config.general?.auto_download_lyric?'on':''}" id="sd-dl-lyric"></div></div>
              <div class="settings-dlg-row"><label>Auto DL Cover</label><div class="toggle \${config.general?.auto_download_album_cover!==false?'on':''}" id="sd-dl-cover"></div></div>
              <div class="settings-dlg-row"><label>Check Updates</label><div class="toggle \${config.general?.check_update_when_start!==false?'on':''}" id="sd-check-update"></div></div>
              <div class="settings-dlg-row"><label>Minimize to Tray</label><div class="toggle \${config.general?.minimize_to_notify_icon?'on':''}" id="sd-min-tray"></div></div>
            </div>\`;
        } else if (tabId === 'about') {
          html = \`
            <div class="settings-dlg-about">
              <h2>1028 Music Player</h2>
              <p>Version 1.0.0</p>
              <p style="margin-top:12px">基于 Rust + Tauri + WebView 构建</p>
              <p>音频引擎: BASS / FFmpeg / MCI</p>
              <p style="margin-top:12px;color:var(--text3)">MusicPlayer2 的 Rust 跨平台重写版本</p>
              <p style="margin-top:16px;font-size:11px;color:var(--text3)">
                <a href="#" style="color:var(--accent);text-decoration:none" id="sd-license-link">开源许可</a>
                &nbsp;·&nbsp;
                <a href="#" style="color:var(--accent);text-decoration:none" id="sd-credits-link">致谢</a>
              </p>
              <div id="sd-license-text" style="display:none;margin-top:8px;padding:8px;background:var(--bg3);border-radius:4px;font-size:10px;color:var(--text3);text-align:left;max-height:120px;overflow-y:auto">
                MIT License<br><br>
                Copyright (c) 2024-2026 MusicPlayer2 Contributors<br><br>
                Permission is hereby granted, free of charge, to any person obtaining a copy...
              </div>
            </div>\`;
        }
        content.innerHTML = html;
        // Bind settings events after render
        bindSettingsDlgEvents(tabId, config, status, eqState, reverbState);
      }

      renderTab('appearance');

      // Tab switching
      document.querySelectorAll('.settings-tab').forEach(tab => {
        tab.addEventListener('click', () => {
          document.querySelectorAll('.settings-tab').forEach(t => t.classList.remove('active'));
          tab.classList.add('active');
          renderTab(tab.dataset.stab);
        });
      });

      // Load audio devices
      setTimeout(async () => {
        const devSel = document.getElementById('sd-output-dev');
        if (devSel) {
          try {
            const devs = await api('GET', '/api/devices');
            if (devs?.devices) {
              devs.devices.forEach(d => {
                const opt = document.createElement('option');
                opt.value = d.name || d.index; opt.textContent = d.name || 'Device ' + d.index;
                devSel.appendChild(opt);
              });
            }
          } catch {}
        }
      }, 100);
    },
  });
}

// Settings dialog event bindings
function bindSettingsDlgEvents(tabId, config, status, eqState, reverbState) {
  // Toggle helper
  const bindToggle = (id, apiKey, lsKey) => {
    const el = document.getElementById(id);
    if (!el) return;
    el.addEventListener('click', function () {
      const on = this.classList.toggle('on');
      if (apiKey) api('POST', '/api/config', { key: apiKey, value: on });
      if (lsKey) localStorage.setItem(lsKey, on ? 'true' : 'false');
    });
  };
  const bindSelect = (id, apiKey, lsKey) => {
    const el = document.getElementById(id);
    if (!el) return;
    el.addEventListener('change', () => {
      if (apiKey) api('POST', '/api/config', { key: apiKey, value: el.value });
      if (lsKey) localStorage.setItem(lsKey, el.value);
      if (id === 'sd-spectrum-col' || id === 'sd-spectrum-visual' || id === 'sd-spectrum-height') resizeCanvas();
    });
  };
  const bindRange = (id, valId, apiCmd, lsKey) => {
    const el = document.getElementById(id);
    if (!el) return;
    el.addEventListener('input', () => {
      if (valId) document.getElementById(valId).textContent = el.value + (id.includes('speed') ? 'x' : id.includes('size') ? 'px' : '');
    });
    el.addEventListener('change', () => {
      if (apiCmd) api('POST', '/api/command', { command: apiCmd.replace('{v}', el.value) });
      if (lsKey) localStorage.setItem(lsKey, el.value);
      if (id === 'sd-spectrum-height') resizeCanvas();
    });
  };
  const bindBtn = (id, cmd) => {
    const el = document.getElementById(id);
    if (el) el.addEventListener('click', () => api('POST', '/api/command', { command: cmd }));
  };

  if (tabId === 'appearance') {
    bindSelect('sd-theme', null, null);
    document.getElementById('sd-theme')?.addEventListener('change', function () {
      const theme = THEMES?.[this.value];
      if (theme) { applyTheme(this.value); localStorage.setItem('mp_theme', this.value); }
    });
    bindToggle('sd-dark', 'appearance.dark_mode');
    bindRange('sd-opacity', 'sd-opacity-val', null, 'mp_opacity');
    bindSelect('sd-spectrum-col', 'appearance.spectrum_columns');
    bindSelect('sd-spectrum-style', 'appearance.spectrum_style');
    document.getElementById('sd-spectrum-visual')?.addEventListener('change', function () {
      localStorage.setItem('mp_spectrum_visual_style', this.value); resizeCanvas();
    });
    document.getElementById('sd-spectrum-reflection')?.addEventListener('click', function () {
      const on = this.classList.toggle('on'); localStorage.setItem('mp_spectrum_reflection', on ? 'true' : 'false');
    });
    bindRange('sd-spectrum-height', 'sd-spectrum-height-val', null, null);
  }
  if (tabId === 'playback') {
    bindSelect('sd-engine', 'play.engine');
    document.getElementById('sd-volume')?.addEventListener('change', function () {
      api('POST', '/api/config', { key: 'play.default_volume', value: parseInt(this.value) });
    });
    bindToggle('sd-fade', 'play.fade_effect');
    document.getElementById('sd-fade-time')?.addEventListener('change', function () {
      api('POST', '/api/config', { key: 'play.fade_time', value: parseInt(this.value) });
    });
    bindToggle('sd-auto-play', 'play.auto_play_when_start');
    bindRange('sd-speed', 'sd-speed-val', 'speed set {v}');
    bindRange('sd-pitch', 'sd-pitch-val', 'pitch set {v}');
  }
  if (tabId === 'lyrics') {
    bindToggle('sd-translate', 'lyric.show_translate');
    bindToggle('sd-fuzzy', 'lyric.fuzzy_match');
    document.getElementById('sd-lyric-color')?.addEventListener('change', function () {
      localStorage.setItem('mp_lyric_color', this.value);
    });
    document.getElementById('sd-lyric-tricolor')?.addEventListener('change', function () {
      localStorage.setItem('mp_lyric_tricolor', this.value);
    });
    bindRange('sd-lyric-opacity', 'sd-lyric-opacity-val', null, 'mp_lyric_opacity');
    document.getElementById('sd-lyric-align')?.addEventListener('change', function () {
      localStorage.setItem('mp_lyric_align', this.value);
    });
    bindRange('sd-lyric-size', 'sd-lyric-size-val', null, 'mp_lyric_size');
    bindRange('sd-lyric-height', 'sd-lyric-height-val', null, 'mp_lyric_height');
  }
  if (tabId === 'media') {
    bindToggle('sd-auto-scan', 'media_lib.auto_scan');
    document.getElementById('sd-min-dur')?.addEventListener('change', function () {
      api('POST', '/api/config', { key: 'media_lib.min_duration_secs', value: parseInt(this.value) });
    });
    bindBtn('sd-rescan', 'media rescan');
    bindToggle('sd-lastfm-enable', 'lastfm.enabled');
    document.getElementById('sd-lastfm-user')?.addEventListener('change', function () {
      api('POST', '/api/config', { key: 'lastfm.username', value: this.value });
    });
  }
  if (tabId === 'general') {
    bindSelect('sd-language', 'general.language');
    bindToggle('sd-dl-lyric', 'general.auto_download_lyric');
    bindToggle('sd-dl-cover', 'general.auto_download_album_cover');
    bindToggle('sd-check-update', 'general.check_update_when_start');
    bindToggle('sd-min-tray', 'general.minimize_to_notify_icon');
  }
  if (tabId === 'hotkeys') {
    document.getElementById('sd-hotkey-reset')?.addEventListener('click', () => {
      const defaults = {'Space':'pause','Ctrl+Left':'prev','Ctrl+Right':'next','Ctrl+Up':'volume_up','Ctrl+Down':'volume_down','F11':'fullscreen','Ctrl+O':'open','Ctrl+M':'media_lib','Ctrl+Alt+M':'mini_mode','?':'shortcuts'};
      saveHotkeys(defaults);
      renderTab('hotkeys'); // Re-render
    });
  }
  if (tabId === 'about') {
    document.getElementById('sd-license-link')?.addEventListener('click', (e) => {
      e.preventDefault();
      const el = document.getElementById('sd-license-text');
      el.style.display = el.style.display === 'none' ? 'block' : 'none';
    });
  }
}

// Hotkey defaults
const DEFAULT_HOTKEYS = {
  'Space': 'pause', 'Ctrl+Left': 'prev', 'Ctrl+Right': 'next',
  'Ctrl+Up': 'volume_up', 'Ctrl+Down': 'volume_down',
  'F11': 'fullscreen', 'Ctrl+O': 'open', 'Ctrl+M': 'media_lib',
  'Ctrl+Alt+M': 'mini_mode', '?': 'shortcuts',
};
const ACTION_LABELS = {
  pause: 'Play/Pause', prev: 'Previous Track', next: 'Next Track',
  stop: 'Stop', volume_up: 'Volume +5', volume_down: 'Volume -5',
  fullscreen: 'Toggle Fullscreen', open: 'Open File',
  media_lib: 'Media Library', mini_mode: 'Mini Mode', shortcuts: 'Shortcut Help',
};
async function fetchEqState() {
  try {
    const r = await fetch('/api/eq');
    const data = await r.json();
    window._eqState = data;
    const bandContainer = document.getElementById('eq-bands');
    if (bandContainer) {
      const freqs = ['31','62','125','250','500','1k','2k','4k','8k','16k'];
      bandContainer.style.display = data.enabled ? 'grid' : 'none';
      bandContainer.innerHTML = data.bands.map((g, i) =>
        `<div class="eq-band-row"><label>${freqs[i]}Hz</label><input type="range" min="-15" max="15" value="${g}" step="1" data-band="${i}"><span class="eq-val">${g > 0 ? '+' : ''}${g}dB</span></div>`
      ).join('');
      bandContainer.querySelectorAll('input[type="range"]').forEach(el => {
        el.addEventListener('input', () => {
          const val = el.value;
          el.nextElementSibling.textContent = `${val > 0 ? '+' : ''}${val}dB`;
        });
        el.addEventListener('change', () => {
          api('POST', '/api/command', { command: `eq set ${el.dataset.band} ${el.value}` });
        });
      });
    }
    const toggle = document.getElementById('set-eq-enable');
    if (toggle) toggle.classList.toggle('on', data.enabled);
    // Populate user presets in dropdown
    const sel = document.getElementById('set-eq-preset');
    if (sel) {
      // Remove old user presets
      Array.from(sel.options).forEach(o => { if (o.dataset.user) o.remove(); });
      const userPresets = loadUserEqPresets();
      userPresets.forEach(name => {
        const opt = document.createElement('option');
        opt.value = name;
        opt.textContent = '★ ' + name;
        opt.dataset.user = '1';
        sel.appendChild(opt);
      });
    }
  } catch {}
}

// User EQ presets (localStorage)
function loadUserEqPresets() {
  try { return JSON.parse(localStorage.getItem('mp_eq_presets') || '{}'); } catch { return {}; }
}
function saveUserEqPresets(map) { localStorage.setItem('mp_eq_presets', JSON.stringify(map)); }

async function fetchReverbState() {
  try {
    const r = await fetch('/api/reverb');
    const data = await r.json();
    window._reverbState = data;
    const ctrl = document.getElementById('reverb-controls');
    if (ctrl) ctrl.style.display = data.enabled ? 'block' : 'none';
    const toggle = document.getElementById('set-reverb-enable');
    if (toggle) toggle.classList.toggle('on', data.enabled);
    const mixSlider = document.getElementById('set-reverb-mix');
    if (mixSlider) mixSlider.value = data.mix;
    const timeSlider = document.getElementById('set-reverb-time');
    if (timeSlider) timeSlider.value = data.time;
  } catch {}
}

// ===== File Path Dialogs =====
function showOpenFileDialog() {
  showDialog({
    title: '打开文件',
    width: '420px',
    body: '<div style="margin-bottom:8px;font-size:12px;color:var(--text2)">输入文件路径（支持拖放文件到窗口）</div><input type="text" id="file-path-input" placeholder="例如: C:\\Music\\song.mp3" style="width:100%;padding:8px;border:1px solid var(--border);border-radius:4px;background:var(--bg);color:var(--text);font-size:13px" />',
    footer: '<button class="primary" id="file-path-play-btn">播放</button><button data-dlg-close>取消</button>',
    onOpen: () => {
      document.getElementById('file-path-play-btn')?.addEventListener('click', () => {
        const val = document.getElementById('file-path-input')?.value?.trim();
        if (val) { cmd('play "' + val.replace(/"/g, '\\"') + '"'); hideDialog(); }
      });
      setTimeout(() => document.getElementById('file-path-input')?.focus(), 100);
    }
  });
}

function showOpenFolderDialog() {
  showDialog({
    title: '打开文件夹',
    width: '420px',
    body: '<div style="margin-bottom:8px;font-size:12px;color:var(--text2)">输入文件夹路径</div><input type="text" id="folder-path-input" placeholder="例如: C:\\Music\\" style="width:100%;padding:8px;border:1px solid var(--border);border-radius:4px;background:var(--bg);color:var(--text);font-size:13px" />',
    footer: '<button class="primary" id="folder-path-play-btn">播放</button><button data-dlg-close>取消</button>',
    onOpen: () => {
      document.getElementById('folder-path-play-btn')?.addEventListener('click', () => {
        const val = document.getElementById('folder-path-input')?.value?.trim();
        if (val) { cmd('play "' + val.replace(/"/g, '\\"') + '" --add'); hideDialog(); }
      });
      setTimeout(() => document.getElementById('folder-path-input')?.focus(), 100);
    }
  });
}

// ===== URL Dialog =====
function showUrlDialog() {
  showDialog({
    title: '打开 URL',
    width: '380px',
    body: '<input type="text" id="url-input" placeholder="例如: https://example.com/song.mp3" style="width:100%;padding:8px;border:1px solid var(--border);border-radius:4px;background:var(--bg);color:var(--text);font-size:13px" />',
    footer: '<button class="primary" id="url-play-btn">播放</button><button data-dlg-close>取消</button>',
    onOpen: () => {
      document.getElementById('url-play-btn')?.addEventListener('click', () => {
        const val = document.getElementById('url-input')?.value?.trim();
        if (val) { cmd('play "' + val + '"'); hideDialog(); }
      });
      setTimeout(() => document.getElementById('url-input')?.focus(), 100);
    }
  });
}

// ===== Last.fm Settings Dialog =====
async function showLastfmDialog() {
  const cfg = await api('GET', '/api/config');
  const lfm = cfg?.lastfm || {};
  const status = lfm.enabled ? (lfm.session_key ? '已认证' : '未登录') : '已禁用';
  showDialog({
    title: 'Last.fm 设置',
    width: '420px',
    body: `
      <div class="prop-grid">
        <span class="prop-label">状态</span><span class="prop-value" id="lfm-status">${status}</span>
        <span class="prop-label">用户名</span><span class="prop-value">${escHtml(lfm.username || '-')}</span>
        <span class="prop-label">自动提交</span><span class="prop-value">${lfm.auto_scrobble ? '开启' : '关闭'}</span>
      </div>
      <div style="margin-top:12px;padding:12px;background:var(--bg);border-radius:6px;font-size:12px;color:var(--text2)">
        <div style="margin-bottom:8px;font-weight:600;color:var(--text)">登录 Last.fm</div>
        <input type="text" id="lfm-api-key" placeholder="API Key" style="width:100%;padding:6px;margin-bottom:6px;border:1px solid var(--border);border-radius:4px;background:var(--bg2);color:var(--text);font-size:12px" />
        <input type="text" id="lfm-shared-secret" placeholder="Shared Secret" style="width:100%;padding:6px;margin-bottom:6px;border:1px solid var(--border);border-radius:4px;background:var(--bg2);color:var(--text);font-size:12px" />
        <input type="text" id="lfm-username" placeholder="用户名" style="width:100%;padding:6px;margin-bottom:6px;border:1px solid var(--border);border-radius:4px;background:var(--bg2);color:var(--text);font-size:12px" />
        <div style="font-size:11px;color:var(--text3);margin-bottom:6px">
          1. 在 <a href="https://www.last.fm/api/" target="_blank" style="color:var(--accent)">last.fm/api</a> 注册应用获取 API Key
        </div>
        <div style="font-size:11px;color:var(--text3);margin-bottom:6px">
          2. 访问授权 URL 获取 token: <code style="word-break:break-all" id="lfm-auth-url"></code>
        </div>
        <input type="text" id="lfm-token" placeholder="授权 Token" style="width:100%;padding:6px;margin-bottom:6px;border:1px solid var(--border);border-radius:4px;background:var(--bg2);color:var(--text);font-size:12px" />
        <div id="lfm-login-result" style="font-size:12px;margin-top:4px;color:var(--success)"></div>
      </div>`,
    footer: '<button class="primary" id="lfm-login-btn">登录</button><button id="lfm-status-btn">查看状态</button><button data-dlg-close>关闭</button>',
    onOpen: () => {
      const keyInput = document.getElementById('lfm-api-key');
      const secretInput = document.getElementById('lfm-shared-secret');
      const userInput = document.getElementById('lfm-username');
      const tokenInput = document.getElementById('lfm-token');
      const authUrl = document.getElementById('lfm-auth-url');

      function updateAuthUrl() {
        const key = keyInput?.value?.trim();
        authUrl.textContent = key ? `https://www.last.fm/api/auth/?api_key=${key}&token=` : '(填入 API Key 后显示)';
      }
      keyInput?.addEventListener('input', updateAuthUrl);
      updateAuthUrl();

      document.getElementById('lfm-login-btn')?.addEventListener('click', async () => {
        const key = keyInput?.value?.trim();
        const secret = secretInput?.value?.trim();
        const user = userInput?.value?.trim();
        const token = tokenInput?.value?.trim();
        if (!key || !secret || !user || !token) {
          document.getElementById('lfm-login-result').textContent = '请填写所有字段';
          document.getElementById('lfm-login-result').style.color = 'var(--warning)';
          return;
        }
        const res = await api('POST', '/api/command', { command: `lastfm login ${user} "${key} ${secret} ${token}"` });
        const el = document.getElementById('lfm-login-result');
        if (res?.success) {
          el.textContent = '登录成功！';
          el.style.color = 'var(--success)';
          document.getElementById('lfm-status').textContent = '已认证';
        } else {
          el.textContent = '登录失败: ' + (res?.error || '未知错误');
          el.style.color = 'var(--accent)';
        }
      });

      document.getElementById('lfm-status-btn')?.addEventListener('click', () => {
        cmd('lastfm status');
      });
    }
  });
}

// ===== Init =====
function resizeCanvas() {
  const container = document.getElementById('spectrum-section');
  const cfg = getSpectrumConfig();
  canvas.width = container.clientWidth || 640;
  canvas.height = parseInt(cfg.height) || 80;
}

window._useDlgSettings = true;
window.addEventListener('resize', resizeCanvas);
resizeCanvas();
renderTitleBar();
renderControls();
loadLayout();
loadTheme();
updateConnectionStatus();
initMenuSystem();
bindControls();
initLayoutObserver();
initMediaLibrary();
setupMediaSession();
checkBackend();
if ('Notification' in window && Notification.permission === 'default') Notification.requestPermission();