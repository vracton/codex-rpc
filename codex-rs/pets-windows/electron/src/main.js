const { app, BrowserWindow, ipcMain } = require("electron");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const readline = require("node:readline");

let window = null;
let lastSnapshot = null;
let currentPet = "codex";
let isVisible = false;
const logPath = path.join(os.tmpdir(), "codex-pets-electron.log");
const commandFilePath = argValue("--command-file");

function log(message) {
  fs.appendFileSync(logPath, `${new Date().toISOString()} ${message}\n`);
}

function argValue(name) {
  const index = process.argv.indexOf(name);
  if (index < 0) {
    return null;
  }
  return process.argv[index + 1] || null;
}

function createWindow() {
  log("creating transparent overlay window");
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
      preload: path.join(__dirname, "preload.js"),
      contextIsolation: true,
      nodeIntegration: false,
      backgroundThrottling: false,
    },
  });

  window.setAlwaysOnTop(true, "screen-saver");
  window.loadFile(path.join(__dirname, "index.html"));
  window.once("ready-to-show", () => {
    log("window ready");
    process.stdout.write(JSON.stringify({ type: "ready" }) + "\n");
  });
  window.on("closed", () => {
    log("window closed");
    window = null;
  });
}

function show() {
  log("show command");
  if (window == null) {
    log("show ignored because window is unavailable");
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
  log("hide command");
  isVisible = false;
  if (window != null) {
    window.hide();
  }
  process.stdout.write(JSON.stringify({ type: "hidden" }) + "\n");
}

function sendSnapshot(snapshot) {
  log(`snapshot command: ${JSON.stringify(snapshot)}`);
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
      log("shutdown command");
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
  rl.on("close", () => {
    log("stdin closed");
  });
}

function attachCommandFile(filePath) {
  let offset = 0;
  fs.closeSync(fs.openSync(filePath, "a"));
  log(`watching command file: ${filePath}`);

  setInterval(() => {
    let stat;
    try {
      stat = fs.statSync(filePath);
    } catch (error) {
      log(`failed to stat command file: ${error}`);
      return;
    }
    if (stat.size <= offset) {
      return;
    }

    let buffer;
    try {
      const fd = fs.openSync(filePath, "r");
      buffer = Buffer.alloc(stat.size - offset);
      fs.readSync(fd, buffer, 0, buffer.length, offset);
      fs.closeSync(fd);
    } catch (error) {
      log(`failed to read command file: ${error}`);
      return;
    }
    offset = stat.size;

    for (const rawLine of buffer.toString("utf8").split(/\r?\n/)) {
      const line = rawLine.replace(/^\uFEFF/, "");
      if (line.trim() === "") {
        continue;
      }
      try {
        handleCommand(JSON.parse(line));
      } catch (error) {
        log(`failed to handle command file line: ${error}`);
      }
    }
  }, 100);
}

app.disableHardwareAcceleration();
app.whenReady().then(() => {
  log("electron app ready");
  createWindow();
  if (commandFilePath == null) {
    attachStdin();
  } else {
    attachCommandFile(commandFilePath);
  }
});

ipcMain.on("hide-pet", hide);
