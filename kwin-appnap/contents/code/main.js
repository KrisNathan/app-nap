// CONSTANTS
const SERVICE = "dev.appnap.AppNap";
const PATH = "/dev/appnap/AppNap";
const IFACE = "dev.appnap.AppNap1";

// HELPERS
function debug(message) {
  console.info(`[app-nap] ${message}`);
}

// UTILS
function isPidUsable(window) {
  const pid = +window.pid;
  return typeof pid === "number" && pid > 0;
}

function isTrackable(window) {
  debug(`
    window: id=${window.internalId.toString()} pid=${window.pid}
    specialWindow=${window.specialWindow} popupWindow=${window.popupWindow}
    normalWindow=${window.normalWindow}
    skipTaskbar=${window.skipTaskbar} skipPager=${window.skipPager} skipSwitcher=${window.skipSwitcher}
    usablePid=${isPidUsable(window)}
  `);
  return (
    !!window &&
    !window.specialWindow &&
    !window.popupWindow &&
    window.normalWindow &&
    !window.skipTaskbar &&
    !window.skipPager &&
    !window.skipSwitcher &&
    isPidUsable(window)
  );
}

// MAIN
function main() {
  debug("INIT");
  debug("AMOGUS2");

  workspace.windowAdded.connect(function(window) {
    if (!isTrackable(window)) {
      return;
    }
    const id = window.internalId.toString();
    const pid = window.pid;
    debug(`window added: id=${id} pid=${pid}`);

    callDBus(SERVICE, PATH, IFACE, "AddWindow", id, pid);

    window.minimizedChanged.connect(function() {
      const minimized = window.minimized;
      debug(
        `window minimized changed: id=${id} pid=${pid} minimized=${minimized}`,
      );
      callDBus(SERVICE, PATH, IFACE, "MinimizedChanged", id, pid, minimized);
    });
  });
  // if proc is minimized, frozen
  // this signal will be emitted only when the window is unminimized first
  workspace.windowRemoved.connect(function(window) {
    if (!isTrackable(window)) {
      return;
    }
    const id = window.internalId.toString();
    const pid = window.pid;
    debug(`window removed: id=${id} pid=${pid}`);
    callDBus(SERVICE, PATH, IFACE, "RemoveWindow", id, pid);
  });

  // I'm currently planning for implementing "load all existing window on init so that all windows are tracked"
  for (let w of workspace.windowList()) {
    debug(`INIT: ${w.resourceName} ${w.internalId.toString()} isTrackable: ${isTrackable(w)}`)
  }
}

main();
