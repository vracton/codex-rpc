const { app, BrowserWindow, ipcMain } = require("electron");
const readline = require("node:readline");

let window = null;
let lastSnapshot = null;
let currentPet = "codex";
let isVisible = false;

function createWindow() {
  window = new BrowserWindow({
    width: 356,
    height: 320,
    transparent: true,
    frame: false,
    resizable: false,
    skipTaskbar: true,
    alwaysOnTop: true,
    hasShadow: false,
    show: false,
    backgroundColor: "#00000000",
    webPreferences: {
      preload: require("node:path").join(__dirname, "preload.js"),
      contextIsolation: true,
      nodeIntegration: false,
      backgroundThrottling: false,
    },
  });

  window.setAlwaysOnTop(true, "screen-saver");
  window.loadFile(require("node:path").join(__dirname, "index.html"));
  window.once("ready-to-show", () => {
    process.stdout.write(JSON.stringify({ type: "ready" }) + "\n");
  });
  window.on("closed", () => {
    window = null;
  });
}

function show() {
  if (window == null) {
    return;
  }
  if (!isVisible) {
    const display = require("electron").screen.getPrimaryDisplay().workArea;
    window.setPosition(
      display.x + display.width - 356 - 24,
      display.y + display.height - 320 - 24,
      false,
    );
  }
  isVisible = true;
  window.showInactive();
  window.webContents.send("pet-command", {
    type: "show",
    pet: currentPet,
    snapshot: lastSnapshot,
  });
}

function hide() {
  isVisible = false;
  if (window != null) {
    window.hide();
  }
  process.stdout.write(JSON.stringify({ type: "hidden" }) + "\n");
}

function sendSnapshot(snapshot) {
  lastSnapshot = snapshot;
  currentPet = snapshot.pet || currentPet;
  if (window != null) {
    window.webContents.send("pet-command", {
      type: "snapshot",
      snapshot,
    });
  }
}

function handleCommand(command) {
  switch (command.type) {
    case "show":
      currentPet = command.pet || currentPet;
      show();
      break;
    case "hide":
      hide();
      break;
    case "set_snapshot":
      sendSnapshot(command.snapshot);
      break;
    case "shutdown":
      app.quit();
      break;
    default:
      process.stdout.write(
        JSON.stringify({
          type: "error",
          message: `unknown pets command: ${command.type}`,
        }) + "\n",
      );
      break;
  }
}

function attachStdin() {
  const rl = readline.createInterface({
    input: process.stdin,
    crlfDelay: Infinity,
  });
  rl.on("line", (line) => {
    if (line.trim() === "") {
      return;
    }
    try {
      handleCommand(JSON.parse(line));
    } catch (error) {
      process.stdout.write(
        JSON.stringify({
          type: "error",
          message: error instanceof Error ? error.message : String(error),
        }) + "\n",
      );
    }
  });
  rl.on("close", () => app.quit());
}

app.disableHardwareAcceleration();
app.whenReady().then(() => {
  createWindow();
  attachStdin();
});

ipcMain.on("hide-pet", hide);
