const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('electronAPI', {
  windowMinimize: () => ipcRenderer.invoke('window-minimize'),
  windowMaximize: () => ipcRenderer.invoke('window-maximize'),
  windowClose: () => ipcRenderer.invoke('window-close'),
  windowToggleMini: () => ipcRenderer.invoke('window-toggle-mini'),
  getApiUrl: () => ipcRenderer.invoke('get-api-url'),
  getWsUrl: () => ipcRenderer.invoke('get-ws-url'),
  openExternal: (url) => ipcRenderer.invoke('open-external', url),
  showNotification: (data) => ipcRenderer.invoke('show-notification', data),
  onBackendCommand: (callback) => {
    ipcRenderer.on('backend-command', (event, command) => {
      callback(command);
    });
  }
});