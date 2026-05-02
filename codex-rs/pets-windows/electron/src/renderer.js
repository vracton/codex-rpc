const ASSETS = {
  bsod: "../assets/bsod.webp",
  codex: "../assets/codex.webp",
  dewey: "../assets/dewey.webp",
  fireball: "../assets/fireball.webp",
  "null-signal": "../assets/null-signal.webp",
  rocky: "../assets/rocky.webp",
  seedy: "../assets/seedy.webp",
  stacky: "../assets/stacky.webp",
};

const IDLE_FRAMES = [
  { rowIndex: 0, columnIndex: 0, frameDurationMs: 280 },
  { rowIndex: 0, columnIndex: 1, frameDurationMs: 110 },
  { rowIndex: 0, columnIndex: 2, frameDurationMs: 110 },
  { rowIndex: 0, columnIndex: 3, frameDurationMs: 140 },
  { rowIndex: 0, columnIndex: 4, frameDurationMs: 140 },
  { rowIndex: 0, columnIndex: 5, frameDurationMs: 320 },
];

const SLOW_IDLE_FRAMES = IDLE_FRAMES.map((frame) => ({
  ...frame,
  frameDurationMs: frame.frameDurationMs * 6,
}));

const FRAME_SETS = {
  failed: row(5, 8, 140, 240),
  idle: IDLE_FRAMES,
  jumping: row(4, 5, 140, 280),
  review: row(8, 6, 150, 280),
  running: row(7, 6, 120, 220),
  "running-left": row(2, 8, 120, 220),
  "running-right": row(1, 8, 120, 220),
  waving: row(3, 4, 140, 280),
  waiting: row(6, 6, 150, 260),
};

const overlay = document.getElementById("overlay");
const avatar = document.getElementById("avatar");
const badge = document.getElementById("badge");
const title = document.getElementById("title");
const body = document.getElementById("body");
const statusIcon = document.getElementById("statusIcon");
const collapseButton = document.getElementById("collapseButton");
const mascotButton = document.getElementById("mascotButton");

let animationTimer = null;
let collapsed = false;
let currentState = "idle";

function row(rowIndex, count, frameDurationMs, lastFrameDurationMs) {
  return Array.from({ length: count }, (_value, columnIndex) => ({
    rowIndex,
    columnIndex,
    frameDurationMs:
      columnIndex === count - 1 ? lastFrameDurationMs : frameDurationMs,
  }));
}

function framePosition(frame) {
  return `${(frame.columnIndex / 7) * 100}% ${(frame.rowIndex / 8) * 100}%`;
}

function setAnimation(state) {
  window.clearTimeout(animationTimer);
  currentState = state in FRAME_SETS ? state : "idle";
  const baseFrames = FRAME_SETS[currentState];
  const sequence =
    currentState === "idle"
      ? SLOW_IDLE_FRAMES
      : [...baseFrames, ...baseFrames, ...baseFrames, ...SLOW_IDLE_FRAMES];
  const loopStartIndex = currentState === "idle" ? 0 : baseFrames.length * 3;
  let index = 0;

  function tick() {
    const frame = sequence[index];
    avatar.style.backgroundPosition = framePosition(frame);
    animationTimer = window.setTimeout(() => {
      index += 1;
      if (index >= sequence.length) {
        index = loopStartIndex;
      }
      tick();
    }, frame.frameDurationMs);
  }

  tick();
}

function petAsset(pet) {
  return ASSETS[pet] || ASSETS.codex;
}

function mapSnapshotState(state) {
  switch (state) {
    case "running":
      return {
        mascotState: "running",
        overlayClass: "is-running",
        iconClass: "spinner",
        fallbackBody: "Thinking",
        showBadge: false,
      };
    case "waiting":
      return {
        mascotState: "waiting",
        overlayClass: "is-waiting",
        iconClass: "spinner",
        fallbackBody: "Needs input",
        showBadge: false,
      };
    case "review":
      return {
        mascotState: "review",
        overlayClass: "is-review",
        iconClass: "check",
        fallbackBody: "Ready",
        showBadge: true,
      };
    case "failed":
      return {
        mascotState: "failed",
        overlayClass: "is-failed",
        iconClass: "failed",
        fallbackBody: "Blocked",
        showBadge: true,
      };
    case "idle":
    default:
      return {
        mascotState: "idle",
        overlayClass: "is-idle",
        iconClass: "spinner",
        fallbackBody: "",
        showBadge: false,
      };
  }
}

function applySnapshot(snapshot) {
  if (snapshot == null) {
    return;
  }

  const mapped = mapSnapshotState(snapshot.state);
  avatar.style.backgroundImage = `url("${petAsset(snapshot.pet)}")`;
  title.textContent = snapshot.title || "Codex";
  body.textContent = snapshot.subtitle || snapshot.detail || mapped.fallbackBody;
  badge.hidden = !mapped.showBadge;
  const notificationCount = Number(snapshot.notification_count || 0);
  badge.textContent = String(Math.max(1, notificationCount));
  badge.hidden = notificationCount === 0 && !mapped.showBadge;
  statusIcon.className = `status-icon ${mapped.iconClass}`;
  overlay.className = `overlay ${mapped.overlayClass}${
    collapsed ? " is-collapsed" : ""
  }`;

  if (currentState !== mapped.mascotState) {
    setAnimation(mapped.mascotState);
  }
}

collapseButton.addEventListener("click", () => {
  collapsed = !collapsed;
  overlay.classList.toggle("is-collapsed", collapsed);
});

mascotButton.addEventListener("contextmenu", (event) => {
  event.preventDefault();
  window.codexPets.hide();
});

window.codexPets.onCommand((command) => {
  if (command.type === "show") {
    collapsed = false;
    overlay.classList.remove("is-collapsed");
    avatar.style.backgroundImage = `url("${petAsset(command.pet)}")`;
    applySnapshot(command.snapshot);
    return;
  }
  if (command.type === "snapshot") {
    applySnapshot(command.snapshot);
  }
});

avatar.style.backgroundImage = `url("${ASSETS.codex}")`;
setAnimation("idle");
