const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("codexPets", {
  onCommand(callback) {
    ipcRenderer.on("pet-command", (_event, command) => callback(command));
  },
  dragEnd(payload) {
    ipcRenderer.send("pet-drag-end", payload);
  },
  dragMove(payload) {
    ipcRenderer.send("pet-drag-move", payload);
  },
  dragStart(payload) {
    ipcRenderer.send("pet-drag-start", payload);
  },
  hide() {
    ipcRenderer.send("hide-pet");
  },
});
