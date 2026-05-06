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

const LAYOUT = {
  mascot: { left: 244, top: 191, width: 112, height: 121 },
  tray: { left: 48, top: 56, width: 276, height: 131 },
};
const DRAG_THRESHOLD_PX = 4;
const VELOCITY_SAMPLE_WINDOW_MS = 100;
const MIN_RELEASE_VELOCITY = 320;
const MAX_RELEASE_VELOCITY = 1600;
const SPRITE_COLUMNS = 8;
const SPRITE_ROWS = 9;
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
const ICONS = {
  "check-circle": `<svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="10" fill="currentColor"/><path d="M8 12.5l2.5 2.5L16 9.5" fill="none" stroke="var(--token-bg-primary)" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"/></svg>`,
  chevron: `<svg width="16" height="16" viewBox="0 0 20 21" fill="none" aria-hidden="true"><path d="M15.2793 7.71101C15.539 7.45131 15.961 7.45131 16.2207 7.71101C16.4804 7.97071 16.4804 8.39272 16.2207 8.65242L10.4707 14.4024C10.211 14.6621 9.78902 14.6621 9.52932 14.4024L3.77932 8.65242L3.69436 8.54792C3.52385 8.28979 3.55205 7.93828 3.77932 7.71101C4.00659 7.48374 4.3581 7.45554 4.61623 7.62605L4.72073 7.71101L10 12.9903L15.2793 7.71101Z" fill="currentColor" stroke="currentColor" stroke-width=".6"/></svg>`,
  clock: `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" aria-hidden="true"><circle cx="12" cy="12" r="8" stroke="currentColor" stroke-width="2"/><path d="M12 7v5l3 2" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg>`,
  spinner: `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" aria-hidden="true"><path opacity=".3" d="M18 12C18 8.68629 15.3137 6 12 6C8.68629 6 6 8.68629 6 12C6 15.3137 8.68629 18 12 18C15.3137 18 18 15.3137 18 12ZM20 12C20 16.4183 16.4183 20 12 20C7.58172 20 4 16.4183 4 12C4 7.58172 7.58172 4 12 4C16.4183 4 20 7.58172 20 12Z" fill="currentColor"/><path d="M12 4C16.4183 4 20 7.58172 20 12C20 16.4183 16.4183 20 12 20C7.58172 20 4 16.4183 4 12H6C6 15.3137 8.68629 18 12 18C15.3137 18 18 15.3137 18 12C18 8.68629 15.3137 6 12 6V4Z" fill="currentColor"/></svg>`,
  warning: `<svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3 2 21h20L12 3Z" fill="currentColor"/><path d="M12 9v5" stroke="var(--token-bg-primary)" stroke-width="2" stroke-linecap="round"/><circle cx="12" cy="17" r="1.2" fill="var(--token-bg-primary)"/></svg>`,
};

const overlay = document.getElementById("overlay");
const contentFrame = document.getElementById("contentFrame");
const tray = document.getElementById("tray");
const trayInner = document.getElementById("trayInner");
const mascotFrame = document.getElementById("mascotFrame");
const mascotButton = document.getElementById("mascotButton");
const avatar = document.getElementById("avatar");
const badge = document.getElementById("badge");
const badgeContent = document.getElementById("badgeContent");
const title = document.getElementById("title");
const body = document.getElementById("body");
const statusIcon = document.getElementById("statusIcon");

let animationTimer = null;
let baseMascotState = "idle";
let currentAnimationState = null;
let collapsed = false;
let lastSnapshot = null;
let pointerDrag = null;
let suppressNextClick = false;

applyLayout();
avatar.style.backgroundImage = `url("${ASSETS.codex}")`;
setAnimation("idle");

function applyLayout() {
  Object.assign(mascotFrame.style, pxRect(LAYOUT.mascot));
  Object.assign(tray.style, pxRect(LAYOUT.tray));
}

function pxRect(rect) {
  return {
    height: `${rect.height}px`,
    left: `${rect.left}px`,
    top: `${rect.top}px`,
    width: `${rect.width}px`,
  };
}

function row(rowIndex, count, frameDurationMs, lastFrameDurationMs) {
  return Array.from({ length: count }, (_value, columnIndex) => ({
    rowIndex,
    columnIndex,
    frameDurationMs:
      columnIndex === count - 1 ? lastFrameDurationMs : frameDurationMs,
  }));
}

function framePosition(frame) {
  const x = (frame.columnIndex / (SPRITE_COLUMNS - 1)) * 100;
  const y = (frame.rowIndex / (SPRITE_ROWS - 1)) * 100;
  return `${x}% ${y}%`;
}

function animationSequence(state) {
  const frames = FRAME_SETS[state] || FRAME_SETS.idle;
  if (state === "idle") {
    return { frames: SLOW_IDLE_FRAMES, loopStartIndex: 0 };
  }
  const intro = [...frames, ...frames, ...frames];
  return { frames: [...intro, ...SLOW_IDLE_FRAMES], loopStartIndex: intro.length };
}

function setAnimation(state) {
  const nextState = FRAME_SETS[state] ? state : "idle";
  if (currentAnimationState === nextState) {
    return;
  }
  currentAnimationState = nextState;
  window.clearTimeout(animationTimer);

  const sequence = animationSequence(nextState);
  let index = 0;
  function tick() {
    const frame = sequence.frames[index];
    avatar.style.backgroundPosition = framePosition(frame);
    animationTimer = window.setTimeout(() => {
      index += 1;
      if (index >= sequence.frames.length) {
        index = sequence.loopStartIndex;
      }
      tick();
    }, frame.frameDurationMs);
  }
  tick();
}

function petAsset(pet) {
  return ASSETS[pet] || ASSETS.codex;
}

function mapSnapshotState(snapshot) {
  if (snapshot == null) {
    return {
      badgeBackground: "var(--token-activity-bar-badge-background)",
      badgeForeground: "var(--token-activity-bar-badge-foreground)",
      fallbackBody: "Info",
      iconClass: "clock",
      mascotState: "idle",
    };
  }
  if (snapshot.state === "running") {
    return {
      badgeBackground: "var(--token-activity-bar-badge-background)",
      badgeForeground: "var(--token-activity-bar-badge-foreground)",
      fallbackBody: "Thinking",
      iconClass: "spinner",
      mascotState: "running",
    };
  }
  if (snapshot.state === "waiting") {
    return {
      badgeBackground: "var(--token-editor-warning-foreground)",
      badgeForeground: "var(--token-bg-primary)",
      fallbackBody: "Needs input",
      iconClass: "clock",
      mascotState: "waiting",
    };
  }
  if (snapshot.state === "failed") {
    return {
      badgeBackground: "var(--token-error-foreground)",
      badgeForeground: "var(--token-bg-primary)",
      fallbackBody: "Blocked",
      iconClass: "warning",
      mascotState: "failed",
    };
  }
  if (snapshot.state === "review") {
    return {
      badgeBackground: "var(--token-charts-green)",
      badgeForeground: "var(--token-bg-primary)",
      fallbackBody: "Ready",
      iconClass: "check-circle",
      mascotState: "review",
    };
  }
  return {
    badgeBackground: "var(--token-activity-bar-badge-background)",
    badgeForeground: "var(--token-activity-bar-badge-foreground)",
    fallbackBody: "",
    iconClass: "clock",
    mascotState: "idle",
  };
}

function applySnapshot(snapshot) {
  if (snapshot == null) {
    return;
  }

  lastSnapshot = snapshot;
  const mapped = mapSnapshotState(snapshot);
  const notificationCount = Number(snapshot.notification_count || 0);
  const hasNotification = snapshot.state !== "idle";
  const trayVisible = hasNotification && !collapsed;

  avatar.style.backgroundImage = `url("${petAsset(snapshot.pet)}")`;
  title.textContent = snapshot.title || "Codex";
  body.textContent = snapshot.subtitle || snapshot.detail || mapped.fallbackBody;
  statusIcon.className = `status-icon ${mapped.iconClass}`;
  statusIcon.innerHTML = ICONS[mapped.iconClass] || ICONS.clock;
  badge.hidden = !hasNotification;
  badgeContent.textContent = "";
  if (collapsed) {
    badgeContent.textContent = String(Math.max(1, notificationCount));
  } else {
    badgeContent.innerHTML = ICONS.chevron;
  }
  badge.classList.toggle("is-icon-only", !collapsed);
  badge.classList.toggle("is-count", collapsed);
  if (collapsed) {
    badge.style.backgroundColor = mapped.badgeBackground;
    badge.style.color = mapped.badgeForeground;
  } else {
    badge.style.removeProperty("background-color");
    badge.style.removeProperty("color");
  }
  tray.setAttribute("aria-hidden", trayVisible ? "false" : "true");
  tray.style.pointerEvents = trayVisible ? "" : "none";
  overlay.className = `overlay is-${snapshot.state}${collapsed ? " is-collapsed" : ""}`;
  trayInner.style.transformOrigin = "bottom right";

  baseMascotState = mapped.mascotState;
  if (pointerDrag?.transientState != null) {
    setAnimation(pointerDrag.transientState);
  } else {
    setAnimation(baseMascotState);
  }
}

function toggleTray() {
  collapsed = !collapsed;
  applySnapshot(lastSnapshot);
}

function pointerSample(event) {
  return {
    screenX: event.screenX,
    screenY: event.screenY,
    timeMs: event.timeStamp,
  };
}

function trimSamples(samples) {
  const latest = samples.at(-1);
  if (latest == null) {
    return samples;
  }
  return samples.filter(
    (sample) => latest.timeMs - sample.timeMs <= VELOCITY_SAMPLE_WINDOW_MS,
  );
}

function releaseVelocity(drag, sample) {
  if (!drag.hasMoved) {
    return null;
  }
  const samples = trimSamples([...drag.samples, sample]);
  const latest = samples.at(-1);
  if (latest == null) {
    return null;
  }
  const previous = samples.find((entry) => latest.timeMs - entry.timeMs > 16);
  if (previous == null) {
    return null;
  }
  const seconds = (latest.timeMs - previous.timeMs) / 1000;
  if (seconds <= 0) {
    return null;
  }
  const velocity = {
    x: (latest.screenX - previous.screenX) / seconds,
    y: (latest.screenY - previous.screenY) / seconds,
  };
  const speed = Math.hypot(velocity.x, velocity.y);
  if (speed < MIN_RELEASE_VELOCITY) {
    return null;
  }
  if (speed <= MAX_RELEASE_VELOCITY) {
    return velocity;
  }
  const scale = MAX_RELEASE_VELOCITY / speed;
  return { x: velocity.x * scale, y: velocity.y * scale };
}

function endDrag(event) {
  const drag = pointerDrag;
  if (drag == null || drag.pointerId !== event.pointerId) {
    return;
  }
  pointerDrag = null;
  contentFrame.releasePointerCapture?.(event.pointerId);
  setAnimation(baseMascotState);
  suppressNextClick = drag.hasMoved;
  window.codexPets.dragEnd({
    shouldHide: false,
    velocity: releaseVelocity(drag, pointerSample(event)),
  });
}

contentFrame.addEventListener("pointerdown", (event) => {
  if (
    event.button !== 0 ||
    !(event.target instanceof Element) ||
    event.target.closest("[data-avatar-mascot='true']") == null ||
    event.target.closest(".no-drag") != null
  ) {
    return;
  }

  event.preventDefault();
  contentFrame.setPointerCapture?.(event.pointerId);
  pointerDrag = {
    hasMoved: false,
    pointerId: event.pointerId,
    samples: [pointerSample(event)],
    screenX: event.screenX,
    screenY: event.screenY,
    transientState: null,
  };
  window.codexPets.dragStart({
    pointerWindowX: event.clientX,
    pointerWindowY: event.clientY,
  });
});

contentFrame.addEventListener("pointermove", (event) => {
  const drag = pointerDrag;
  if (drag == null || drag.pointerId !== event.pointerId) {
    return;
  }
  const sample = pointerSample(event);
  drag.samples = trimSamples([...drag.samples, sample]);
  const deltaX = sample.screenX - drag.screenX;
  const deltaY = sample.screenY - drag.screenY;
  if (Math.abs(deltaX) < DRAG_THRESHOLD_PX && Math.abs(deltaY) < DRAG_THRESHOLD_PX) {
    return;
  }
  drag.hasMoved = true;
  drag.screenX = sample.screenX;
  drag.screenY = sample.screenY;
  drag.transientState =
    deltaX >= DRAG_THRESHOLD_PX
      ? "running-right"
      : deltaX <= -DRAG_THRESHOLD_PX
        ? "running-left"
        : drag.transientState;
  setAnimation(drag.transientState || baseMascotState);
  window.codexPets.dragMove({
    screenX: event.screenX,
    screenY: event.screenY,
  });
});

contentFrame.addEventListener("pointerup", (event) => {
  endDrag(event);
});

contentFrame.addEventListener("pointercancel", (event) => {
  endDrag(event);
});

contentFrame.addEventListener("lostpointercapture", (event) => {
  if (pointerDrag?.pointerId === event.pointerId) {
    pointerDrag = null;
    setAnimation(baseMascotState);
    window.codexPets.dragEnd({ shouldHide: false, velocity: null });
  }
});

mascotButton.addEventListener("pointerenter", () => {
  if (pointerDrag == null) {
    setAnimation("jumping");
  }
});

mascotButton.addEventListener("pointerleave", () => {
  if (pointerDrag == null) {
    setAnimation(baseMascotState);
  }
});

mascotButton.addEventListener("contextmenu", (event) => {
  event.preventDefault();
  window.codexPets.hide();
});

badge.addEventListener("click", (event) => {
  event.stopPropagation();
  toggleTray();
});

contentFrame.addEventListener("click", (event) => {
  if (
    !(event.target instanceof Element) ||
    event.target.closest(".no-drag") != null
  ) {
    return;
  }
  if (suppressNextClick) {
    suppressNextClick = false;
    return;
  }
  window.codexPets.openTerminal();
});

window.codexPets.onCommand((command) => {
  if (command.type === "show") {
    collapsed = false;
    if (command.pet != null) {
      avatar.style.backgroundImage = `url("${petAsset(command.pet)}")`;
    }
    applySnapshot(command.snapshot || lastSnapshot);
    return;
  }
  if (command.type === "snapshot") {
    applySnapshot(command.snapshot);
  }
});
