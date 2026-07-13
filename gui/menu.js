// Data-driven menu configuration
const MENU_CONFIG = [
  {
    id: 'file',
    label: '文件',
    icon: 'menu',
    items: [
      { id: 'open_file', label: '打开文件', icon: 'open_in_new', accelerator: 'Ctrl+O', action: () => showOpenFileDialog() },
      { id: 'open_folder', label: '打开文件夹', icon: 'folder', accelerator: 'Ctrl+F', action: () => showOpenFolderDialog() },
      { id: 'open_url', label: '打开 URL', icon: 'music_note', accelerator: 'Ctrl+U', action: () => showUrlDialog() },
      { type: 'separator' },
      { id: 'save_playlist', label: '保存播放列表', icon: 'download', accelerator: 'Ctrl+S', action: () => cmd('playlist save') },
      { id: 'load_playlist', label: '加载播放列表', icon: 'folder', accelerator: 'Ctrl+L', action: () => cmd('playlist load') },
      { type: 'separator' },
      { id: 'file_assoc', label: '文件关联', icon: 'info', action: () => cmd('file-assoc') },
      { type: 'separator' },
      { id: 'exit', label: '退出', icon: 'close', accelerator: 'Alt+F4', action: () => cmd('daemon stop') },
    ],
  },
  {
    id: 'playback',
    label: '播放控制',
    icon: 'play_arrow',
    items: [
      { id: 'play', label: '播放/暂停', icon: 'play_arrow', accelerator: 'Space', action: () => cmd('pause') },
      { id: 'stop', label: '停止', icon: 'stop', accelerator: 'Ctrl+S', action: () => cmd('stop') },
      { id: 'next', label: '下一曲', icon: 'skip_next', accelerator: 'Ctrl+Right', action: () => cmd('next') },
      { id: 'prev', label: '上一曲', icon: 'skip_previous', accelerator: 'Ctrl+Left', action: () => cmd('prev') },
      { type: 'separator' },
      {
        id: 'repeat_mode', label: '循环模式', icon: 'repeat',
        items: [
          { id: 'repeat_loop', label: '列表循环', icon: 'repeat', action: () => cmd('repeat loop'), checkable: true, checked: () => state.repeat === 'loop' },
          { id: 'repeat_order', label: '顺序播放', icon: 'playlist_play', action: () => cmd('repeat order'), checkable: true, checked: () => state.repeat === 'order' },
          { id: 'repeat_shuffle', label: '随机播放', icon: 'shuffle', action: () => cmd('repeat shuffle'), checkable: true, checked: () => state.repeat === 'shuffle' },
          { id: 'repeat_track', label: '单曲循环', icon: 'repeat_one', action: () => cmd('repeat track'), checkable: true, checked: () => state.repeat === 'track' },
        ],
      },
      { type: 'separator' },
      { id: 'volume_up', label: '音量 +5', icon: 'volume_up', accelerator: 'Up', action: () => cmd(`volume set ${Math.min((state.volume || 50) + 5, 100)}`) },
      { id: 'volume_down', label: '音量 -5', icon: 'volume_mute', accelerator: 'Down', action: () => cmd(`volume set ${Math.max((state.volume || 50) - 5, 0)}`) },
      { type: 'separator' },
      { id: 'fade', label: '淡入淡出', icon: 'speed', checkable: true, checked: () => settingsCache?.play?.fade_effect, action: () => api('POST', '/api/config', { key: 'play.fade_effect', value: settingsCache?.play?.fade_effect ? 'false' : 'true' }).then(() => { settingsCache = null; fetchSettings(); }) },
    ],
  },
  {
    id: 'playlist',
    label: '播放列表',
    icon: 'queue_music',
    items: [
      { id: 'pl_manager', label: '管理播放列表...', icon: 'queue_music', action: () => showPlaylistManager() },
      { type: 'separator' },
      { id: 'pl_save', label: '保存到文件', icon: 'download', accelerator: 'Ctrl+S', action: () => cmd('playlist save') },
      { id: 'pl_load', label: '从文件加载', icon: 'folder', accelerator: 'Ctrl+L', action: () => cmd('playlist load') },
      { type: 'separator' },
      { id: 'pl_new', label: '新建播放列表', icon: 'add', action: () => cmd('playlist new "New Playlist"') },
      { type: 'separator' },
      { id: 'pl_add_file', label: '添加文件', icon: 'add', accelerator: 'Ctrl+O', action: () => cmd('playlist add') },
      { id: 'pl_add_folder', label: '添加文件夹', icon: 'folder', action: () => cmd('playlist add folder') },
      { id: 'pl_add_url', label: '添加 URL', icon: 'music_note', action: () => cmd('playlist add url') },
      { type: 'separator' },
      { id: 'pl_remove_selected', label: '删除选中', icon: 'remove', action: () => cmd('playlist remove') },
      { id: 'pl_remove_duplicates', label: '删除重复项', icon: 'delete', action: () => cmd('playlist dedup') },
      { id: 'pl_clear', label: '清空列表', icon: 'delete', action: () => cmd('playlist clear') },
      { type: 'separator' },
      {
        id: 'pl_sort', label: '排序', icon: 'sort',
        items: [
          { id: 'sort_title', label: '按标题', action: () => cmd('playlist sort title') },
          { id: 'sort_artist', label: '按艺术家', action: () => cmd('playlist sort artist') },
          { id: 'sort_album', label: '按专辑', action: () => cmd('playlist sort album') },
          { id: 'sort_path', label: '按路径', action: () => cmd('playlist sort path') },
          { id: 'sort_duration', label: '按时长', action: () => cmd('playlist sort duration') },
        ],
      },
      { id: 'pl_shuffle', label: '打乱顺序', icon: 'shuffle', action: () => cmd('playlist shuffle') },
    ],
  },
  {
    id: 'lyric',
    label: '歌词',
    icon: 'lyrics',
    items: [
      { id: 'lyric_download', label: '下载歌词', icon: 'download', action: () => cmd('lyric download') },
      { id: 'lyric_batch_download', label: '批量下载', icon: 'download', action: () => cmd('lyric batch') },
      { type: 'separator' },
      { id: 'lyric_edit', label: '歌词编辑', icon: 'lyrics', action: () => showLyricEditor() },
      { id: 'lyric_relate', label: '关联歌词', icon: 'lyrics', action: () => cmd('lyric associate') },
      { id: 'lyric_unrelate', label: '取消关联', icon: 'remove', action: () => cmd('lyric unassociate') },
      { type: 'separator' },
      { id: 'lyric_show_translation', label: '显示翻译', icon: 'lyrics', checkable: true, checked: () => settingsCache?.lyric?.show_translate, action: () => api('POST', '/api/config', { key: 'lyric.show_translate', value: settingsCache?.lyric?.show_translate ? 'false' : 'true' }) },
      { id: 'lyric_fuzzy_match', label: '模糊匹配', icon: 'search', checkable: true, checked: () => settingsCache?.lyric?.fuzzy_match, action: () => api('POST', '/api/config', { key: 'lyric.fuzzy_match', value: settingsCache?.lyric?.fuzzy_match ? 'false' : 'true' }) },
    ],
  },
  {
    id: 'view',
    label: '视图',
    icon: 'more_vert',
    items: [
      { id: 'view_mini_mode', label: '迷你模式', icon: 'mini_mode', accelerator: 'Ctrl+Alt+M', action: () => sendTauriCommand('minimode') },
      { id: 'view_fullscreen', label: '全屏', icon: 'fullscreen', accelerator: 'F11', action: () => toggleFullscreen() },
      { id: 'view_menu_bar', label: '菜单栏', icon: 'menu', checkable: true, checked: true, action: () => toggleMenuBar() },
      { type: 'separator' },
      {
        id: 'view_theme', label: '主题', icon: 'dark_mode',
        items: Object.entries(THEMES).map(([k, v]) => ({
          id: `theme_${k}`,
          label: v.name,
          icon: k === 'default' ? 'dark_mode' : 'light_mode',
          checkable: true,
          checked: () => (localStorage.getItem('mp_theme') || 'default') === k,
          action: () => applyTheme(k),
        })),
      },
      { type: 'separator' },
      { id: 'view_spectrum', label: '频谱显示', icon: 'equalizer', checkable: true, checked: true, action: () => toggleSpectrum() },
      { id: 'view_cover', label: '专辑封面', icon: 'album', checkable: true, checked: true, action: () => toggleCover() },
      { id: 'view_lyrics', label: '歌词显示', icon: 'lyrics', checkable: true, checked: true, action: () => toggleLyrics() },
      { type: 'separator' },
      {
        id: 'view_layout', label: '布局模式', icon: 'mini_mode',
        items: [
          { id: 'layout_big', label: '完整界面 (BIG)', icon: 'fullscreen', checkable: true, checked: () => currentLayout === 'big', action: () => setLayout('big') },
          { id: 'layout_narrow', label: '窄界面 (NARROW)', icon: 'mini_mode', checkable: true, checked: () => currentLayout === 'narrow', action: () => setLayout('narrow') },
          { id: 'layout_small', label: '微型界面 (SMALL)', icon: 'more_vert', checkable: true, checked: () => currentLayout === 'small', action: () => setLayout('small') },
        ],
      },
    ],
  },
  {
    id: 'tools',
    label: '工具',
    icon: 'settings',
    items: [
      { id: 'tool_equalizer', label: '均衡器', icon: 'equalizer', action: () => openEqualizer() },
      { id: 'tool_format_convert', label: '格式转换', icon: 'music_note', action: () => showFormatConverter() },
      { id: 'tool_media_lib', label: '媒体库', icon: 'library_music', accelerator: 'Ctrl+M', action: () => switchTab('media') },
      { id: 'tool_settings', label: '设置', icon: 'settings', action: () => {
        if (typeof showSettingsDialog === 'function') showSettingsDialog();
        else switchTab('settings');
      } },
      { id: 'tool_track_info', label: '文件属性', icon: 'info', action: () => showFileProperties() },
      { id: 'tool_tag_editor', label: '标签编辑', icon: 'edit', action: () => showTagEditor(null) },
      { id: 'tool_batch_tag', label: '批量标签编辑', icon: 'edit_note', action: () => showBatchTagEditor() },
      { id: 'tool_musicbrainz', label: 'MusicBrainz 自动标签', icon: 'music_note', action: () => showMusicBrainzDialog() },
      { id: 'tool_theme_editor', label: '自定义主题编辑器', icon: 'palette', action: () => showThemeEditor() },
      { id: 'tool_hotkeys', label: '快捷键设置', icon: 'info', action: () => showHotkeyDialog() },
      { id: 'tool_sleep_timer', label: '定时关机', icon: 'schedule', action: () => showSleepTimerDialog() },
      { type: 'separator' },
      { id: 'tool_play_stats', label: '播放统计', icon: 'info', action: () => showPlayStats() },
      { id: 'tool_lastfm', label: 'Last.fm 设置', icon: 'about', action: () => showLastfmDialog() },
    ],
  },
  {
    id: 'help',
    label: '帮助',
    icon: 'help',
    items: [
      { id: 'help_about', label: '关于 HackMagic Music Player', icon: 'about', action: () => showAbout() },
      { id: 'help_check_update', label: '检查更新', icon: 'download', action: () => cmd('info check-update') },
      { type: 'separator' },
      { id: 'help_shortcuts', label: '快捷键参考', icon: 'info', accelerator: '?', action: () => showShortcuts() },
    ],
  },
];

// Context menus
const CONTEXT_MENUS = {
  main: [
    { id: 'ctx_play', label: '播放', icon: 'play_arrow', action: () => cmd('pause') },
    { id: 'ctx_add_to_playlist', label: '添加到播放列表', icon: 'queue_music',
      items: [
        { id: 'ctx_add_current', label: '添加到当前列表', action: () => cmd('playlist add current') },
      ],
    },
    { type: 'separator' },
    { id: 'ctx_properties', label: '属性', icon: 'info', action: () => showFileProperties() },
    { id: 'ctx_edit_tags', label: '编辑标签', icon: 'edit', action: () => showTagEditor(null) },
  ],
  playlist: [
    { id: 'ctx_pl_play', label: '播放', icon: 'play_arrow', action: () => { if (contextMenuIndex != null) cmd(`jump ${contextMenuIndex}`); } },
    { id: 'ctx_pl_next', label: '下一首播放', icon: 'playlist_play', action: () => { if (contextMenuIndex != null) cmd(`play --next "${state.playlist[contextMenuIndex].file_path}"`); } },
    { type: 'separator' },
    { id: 'ctx_pl_remove', label: '从列表删除', icon: 'remove', action: () => { if (contextMenuIndex != null) cmd(`playlist remove ${contextMenuIndex}`); } },
    { id: 'ctx_pl_delete', label: '从磁盘删除', icon: 'delete', action: () => { if (contextMenuIndex != null) cmd(`playlist delete ${contextMenuIndex}`); } },
    { type: 'separator' },
    { id: 'ctx_pl_favourite', label: '收藏', icon: 'favorite', action: () => { if (contextMenuIndex != null) cmd(`playlist favourite ${contextMenuIndex}`); } },
    {
      id: 'ctx_pl_rating', label: '评级', icon: 'star',
      items: [1,2,3,4,5].map(r => ({
        id: `ctx_rating_${r}`,
        label: '★'.repeat(r) + '☆'.repeat(5-r),
        action: () => { if (contextMenuIndex != null) cmd(`playlist rate ${contextMenuIndex} ${r}`); },
      })),
    },
    { type: 'separator', _cueSep: true },
    {
      id: 'ctx_pl_cue_jump', label: '跳转分轨', icon: 'playlist_play',
      _cueOnly: true,
      items: () => {
        const track = contextMenuIndex != null && typeof state !== 'undefined' && state.playlist ? state.playlist[contextMenuIndex] : null;
        if (!track?.is_cue) return [];
        const filePath = track.cue_file_path || track.file_path;
        const siblings = (state?.playlist || []).map((t, i) => ({ t, i }))
          .filter(({ t }) => t.is_cue && (t.cue_file_path || t.file_path) === filePath);
        return siblings.map(({ t, i }) => ({
          id: `ctx_cue_${i}`,
          label: `#${t.cue_track_number || '?'} ${t.title || (t.file_path?.split(/[/\\]/).pop())}`,
          action: () => cmd(`jump ${i}`),
        }));
      },
    },
    { type: 'separator' },
    { id: 'ctx_pl_properties', label: '属性', icon: 'info', action: () => showFileProperties() },
    { id: 'ctx_pl_edit_tags', label: '编辑标签', icon: 'edit', action: () => { if (contextMenuIndex != null) showTagEditor(contextMenuIndex); } },
    { type: 'separator' },
    { id: 'ctx_pl_copy_path', label: '复制路径', icon: 'content_copy', action: () => {
        if (contextMenuIndex != null && state?.playlist?.[contextMenuIndex]) {
          const p = state.playlist[contextMenuIndex].file_path;
          navigator.clipboard.writeText(p).catch(() => {});
        }
      }
    },
    { id: 'ctx_pl_explore', label: '打开文件位置', icon: 'folder', action: () => {
        if (contextMenuIndex != null && state?.playlist?.[contextMenuIndex]) {
          const p = state.playlist[contextMenuIndex].file_path;
          try { window.__TAURI__?.shell?.openPath?.(p.replace(/\/[^/]*$/, '')); } catch {}
          cmd(`open-location "${p}"`);
        }
      }
    },
  ],
};

// Menu bar state
let menuBarVisible = true;
let activeMenuId = null;
let contextMenuIndex = null;

function getIcon(iconName) {
  return ICONS[iconName] || ICONS.music_note;
}

function renderMenuItemHTML(item, depth = 0) {
  if (item.type === 'separator') return '<div class="menu-separator"></div>';

  const hasSub = item.items && item.items.length > 0;
  const isChecked = item.checkable && (typeof item.checked === 'function' ? item.checked() : item.checked);
  const iconHtml = getIcon(item.icon || 'music_note');
  const classes = 'menu-item' + (hasSub ? ' has-submenu' : '') + (isChecked ? ' checked' : '');

  let html = `<div class="${classes}" data-id="${item.id}" data-depth="${depth}">`;
  html += `<span class="mi-icon">${iconHtml}</span>`;
  html += `<span class="mi-label">${escHtml(item.label)}</span>`;
  if (isChecked) html += `<span class="mi-check">✓</span>`;
  if (item.accelerator) html += `<span class="mi-accel">${escHtml(item.accelerator)}</span>`;
  if (hasSub) html += `<span class="mi-arrow">▸</span>`;
  html += '</div>';

  if (hasSub) {
    html += '<div class="submenu">';
    for (const sub of item.items) {
      html += renderMenuItemHTML(sub, depth + 1);
    }
    html += '</div>';
  }

  return html;
}

function renderMenuBar() {
  const bar = document.getElementById('menu-bar');
  if (!bar) return;

  let html = '<div class="menu-bar-inner">';
  for (const menu of MENU_CONFIG) {
    const active = activeMenuId === menu.id ? ' active' : '';
    html += `<div class="menu-trigger${active}" data-menu-id="${menu.id}">`;
    html += `<span class="menu-trigger-icon">${getIcon(menu.icon)}</span>`;
    html += `<span class="menu-trigger-label">${escHtml(menu.label)}</span>`;
    html += '</div>';
  }
  html += '</div>';

  // Render a single dropdown container
  html += '<div id="menu-dropdown" class="menu-dropdown" style="display:none"></div>';
  bar.innerHTML = html;

  // Bind menu trigger events
  bar.querySelectorAll('.menu-trigger').forEach(el => {
    el.addEventListener('click', (e) => {
      e.stopPropagation();
      const menuId = el.dataset.menuId;
      toggleMenu(menuId, el);
    });
    el.addEventListener('mouseenter', () => {
      if (activeMenuId) {
        const menuId = el.dataset.menuId;
        showMenu(menuId, el);
      }
    });
  });
}

function toggleMenu(menuId, triggerEl) {
  if (activeMenuId === menuId) {
    hideMenu();
    return;
  }
  showMenu(menuId, triggerEl);
}

function showMenu(menuId, triggerEl) {
  activeMenuId = menuId;
  document.querySelectorAll('.menu-trigger').forEach(t => t.classList.toggle('active', t.dataset.menuId === menuId));

  const config = MENU_CONFIG.find(m => m.id === menuId);
  if (!config) return;

  const dropdown = document.getElementById('menu-dropdown');
  let html = '';
  for (const item of config.items) {
    html += renderMenuItemHTML(item);
  }
  dropdown.innerHTML = html;
  dropdown.style.display = 'block';

  // Position the dropdown
  const barRect = document.getElementById('menu-bar').getBoundingClientRect();
  const triggerRect = triggerEl.getBoundingClientRect();
  const dropdownRect = dropdown.getBoundingClientRect();

  let left = triggerRect.left - barRect.left;
  const maxRight = barRect.width - dropdownRect.width;
  if (left > maxRight) left = Math.max(0, maxRight);
  if (left < 0) left = 0;

  dropdown.style.left = left + 'px';
  dropdown.style.top = (triggerRect.bottom - barRect.top) + 'px';

  // Bind menu item events
  bindMenuEvents(dropdown);

  // Submenu positioning on hover
  dropdown.querySelectorAll('.has-submenu').forEach(el => {
    el.addEventListener('mouseenter', () => {
      const sub = el.parentElement.querySelector('.submenu');
      if (!sub) return;
      sub.style.display = 'block';
      const subRect = sub.getBoundingClientRect();
      const ddRect = dropdown.getBoundingClientRect();
      const overflowRight = subRect.right - ddRect.right;
      if (overflowRight > 0) {
        sub.style.left = 'auto';
        sub.style.right = '100%';
      } else {
        sub.style.left = '100%';
        sub.style.right = 'auto';
      }
    });
    el.addEventListener('mouseleave', () => {
      const sub = el.parentElement.querySelector('.submenu');
      if (sub) sub.style.display = 'none';
    });
  });
}

function bindMenuEvents(container) {
  container.querySelectorAll('.menu-item:not(.has-submenu)').forEach(el => {
    el.addEventListener('click', (e) => {
      e.stopPropagation();
      const menuId = el.dataset.id;
      // Find action from config
      const action = findMenuAction(menuId);
      if (action) action();
      hideMenu();
    });
  });
}

function findMenuAction(id) {
  for (const menu of MENU_CONFIG) {
    const found = findInItems(menu.items, id);
    if (found) return found.action;
  }
  for (const ctx of Object.values(CONTEXT_MENUS)) {
    const found = findInItems(ctx, id);
    if (found) return found.action;
  }
  return null;
}

function findInItems(items, id) {
  for (const item of items) {
    if (item.id === id) return item;
    if (item.items) {
      const found = findInItems(item.items, id);
      if (found) return found;
    }
  }
  return null;
}

function hideMenu() {
  activeMenuId = null;
  document.querySelectorAll('.menu-trigger').forEach(t => t.classList.remove('active'));
  const dropdown = document.getElementById('menu-dropdown');
  if (dropdown) {
    dropdown.style.display = 'none';
    dropdown.querySelectorAll('.submenu').forEach(s => s.style.display = '');
    dropdown.querySelectorAll('.submenu').forEach(s => {
      s.style.left = '';
      s.style.right = '';
    });
  }
  hideContextMenu();
}

// Context menu
function showContextMenu(items, x, y) {
  hideMenu();
  const dropdown = document.getElementById('menu-dropdown');
  let html = '';
  const track = contextMenuIndex != null && typeof state !== 'undefined' && state.playlist ? state.playlist[contextMenuIndex] : null;
  const isCueTrack = track?.is_cue === true;

  function renderItems(list) {
    let out = '';
    let prevWasSep = false;
    for (const item of list) {
      // Filter CUE-only items
      if (item._cueOnly && !isCueTrack) continue;
      if (item._cueSep && !isCueTrack) continue;
      if (item.type === 'separator') { if (prevWasSep) continue; prevWasSep = true; }
      else prevWasSep = false;
      // Resolve dynamic submenu items
      const resolved = { ...item };
      if (typeof resolved.items === 'function') resolved.items = resolved.items();
      out += renderMenuItemHTML(resolved);
    }
    return out;
  }

  html = renderItems(items);
  dropdown.innerHTML = html;
  dropdown.style.display = 'block';
  dropdown.style.left = x + 'px';
  dropdown.style.top = y + 'px';
  dropdown.classList.add('context-menu');
  bindMenuEvents(dropdown);

  document.addEventListener('click', hideContextMenu, { once: true });
}

function hideContextMenu() {
  const dropdown = document.getElementById('menu-dropdown');
  if (dropdown) {
    dropdown.classList.remove('context-menu');
    dropdown.style.display = 'none';
  }
}

function toggleMenuBar() {
  menuBarVisible = !menuBarVisible;
  const bar = document.getElementById('menu-bar');
  if (bar) bar.style.display = menuBarVisible ? 'flex' : 'none';
}

// Dialog launchers (connected via app.js)
function openEqualizer() {
  showDialog({
    title: '均衡器 & 音效',
    width: '620px',
    body: \`
      <div id="eq-main" style="display:flex;flex-direction:column;gap:12px">
        <div id="eq-visual-wrap" style="position:relative;height:90px;border-radius:6px;background:rgba(0,0,0,0.3);border:1px solid var(--border);overflow:hidden">
          <canvas id="eq-curve" width="560" height="90" style="width:100%;height:90px;display:block"></canvas>
          <div style="position:absolute;bottom:2px;left:0;right:0;display:flex;justify-content:space-around;padding:0 8px;font-size:8px;color:var(--text3);pointer-events:none">
            <span>31</span><span>62</span><span>125</span><span>250</span><span>500</span><span>1k</span><span>2k</span><span>4k</span><span>8k</span><span>16k</span>
          </div>
        </div>
        <div style="display:flex;align-items:center;gap:8px">
          <div class="toggle on" id="dlg-eq-enable" style="flex-shrink:0"></div>
          <label style="font-size:12px;color:var(--text2);flex-shrink:0">EQ</label>
          <select id="dlg-eq-preset" style="flex:1;padding:4px 8px;border:1px solid var(--border);border-radius:4px;background:var(--bg2);color:var(--text);font-size:12px">
            <option value="none">None</option>
            <option value="classical">Classical</option><option value="pop">Pop</option>
            <option value="jazz">Jazz</option><option value="rock">Rock</option>
            <option value="soft">Soft</option><option value="bass">Bass</option>
            <option value="nobass">No Bass</option><option value="nohigh">No High</option>
          </select>
          <button id="dlg-eq-save" style="padding:3px 8px;font-size:11px;border:1px solid var(--border);border-radius:4px;background:var(--bg2);color:var(--text);cursor:pointer">Save</button>
          <button id="dlg-eq-reset" style="padding:3px 8px;font-size:11px;border:1px solid var(--border);border-radius:4px;background:var(--bg2);color:var(--text);cursor:pointer">Reset</button>
        </div>
        <div id="dlg-eq-bands" style="display:grid;grid-template-columns:repeat(10,1fr);gap:3px;padding:4px 0"></div>
        <hr style="border:none;border-top:1px solid var(--border);margin:4px 0">
        <div style="display:flex;gap:16px;flex-wrap:wrap">
          <div style="flex:1;min-width:180px">
            <div style="display:flex;align-items:center;gap:6px;margin-bottom:6px">
              <div class="toggle" id="dlg-reverb-enable" style="flex-shrink:0"></div>
              <label style="font-size:12px;color:var(--text2)">Reverb</label>
            </div>
            <div id="dlg-reverb-controls" style="display:none">
              <div style="display:flex;align-items:center;gap:6px;margin-bottom:4px">
                <label style="font-size:11px;color:var(--text3);width:30px">Mix</label>
                <input type="range" id="dlg-reverb-mix" min="0" max="100" step="1" value="50" style="flex:1">
                <span id="dlg-reverb-mix-val" style="font-size:11px;color:var(--text2);width:36px;text-align:right">50</span>
              </div>
              <div style="display:flex;align-items:center;gap:6px">
                <label style="font-size:11px;color:var(--text3);width:30px">Time</label>
                <input type="range" id="dlg-reverb-time" min="10" max="3000" step="10" value="100" style="flex:1">
                <span id="dlg-reverb-time-val" style="font-size:11px;color:var(--text2);width:36px;text-align:right">100</span>
              </div>
            </div>
          </div>
          <div style="flex:1;min-width:140px">
            <div style="display:flex;align-items:center;gap:6px;margin-bottom:6px">
              <label style="font-size:12px;color:var(--text2)">Speed</label>
              <input type="range" id="dlg-speed" min="0.5" max="2" step="0.05" value="1" style="flex:1">
              <span id="dlg-speed-val" style="font-size:11px;color:var(--text2);width:40px;text-align:right">1.0x</span>
            </div>
            <div style="display:flex;align-items:center;gap:6px">
              <label style="font-size:12px;color:var(--text2)">Pitch</label>
              <input type="range" id="dlg-pitch" min="-12" max="12" step="1" value="0" style="flex:1">
              <span id="dlg-pitch-val" style="font-size:11px;color:var(--text2);width:40px;text-align:right">0</span>
            </div>
          </div>
        </div>
      </div>
    \`,
    onOpen: async () => {
      const freqs = ['31','62','125','250','500','1k','2k','4k','8k','16k'];
      const bandContainer = document.getElementById('dlg-eq-bands');
      const canvas = document.getElementById('eq-curve');
      const ctx = canvas.getContext('2d');
      let eqData = { enabled: true, bands: Array(10).fill(0) };
      try { const r = await api('GET', '/api/eq'); if (r) eqData = r; } catch {}
      bandContainer.innerHTML = eqData.bands.map((g, i) =>
        \`<div style="display:flex;flex-direction:column;align-items:center;gap:2px">
          <span style="font-size:9px;color:var(--text3)">\${freqs[i]}</span>
          <input type="range" orient="vertical" min="-15" max="15" value="\${g}" step="1" data-band="\${i}"
            style="writing-mode:vertical-lr;direction:rtl;width:24px;height:80px;accent-color:var(--accent)">
          <span class="dlg-eq-val" style="font-size:10px;color:var(--text2);font-family:var(--font-mono)">\${g>0?'+':''}\${g}</span>
        </div>\`
      ).join('');
      let revData = { enabled: false, mix: 50, time: 100 };
      try { const r = await api('GET', '/api/reverb'); if (r) revData = r; } catch {}
      const revToggle = document.getElementById('dlg-reverb-enable');
      revToggle.classList.toggle('on', revData.enabled);
      document.getElementById('dlg-reverb-controls').style.display = revData.enabled ? 'block' : 'none';
      document.getElementById('dlg-reverb-mix').value = revData.mix;
      document.getElementById('dlg-reverb-mix-val').textContent = revData.mix;
      document.getElementById('dlg-reverb-time').value = revData.time;
      document.getElementById('dlg-reverb-time-val').textContent = revData.time;
      try {
        const s = await api('GET', '/api/status');
        if (s) {
          document.getElementById('dlg-speed').value = s.speed || 1;
          document.getElementById('dlg-speed-val').textContent = (s.speed || 1).toFixed(2) + 'x';
          document.getElementById('dlg-pitch').value = s.pitch || 0;
          document.getElementById('dlg-pitch-val').textContent = (s.pitch > 0 ? '+' : '') + (s.pitch || 0);
        }
      } catch {}
      function drawEqCurve(bands) {
        const w = canvas.width, h = canvas.height;
        ctx.clearRect(0, 0, w, h);
        const pad = 10, ch = h - pad * 2, cw = w - pad * 2, mid = pad + ch / 2;
        ctx.strokeStyle = 'rgba(255,255,255,0.06)'; ctx.lineWidth = 1;
        for (let db = -15; db <= 15; db += 5) {
          const y = mid - (db / 15) * (ch / 2);
          ctx.beginPath(); ctx.moveTo(pad, y); ctx.lineTo(w - pad, y); ctx.stroke();
          ctx.fillStyle = 'rgba(255,255,255,0.15)'; ctx.font = '8px sans-serif'; ctx.textAlign = 'right';
          ctx.fillText(db + 'dB', pad - 2, y + 3);
        }
        ctx.strokeStyle = 'rgba(255,255,255,0.15)'; ctx.lineWidth = 1; ctx.setLineDash([3, 3]);
        ctx.beginPath(); ctx.moveTo(pad, mid); ctx.lineTo(w - pad, mid); ctx.stroke(); ctx.setLineDash([]);
        ctx.beginPath(); ctx.moveTo(pad, mid);
        bands.forEach((g, i) => { const x = pad + (i / (bands.length - 1)) * cw; ctx.lineTo(x, mid - (g / 15) * (ch / 2)); });
        ctx.lineTo(w - pad, mid); ctx.closePath();
        const fg = ctx.createLinearGradient(0, 0, 0, h); fg.addColorStop(0, 'rgba(233,69,96,0.2)'); fg.addColorStop(1, 'rgba(233,69,96,0.02)');
        ctx.fillStyle = fg; ctx.fill();
        ctx.beginPath();
        bands.forEach((g, i) => { const x = pad + (i / (bands.length - 1)) * cw; const y = mid - (g / 15) * (ch / 2); if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y); });
        ctx.strokeStyle = '#e94560'; ctx.lineWidth = 2; ctx.stroke();
        bands.forEach((g, i) => { const x = pad + (i / (bands.length - 1)) * cw; const y = mid - (g / 15) * (ch / 2);
          ctx.beginPath(); ctx.arc(x, y, 4, 0, Math.PI * 2); ctx.fillStyle = '#e94560'; ctx.fill();
          ctx.beginPath(); ctx.arc(x, y, 2, 0, Math.PI * 2); ctx.fillStyle = '#fff'; ctx.fill();
        });
      }
      drawEqCurve(eqData.bands);
      document.getElementById('dlg-eq-enable').addEventListener('click', function () {
        const on = this.classList.toggle('on'); api('POST', '/api/command', { command: on ? 'eq enable' : 'eq disable' });
        bandContainer.style.display = on ? 'grid' : 'none';
      });
      bandContainer.style.display = eqData.enabled ? 'grid' : 'none';
      bandContainer.querySelectorAll('input[type="range"]').forEach(el => {
        el.addEventListener('input', () => {
          el.nextElementSibling.textContent = parseInt(el.value) > 0 ? '+' + parseInt(el.value) : String(parseInt(el.value));
          drawEqCurve([...bandContainer.querySelectorAll('input[type="range"]')].map(s => parseInt(s.value)));
        });
        el.addEventListener('change', () => { api('POST', '/api/command', { command: \`eq set \${el.dataset.band} \${el.value}\` }); });
      });
      document.getElementById('dlg-eq-preset').addEventListener('change', function () {
        if (this.value !== 'none') {
          api('POST', '/api/command', { command: \`eq preset \${this.value}\` });
          setTimeout(async () => {
            const r = await api('GET', '/api/eq');
            if (r) {
              const sliders = bandContainer.querySelectorAll('input[type="range"]');
              r.bands.forEach((g, i) => { if (sliders[i]) { sliders[i].value = g; sliders[i].nextElementSibling.textContent = g > 0 ? '+' + g : String(g); } });
              drawEqCurve(r.bands); eqData = r;
            }
            this.value = 'none';
          }, 200);
        }
      });
      document.getElementById('dlg-eq-reset').addEventListener('click', () => {
        api('POST', '/api/command', { command: 'eq reset' });
        bandContainer.querySelectorAll('input[type="range"]').forEach(el => { el.value = 0; el.nextElementSibling.textContent = '0'; });
        drawEqCurve(Array(10).fill(0));
      });
      document.getElementById('dlg-eq-save').addEventListener('click', () => {
        const name = prompt('Save current EQ as preset:');
        if (!name) return;
        const vals = [...bandContainer.querySelectorAll('input[type="range"]')].map(s => parseInt(s.value));
        const presets = typeof loadUserEqPresets === 'function' ? loadUserEqPresets() : {};
        presets[name] = vals;
        if (typeof saveUserEqPresets === 'function') saveUserEqPresets(presets);
      });
      revToggle.addEventListener('click', function () {
        const on = this.classList.toggle('on'); document.getElementById('dlg-reverb-controls').style.display = on ? 'block' : 'none';
        api('POST', '/api/command', { command: on ? 'reverb enable' : 'reverb disable' });
      });
      document.getElementById('dlg-reverb-mix').addEventListener('input', function () { document.getElementById('dlg-reverb-mix-val').textContent = this.value; });
      document.getElementById('dlg-reverb-mix').addEventListener('change', function () { api('POST', '/api/command', { command: \`reverb mix \${this.value}\` }); });
      document.getElementById('dlg-reverb-time').addEventListener('input', function () { document.getElementById('dlg-reverb-time-val').textContent = this.value; });
      document.getElementById('dlg-reverb-time').addEventListener('change', function () { api('POST', '/api/command', { command: \`reverb time \${this.value}\` }); });
      document.getElementById('dlg-speed').addEventListener('input', function () { document.getElementById('dlg-speed-val').textContent = parseFloat(this.value).toFixed(2) + 'x'; });
      document.getElementById('dlg-speed').addEventListener('change', function () { api('POST', '/api/command', { command: \`speed set \${parseFloat(this.value).toFixed(2)}\` }); });
      document.getElementById('dlg-pitch').addEventListener('input', function () { const v = parseInt(this.value); document.getElementById('dlg-pitch-val').textContent = (v > 0 ? '+' : '') + v; });
      document.getElementById('dlg-pitch').addEventListener('change', function () { api('POST', '/api/command', { command: \`pitch set \${parseInt(this.value)}\` }); });
    },
  });
}

function switchTab(tabName) {
  const btn = document.querySelector(`.tab-btn[data-tab="${tabName}"]`);
  if (btn) btn.click();
}

function sendTauriCommand(cmd) {
  if (typeof electronAPI !== 'undefined' && electronAPI.windowToggleMini) {
    electronAPI.windowToggleMini();
  } else if (window.__TAURI__) {
    // Mini mode toggle via Tauri - send the command via the existing IPC
    const { invoke } = window.__TAURI__.core;
    invoke(cmd).catch(() => {});
  }
}

function toggleSpectrum() {
  const section = document.getElementById('spectrum-section');
  if (section) section.style.display = section.style.display === 'none' ? '' : 'none';
}

function toggleCover() {
  const art = document.getElementById('album-art');
  if (art) art.style.display = art.style.display === 'none' ? '' : 'none';
}

function toggleLyrics() {
  const section = document.getElementById('lyric-section');
  if (section) section.style.display = section.style.display === 'none' ? '' : 'none';
}

function showAbout() {
  if (typeof showAboutDialog === 'function') showAboutDialog();
}
function showShortcuts() {
  if (typeof showShortcutsDialog === 'function') showShortcutsDialog();
}

// Initialize menu system
function initMenuSystem() {
  renderMenuBar();

  // Click outside to close menu
  document.addEventListener('click', (e) => {
    if (activeMenuId && !e.target.closest('#menu-bar')) {
      hideMenu();
    }
  });

  // Keyboard shortcut to toggle menu bar
  document.addEventListener('keydown', (e) => {
    if (e.altKey && e.code === 'KeyM') {
      e.preventDefault();
      toggleMenuBar();
    }
    if (e.altKey && !e.ctrlKey && !e.metaKey) {
      // Alt+letter opens menu
      const letter = e.key.toLowerCase();
      const idx = 'fplvth'.indexOf(letter); // File, Playback, pLaylist, View, Tools, Help
      if (idx >= 0 && idx < MENU_CONFIG.length) {
        e.preventDefault();
        const triggers = document.querySelectorAll('.menu-trigger');
        if (triggers[idx]) toggleMenu(MENU_CONFIG[idx].id, triggers[idx]);
      }
    }
    if (e.code === 'Escape') {
      hideMenu();
    }
  });
}

// Export for use in app.js
window.ContextMenu = { showContextMenu, hideContextMenu };