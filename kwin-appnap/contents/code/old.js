const SERVICE = "dev.appnap.AppNap";
const PATH = "/dev/appnap/AppNap";
const IFACE = "dev.appnap.AppNap1";

var trackedWindows = Object.create(null);
var fallbackWindowId = 0;

function debug(message) {
  console.warn(`[app-nap] ${message}`);
}

function windowIdFor(window) {
  if (window.internalId !== undefined && window.internalId !== null) {
    return String(window.internalId);
  }
  if (
    window.__appNapFallbackId === undefined ||
    window.__appNapFallbackId === null
  ) {
    fallbackWindowId += 1;
    window.__appNapFallbackId = "fallback-" + fallbackWindowId;
  }
  return window.__appNapFallbackId;
}

function usablePid(window) {
  const pid = window.pid;
  return typeof pid === "number" && pid > 0 ? pid : null;
}

function trackable(window) {
  return (
    !!window &&
    !window.specialWindow &&
    !window.popupWindow &&
    usablePid(window) !== null
  );
}

function stateFor(window) {
  const id = windowIdFor(window);
  return trackedWindows[id] || null;
}

function sendWindowState(window) {
  if (!trackable(window)) {
    return;
  }
  const state = stateFor(window);
  if (!state) {
    return;
  }
  debug(
    "UpdateWindow id=" +
      state.id +
      " pid=" +
      state.pid +
      " active=" +
      !!window.active,
  );
  callDBus(
    SERVICE,
    PATH,
    IFACE,
    "UpdateWindow",
    state.id,
    state.pid,
    !!window.active,
  );
}

function removeWindow(window) {
  if (!trackable(window)) {
    return;
  }
  const state = stateFor(window);
  if (!state) {
    return;
  }
  delete trackedWindows[state.id];
  debug(`RemoveWindow id=${state.id}`);
  callDBus(SERVICE, PATH, IFACE, "RemoveWindow", state.id);
}

function trackWindow(window) {
  if (!trackable(window)) {
    return;
  }

  const pid = usablePid(window);
  if (pid === null) {
    return;
  }

  const id = windowIdFor(window);
  if (trackedWindows[id]) {
    debug("Refreshing tracked window id=" + id + " pid=" + pid);
    sendWindowState(window);
    return;
  }

  trackedWindows[id] = { id: id, pid: pid };
  debug("Tracking window id=" + id + " pid=" + pid);

  window.activeChanged.connect(function () {
    debug("activeChanged id=" + id + " active=" + !!window.active);
    sendWindowState(window);
  });

  sendWindowState(window);
}

function init() {
  debug("Initializing script");
  const windows = workspace.stackingOrder;
  debug("Initial stackingOrder size=" + windows.length);
  for (let i = 0; i < windows.length; i += 1) {
    trackWindow(windows[i]);
  }

  workspace.windowAdded.connect(function (window) {
    debug("windowAdded");
    trackWindow(window);
  });
  workspace.windowRemoved.connect(function (window) {
    debug("windowRemoved");
    removeWindow(window);
  });
}

// init();
