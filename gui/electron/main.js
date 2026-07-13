const { app, BrowserWindow, Menu, Tray, ipcMain, shell, nativeImage } = require('electron');
const path = require('path');
const { spawn } = require('child_process');
const fs = require('fs');

let mainWindow = null;
let miniWindow = null;
let tray = null;
let backendProcess = null;

function createMainWindow() {
  mainWindow = new BrowserWindow({
    width: 960,
    height: 680,
    minWidth: 760,
    minHeight: 480,
    frame: false,
    resizable: true,
    center: true,
    icon: path.join(__dirname, '../src-tauri/icons/icon.ico'),
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      preload: path.join(__dirname, 'preload.js')
    }
  });

  mainWindow.loadFile('index.html');

  mainWindow.on('closed', () => {
    mainWindow = null;
  });

  mainWindow.on('minimize', (e) => {
    e.preventDefault();
    mainWindow.hide();
  });
}

function createMiniWindow() {
  if (miniWindow) {
    miniWindow.show();
    miniWindow.focus();
    return;
  }

  miniWindow = new BrowserWindow({
    width: 400,
    height: 100,
    frame: false,
    resizable: false,
    alwaysOnTop: true,
    skipTaskbar: true,
    transparent: true,
    webPreferences: {
      nodeIntegration: false,
      contextIsolation: true,
      preload: path.join(__dirname, 'preload.js')
    }
  });

  miniWindow.loadFile('mini.html');

  miniWindow.on('closed', () => {
    miniWindow = null;
  });
}

function createTray() {
  const iconPath = path.join(__dirname, '../src-tauri/icons/icon.ico');
  tray = new Tray(iconPath);

  const contextMenu = Menu.buildFromTemplate([
    {
      label: 'Play/Pause',
      accelerator: 'Ctrl+Alt+P',
      click: () => sendCommand('play')
    },
    {
      label: 'Next',
      accelerator: 'Ctrl+Alt+N',
      click: () => sendCommand('next')
    },
    {
      label: 'Previous',
      accelerator: 'Ctrl+Alt+B',
      click: () => sendCommand('prev')
    },
    {
      label: 'Stop',
      click: () => sendCommand('stop')
    },
    { type: 'separator' },
    {
      label: 'Mini Mode',
      accelerator: 'Ctrl+Alt+M',
      click: () => toggleMiniMode()
    },
    { type: 'separator' },
    {
      label: 'Quit',
      accelerator: 'Ctrl+Alt+Q',
      click: () => quitApp()
    }
  ]);

  tray.setToolTip('HackMagic Music Player');
  tray.setContextMenu(contextMenu);

  tray.on('click', () => {
    if (mainWindow) {
      mainWindow.show();
      mainWindow.focus();
    }
  });
}

function sendCommand(command) {
  if (mainWindow && mainWindow.webContents) {
    mainWindow.webContents.send('backend-command', command);
  }
}

function toggleMiniMode() {
  if (miniWindow) {
    miniWindow.close();
  } else {
    createMiniWindow();
  }
}

function quitApp() {
  if (backendProcess) {
    backendProcess.kill();
    backendProcess = null;
  }
  app.quit();
}

app.whenReady().then(() => {
  createMainWindow();
  createTray();

  ipcMain.handle('window-minimize', () => {
    mainWindow?.minimize();
  });

  ipcMain.handle('window-maximize', () => {
    if (mainWindow?.isMaximized()) {
      mainWindow.unmaximize();
    } else {
      mainWindow?.maximize();
    }
  });

  ipcMain.handle('window-close', () => {
    mainWindow?.hide();
  });

  ipcMain.handle('window-toggle-mini', () => {
    toggleMiniMode();
  });

  ipcMain.handle('get-api-url', () => {
    return 'http://127.0.0.1:10280';
  });

  ipcMain.handle('get-ws-url', () => {
    return 'ws://127.0.0.1:10280/ws';
  });

  ipcMain.handle('open-external', (event, url) => {
    shell.openExternal(url);
  });

  ipcMain.handle('show-notification', (event, { title, body, icon }) => {
    const notification = new Notification({
      title,
      body,
      icon: icon || path.join(__dirname, '../src-tauri/icons/icon.ico')
    });
    notification.show();
  });

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createMainWindow();
    }
  });
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

app.on('before-quit', () => {
  if (backendProcess) {
    backendProcess.kill();
  }
});