const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("codexPets", {
  onCommand(callback) {
    ipcRenderer.on("pet-command", (_event, command) => callback(command));
  },
  hide() {
    ipcRenderer.send("hide-pet");
  },
});
