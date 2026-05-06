const { app, BrowserWindow, ipcMain, screen } = require("electron");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const readline = require("node:readline");

let window = null;
let lastSnapshot = null;
let currentPet = "codex";
let isVisible = false;
let dragState = null;
let inertiaTimer = null;
const logPath = path.join(os.tmpdir(), "codex-pets-electron.log");
const commandFilePath = argValue("--command-file");
const WINDOW_SIZE = { width: 356, height: 320 };
const DEFAULT_MARGIN = 24;

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
    width: WINDOW_SIZE.width,
    height: WINDOW_SIZE.height,
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
    stopInertia();
    window = null;
  });
}

function statePath() {
  return path.join(app.getPath("userData"), "window-state.json");
}

function readSavedBounds() {
  try {
    const parsed = JSON.parse(fs.readFileSync(statePath(), "utf8"));
    if (
      Number.isFinite(parsed.x) &&
      Number.isFinite(parsed.y) &&
      Number.isFinite(parsed.width) &&
      Number.isFinite(parsed.height)
    ) {
      return {
        x: parsed.x,
        y: parsed.y,
        width: WINDOW_SIZE.width,
        height: WINDOW_SIZE.height,
      };
    }
  } catch {
    return null;
  }
  return null;
}

function writeSavedBounds(bounds) {
  try {
    fs.mkdirSync(path.dirname(statePath()), { recursive: true });
    fs.writeFileSync(
      statePath(),
      JSON.stringify({
        x: Math.round(bounds.x),
        y: Math.round(bounds.y),
        width: WINDOW_SIZE.width,
        height: WINDOW_SIZE.height,
      }),
      "utf8",
    );
  } catch (error) {
    log(`failed to persist window bounds: ${error}`);
  }
}

function bottomLeftBounds() {
  const cursor = screen.getCursorScreenPoint();
  const workArea = screen.getDisplayNearestPoint(cursor).workArea;
  return {
    x: workArea.x + DEFAULT_MARGIN,
    y: workArea.y + workArea.height - WINDOW_SIZE.height - DEFAULT_MARGIN,
    width: WINDOW_SIZE.width,
    height: WINDOW_SIZE.height,
  };
}

function clampBounds(bounds) {
  const display =
    screen.getDisplayMatching(bounds).workArea || screen.getPrimaryDisplay().workArea;
  const minX = display.x - WINDOW_SIZE.width + 56;
  const minY = display.y - WINDOW_SIZE.height + 56;
  const maxX = display.x + display.width - 56;
  const maxY = display.y + display.height - 56;
  return {
    x: Math.min(Math.max(Math.round(bounds.x), minX), maxX),
    y: Math.min(Math.max(Math.round(bounds.y), minY), maxY),
    width: WINDOW_SIZE.width,
    height: WINDOW_SIZE.height,
  };
}

function placeInitialWindow() {
  if (window == null) {
    return;
  }
  window.setBounds(clampBounds(bottomLeftBounds()), false);
}

function stopInertia() {
  if (inertiaTimer != null) {
    clearInterval(inertiaTimer);
    inertiaTimer = null;
  }
}

function startInertia(velocity) {
  if (window == null || velocity == null) {
    return;
  }
  stopInertia();
  let vx = velocity.x;
  let vy = velocity.y;
  let last = Date.now();
  inertiaTimer = setInterval(() => {
    if (window == null) {
      stopInertia();
      return;
    }
    const now = Date.now();
    const seconds = Math.min(0.05, (now - last) / 1000);
    last = now;
    const bounds = window.getBounds();
    const next = clampBounds({
      ...bounds,
      x: bounds.x + vx * seconds,
      y: bounds.y + vy * seconds,
    });
    window.setBounds(next, false);
    vx *= 0.86;
    vy *= 0.86;
    if (Math.hypot(vx, vy) < 24) {
      stopInertia();
      writeSavedBounds(window.getBounds());
    }
  }, 16);
}

function show() {
  log("show command");
  if (window == null) {
    log("show ignored because window is unavailable");
    return;
  }
  if (isVisible) {
    hide();
    return;
  }
  if (!isVisible) {
    placeInitialWindow();
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
ipcMain.on("open-terminal", () => {
  window?.blur();
});
ipcMain.on("pet-drag-start", (_event, payload) => {
  if (window == null) {
    return;
  }
  stopInertia();
  dragState = {
    pointerWindowX: Number(payload?.pointerWindowX || 0),
    pointerWindowY: Number(payload?.pointerWindowY || 0),
  };
});

ipcMain.on("pet-drag-move", (_event, payload) => {
  if (window == null || dragState == null) {
    return;
  }
  const screenX = Number(payload?.screenX);
  const screenY = Number(payload?.screenY);
  if (!Number.isFinite(screenX) || !Number.isFinite(screenY)) {
    return;
  }
  window.setBounds(
    clampBounds({
      x: screenX - dragState.pointerWindowX,
      y: screenY - dragState.pointerWindowY,
      width: WINDOW_SIZE.width,
      height: WINDOW_SIZE.height,
    }),
    false,
  );
});

ipcMain.on("pet-drag-end", (_event, payload) => {
  if (window == null) {
    return;
  }
  dragState = null;
  if (payload?.shouldHide) {
    hide();
    return;
  }
  const velocity = payload?.velocity;
  if (
    velocity != null &&
    Number.isFinite(velocity.x) &&
    Number.isFinite(velocity.y)
  ) {
    startInertia(velocity);
  } else {
    writeSavedBounds(window.getBounds());
  }
});
