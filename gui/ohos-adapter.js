// HackMagic Music Player - HarmonyOS Bridge Adapter
// This file is loaded before app.js when running in HarmonyOS WebView
// NOTE: app.js also declares 'const OHOS', so we must NOT redeclare it here.

if (typeof ohosBridge !== 'undefined') {
  console.log('[OHOS Adapter] Running in HarmonyOS WebView');

  // ---- Callback dispatcher ----
  // Native BridgeProxy sends all events through one function name.
  // We dispatch by event type to individual handlers.
  window.ohosCallback = {
    _listeners: {},
    on(event, handler) {
      if (!this._listeners[event]) this._listeners[event] = [];
      this._listeners[event].push(handler);
    },
    off(event, handler) {
      if (!this._listeners[event]) return;
      if (handler) this._listeners[event] = this._listeners[event].filter(h => h !== handler);
      else delete this._listeners[event];
    },
    emit(event, data) {
      const handlers = this._listeners[event];
      if (handlers) handlers.forEach(h => { try { h(data); } catch(e) { console.error('[OHOS CB]', e); } });
    },

    // Central dispatcher: called by native bridge for ALL events
    onStateChange(eventName, dataStr) {
      let data;
      try { data = typeof dataStr === 'string' ? JSON.parse(dataStr) : dataStr; } catch (_) { data = dataStr; }
      window.ohosCallback.emit(eventName, data);

      switch (eventName) {
        case 'stateChange':
          if (data.state === 'playing') { state.playing = true; state.paused = false; }
          else if (data.state === 'paused') { state.playing = false; state.paused = true; }
          else { state.playing = false; state.paused = false; }
          updatePlayButton();
          const albumArt = document.getElementById('album-art');
          if (albumArt) albumArt.classList.toggle('playing', state.playing);
          break;
        case 'timeUpdate':
          if (!state.isDragging) {
            state.position = data.position;
            updateProgress();
          }
          break;
        case 'durationUpdate':
          state.duration = data.duration;
          updateProgress();
          break;
        case 'trackChange':
          if (data) {
            state.position = data.position || 0;
            state.duration = data.duration || 0;
            document.getElementById('track-title').textContent = data.title || data.file_path?.split(/[/\\]/).pop() || '';
            document.getElementById('track-artist').textContent = data.artist || 'Unknown Artist';
            document.getElementById('track-album').textContent = data.album ? `Album: ${data.album}` : '';
            document.title = `${data.title || ''} - ${data.artist || 'Unknown'} - HackMagic Music Player`;
            const title = data.title || data.file_path?.split(/[/\\]/).pop() || '';
            if ('Notification' in window && Notification.permission === 'granted') {
              new Notification('正在播放', { body: `${title} - ${data.artist || 'Unknown Artist'}` });
            }
          }
          break;
        case 'error':
          console.error('[OHOS Adapter] Player error:', data);
          break;
      }
    },

    onPickResult(dataStr) {
      let data;
      try { data = typeof dataStr === 'string' ? JSON.parse(dataStr) : dataStr; } catch (_) { data = dataStr; }
      window.ohosCallback.emit('pickResult', data);
    },

    onPlaylistChange(dataStr) {
      let data;
      try { data = typeof dataStr === 'string' ? JSON.parse(dataStr) : dataStr; } catch (_) { data = dataStr; }
      window.ohosCallback.emit('playlistChange', data);
    },
  };

  // Track file path to index mapping for in-memory playlist
  let _plTracks = [];
  let _plIndex = -1;
  let _plName = 'Playlist';

  window._ohosApi = async function(method, path, body) {
    try {
      if (method === 'GET') {
        if (path === '/api/status') return JSON.parse(ohosBridge.getStatus());
        if (path === '/api/health') return { ok: true };

        // Playlist endpoints
        if (path === '/api/playlist') {
          return { tracks: _plTracks, current_index: _plIndex, name: _plName };
        }
        if (path === '/api/playlist/list') {
          return [{ name: _plName, count: _plTracks.length }];
        }
        if (path === '/api/playlist/queue') return [];

        // Media library
        if (path === '/api/media/all') {
          const result = ohosBridge.scanMedia();
          return result ? { tracks: JSON.parse(result) } : { tracks: [] };
        }
        if (path === '/api/media/favourites') return { tracks: [], count: 0 };
        if (path.match(/^\/api\/media\/(artists|recent|albums)$/)) return [];
        if (path === '/api/media/search') return { tracks: [] };

        // Lyric & Cover
        if (path === '/api/lyric') return { has_lyrics: false, lines: [] };
        if (path === '/api/cover') return null;

        // Config
        if (path === '/api/config') return _getDefaultConfig();

        // EQ
        if (path === '/api/equalizer') return JSON.parse(ohosBridge.eqGetState());

        // Misc
        if (path === '/api/stats') return { total_played: 0, total_time_secs: 0, top_artists: [], top_tracks: [] };
        if (path === '/api/audio/devices') return [{ id: 0, name: 'Default Device' }];
        if (path === '/api/reverb') return { enabled: false, mix: 0.3, time: 1000 };

        // Tag reading — native file metadata reading TBD
        if (path.startsWith('/api/tag/read')) {
          return { title: '', artist: '', album: '', has_cover: false };
        }
      }

      if (method === 'POST') {
        if (path === '/api/command') {
          const cmd = body?.command || '';
          const parts = cmd.trim().split(/\s+/);
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
            case 'volume': ohosBridge.setVolume(parseInt(parts[parts.length - 1])); break;
            case 'repeat': ohosBridge.setRepeat(args); break;
            case 'jump': { const idx = parseInt(args); if (!isNaN(idx)) { _plIndex = idx; ohosBridge.playAtIndex(idx); } break; }
            case 'play_index': ohosBridge.playAtIndex(parseInt(args)); break;
            case 'remove_index': ohosBridge.removeFromPlaylist(parseInt(args)); break;
            case 'clear': ohosBridge.clearPlaylist(); _plTracks = []; _plIndex = -1; break;
            case 'playlist': _handlePlaylistCmd(parts[1], parts.slice(2).join(' ')); break;
            case 'open-location': break;
          }
          return { success: true, error: null };
        }

        // EQ
        if (path === '/api/equalizer/band') { ohosBridge.eqSetBand(body.band, body.gain); return { ok: true }; }
        if (path === '/api/equalizer/enable') { ohosBridge.eqEnable(body.enabled); return { ok: true }; }
        if (path === '/api/equalizer/preset') { ohosBridge.eqApplyPreset(body.name); return { ok: true }; }
        if (path === '/api/equalizer/reset') { ohosBridge.eqReset(); return { ok: true }; }

        // Config
        if (path === '/api/config') { _handleConfigSet(body.key, body.value); return { success: true, error: null }; }

        // Lyric search
        if (path === '/api/lyric/search') return [];
        if (path === '/api/tag/read') return { title: '', artist: '', album: '', has_cover: false };
        if (path === '/api/media/search') return { tracks: [] };
      }
    } catch (e) {
      console.error('[OHOS Adapter] _ohosApi error:', path, e);
    }
    return null;
  };

  // ---- In-memory playlist management ----
  function _handlePlaylistCmd(action, args) {
    switch (action) {
      case 'add':
        if (args) {
          args.split(/\s+/).filter(Boolean).forEach(f => {
            _plTracks.push({ file_path: f, title: f.split(/[/\\]/).pop(), artist: 'Unknown', album: '', duration: 0, is_cue: false });
          });
        }
        break;
      case 'remove': {
        const idx = parseInt(args);
        if (!isNaN(idx) && idx >= 0 && idx < _plTracks.length) {
          _plTracks.splice(idx, 1);
          if (idx < _plIndex) _plIndex--;
          else if (idx === _plIndex) _plIndex = -1;
        }
        break;
      }
      case 'clear': _plTracks = []; _plIndex = -1; break;
      case 'new': _plTracks = []; _plIndex = -1; if (args) _plName = args.replace(/"/g, ''); break;
      case 'shuffle':
        for (let i = _plTracks.length - 1; i > 0; i--) {
          const j = Math.floor(Math.random() * (i + 1));
          [_plTracks[i], _plTracks[j]] = [_plTracks[j], _plTracks[i]];
        }
        break;
      case 'sort':
        if (args === 'title' || args === 'name') _plTracks.sort((a, b) => (a.title || '').localeCompare(b.title || ''));
        else if (args === 'artist') _plTracks.sort((a, b) => (a.artist || '').localeCompare(b.artist || ''));
        else if (args === 'album') _plTracks.sort((a, b) => (a.album || '').localeCompare(b.album || ''));
        else if (args === 'path') _plTracks.sort((a, b) => (a.file_path || '').localeCompare(b.file_path || ''));
        break;
    }
  }

  // ---- In-memory config ----
  let _configStore = {};
  function _getDefaultConfig() {
    return {
      play: {
        engine: 'bass', stop_when_error: true, auto_play_when_start: false,
        output_device: '-1', fade_effect: true, fade_time: 500, default_volume: 80,
        always_on_top: false, replaygain: 'off', output_mode: 'directsound',
        wasapi_device: -1, merge_song_different_versions: true, ..._configStore,
      },
      appearance: {
        dark_mode: true, spectrum_columns: 64, fft_size: 1024,
        spectrum_style: 'log', theme: 'default', ..._configStore,
      },
      lyric: { fuzzy_match: false, show_translate: false },
      media_lib: { min_duration_secs: 30, auto_scan: true, media_dirs: [] },
      lastfm: { enabled: false, username: '', api_key: '' },
      general: { auto_download_lyric: false, auto_download_album_cover: true, check_update_when_start: false, minimize_to_notify_icon: false },
    };
  }
  function _handleConfigSet(key, value) { _configStore[key] = value; }

  // Register with native bridge — all events dispatched through onStateChange
  try {
    ohosBridge.onJSCallback('onStateChange');
  } catch(e) {
    console.error('[OHOS Adapter] register callback failed:', e);
  }

  window._ohosConnected = true;

  window._ohosShowDesktopLyric = function() { try { ohosBridge.showDesktopLyric(); } catch(e) { console.error(e); } };
  window._ohosHideDesktopLyric = function() { try { ohosBridge.hideDesktopLyric(); } catch(e) { console.error(e); } };
  window._ohosShowMiniMode = function() { try { ohosBridge.showMiniMode(); } catch(e) { console.error(e); } };
  window._ohosHideMiniMode = function() { try { ohosBridge.hideMiniMode(); } catch(e) { console.error(e); } };
  window._ohosPickAudioFiles = function() { try { ohosBridge.pickAudioFilesAsync(); } catch(e) { console.error(e); } };

  // Push current lyric line to DesktopLyric floating window
  window._ohosPushLyric = function(current, next) {
    try { ohosBridge.updateDesktopLyric(current || '', next || ''); } catch(e) { console.error(e); }
  };

  // Update DesktopLyric track info (title, artist)
  window._ohosPushTrackInfo = function(title, artist) {
    try { ohosBridge.updateDesktopTrack(title || '', artist || ''); } catch(e) { console.error(e); }
  };

  // Push cover art base64 to native (for DesktopLyric/MiniMode)
  window._ohosPushCover = function(base64) {
    try { ohosBridge.updateCoverBase64(base64 || ''); } catch(e) { console.error(e); }
  };

  console.log('[OHOS Adapter] initialized');
}