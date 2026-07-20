# Rough Flow

1. KWin script watches trackable windows and forwards state to the Rust daemon
   over session D-Bus (`AddWindow`, `MinimizedChanged`, `ActiveChanged`,
   `RemoveWindow`). The script does not decide nap policy.
2. The daemon groups windows by PID, resolves related app cgroup(s), and
   reconciles that PID into one tier:
   - **Performance**: at least one window is active.
   - **Background**: no active window, but at least one window is unminimized
     (or all windows are minimized but media/inhibit keeps the app awake).
   - **Nap**: all windows are minimized, and neither media playback nor an
     idle inhibitor applies.
3. Each tier runs its configured action list against the app's cgroup(s)
   (e.g. `systemd-cpu-weight`, `systemd-cpu-quota`, `signal`, `systemd-freeze`,
   `ecore`). On tier change the daemon reverts the previous tier, then applies
   the next. It only reverts what it previously applied.
4. A minimized/frozen window can still be unminimized or focused; KWin still
   emits state changes, and the daemon resumes via the normal reconcile path.
5. Failed tier transitions are left as the last successfully applied tier and
   retried on the next window-state event (not in a busy loop).

## Grouping Windows Into an "App"

Two layers:

1. **Window state key = KWin window PID**  
   The daemon's `HashMap` is keyed by the PID KWin reports for each window.
   Multiple windows that share that PID share one `AppState` (active/minimized
   map + cached cgroups + current tier).

2. **Action / media / inhibit scope = related app cgroup(s)**  
   On first `AddWindow` for a PID, the daemon resolves which systemd app units
   belong to that launch tree and caches those cgroup paths on `AppState`.
   Tier actions, MPRIS matching, and idle-inhibitor matching all use that set.

### PID climbing

From the window PID, walk `/proc/<pid>/status` `PPid` upward until `comm` is
`systemd` (or pid ≤ 1). Collect every ancestor along that chain. This matters
for Chromium-style apps that split across units: the window may live in a
self-created `app-org.chromium.Chromium-*.scope` while children stay in the
desktop-launch `app-*.service`. Climbing to the user systemd instance sees
both sides of the tree.

### App cgroups

For each ancestor, read `/proc/<pid>/cgroup`, trim the hierarchy id, and keep
paths that look like app units under `app.slice/`:

- last path segment starts with `app-`
- ends with `.scope` or `.service`
- path contains `/app.slice/`

Deduplicate. That list is the app's related cgroups (often one unit; sometimes
two for Chromium splits).

### Live procs

When applying signal/ecore (or matching media by PID), expand each cached
cgroup via `/sys/fs/cgroup<cgroup>/cgroup.procs` and union the PIDs. Proc
membership is read live at action/check time, not frozen into `AppState` —
only the cgroup path list is cached from first registration.

## Handling Media Playback

MPRIS over session D-Bus: list `org.mpris.MediaPlayer2.*` players, read
`PlaybackStatus`, map the player name back to a Unix PID via
`GetConnectionUnixProcessID`, then match that PID into the app's cgroup set.
If the app is playing, reconcile keeps it in Background instead of Nap.

```sh
$ qdbus | grep org.mpris.MediaPlayer2
 org.mpris.MediaPlayer2.brave.instance2
 org.mpris.MediaPlayer2.firefox.instance_1_602
```

```sh
$ qdbus org.mpris.MediaPlayer2.firefox.instance_1_602 /org/mpris/MediaPlayer2 org.freedesktop.DBus.Properties.Get org.mpris.MediaPlayer2.Player PlaybackStatus
Playing
```

Brave may need gdbus instead of qdbus for Properties.Get:

```sh
gdbus call --session --dest org.mpris.MediaPlayer2.brave.instance2 --object-path /org/mpris/MediaPlayer2 --method org.freedesktop.DBus.Properties.Get org.mpris.MediaPlayer2.Player PlaybackStatus
```

```sh
$ qdbus org.freedesktop.DBus /org/freedesktop/DBus org.freedesktop.DBus.GetConnectionUnixProcessID org.mpris.MediaPlayer2.firefox.instance_1_602
67229
```

## Handling Idle Inhibition

KDE PowerDevil `org.kde.Solid.PowerManagement.PolicyAgent` → `ListInhibitions`
returns `aas` (`[who, why]` lists). Match `who` (desktop/Flatpak app ID) as a
substring of `/proc/<pid>/cgroup` (the systemd unit name embeds the app ID).
Do not match against `/proc/<pid>/comm`. An active matching inhibitor is a hard
do-not-nap signal (Background instead of Nap; thaw on next reconcile if already
napped). Unmatched inhibitors are ignored.

## Handling Apps with Multiple Windows

Aggregate by PID (e.g. Chromium PWAs):

- Nap only when every tracked window for that PID is minimized (and keep-awake
  signals are clear).
- Move to Performance as soon as any window for that PID becomes active.
- Background when nothing is active but something is still unminimized.

## DBus Monitor

```sh
dbus-monitor "interface='dev.appnap.AppNap1'"
```

## DBus Call

- service: `dev.appnap.AppNap`
- path: `/dev/appnap/AppNap`
- interface: `dev.appnap.AppNap1`

Methods:

- `AddWindow(s window_id, i pid)`
- `RemoveWindow(s window_id, i pid)`
- `MinimizedChanged(s window_id, i pid, b minimized)`
- `ActiveChanged(s window_id, i pid, b active)`

```sh
busctl --user call dev.appnap.AppNap /dev/appnap/AppNap dev.appnap.AppNap1 AddWindow si "random-uuid" 1234
busctl --user call dev.appnap.AppNap /dev/appnap/AppNap dev.appnap.AppNap1 MinimizedChanged sib "random-uuid" 1234 true
busctl --user call dev.appnap.AppNap /dev/appnap/AppNap dev.appnap.AppNap1 ActiveChanged sib "random-uuid" 1234 false
```

KWin Scripting Console:

```sh
plasma-interactiveconsole --kwin
```

```sh
journalctl -f QT_CATEGORY=js QT_CATEGORY=kwin_scripting
```

```js
const SERVICE = "dev.appnap.AppNap";
const PATH = "/dev/appnap/AppNap";
const IFACE = "dev.appnap.AppNap1";
const id = "random-uuid";
const pid = 1234;
callDBus(SERVICE, PATH, IFACE, "AddWindow", id, pid);
callDBus(SERVICE, PATH, IFACE, "MinimizedChanged", id, pid, true);
callDBus(SERVICE, PATH, IFACE, "ActiveChanged", id, pid, false);
```

## Tier Actions / systemd

Configure actions in `~/.config/app-nap/app-nap.toml` (see `example/app-nap.toml`).
Defaults use CPU weight for Performance/Background and CPU quota (+ optional
`ecore`) for Nap. `signal` / `systemd-freeze` remain available when stronger
stop behavior is wanted.

Resolve the app unit from the process cgroup:

```sh
ps -o cgroup <PID>
```

Example:

```
CGROUP
0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-flatpak-com.brave.Browser-1760258369.scope
```

Manual examples (quota is a hard cap; weight is a CPU scheduling weight):

```sh
systemctl --user set-property app-flatpak-com.brave.Browser-1760258369.scope CPUQuota=10%
systemctl --user set-property app-flatpak-com.brave.Browser-1760258369.scope CPUQuota=
systemctl --user set-property app-flatpak-com.brave.Browser-1760258369.scope CPUWeight=1
systemctl --user set-property app-flatpak-com.brave.Browser-1760258369.scope CPUWeight=100
```

Other systemd knobs worth exploring later (not wired yet):

- `CPUSchedulingPolicy=idle`
- `IOWeight=1` / `IOSchedulingClass=idle`

## Handling Apps That Minimize to Tray When Closed

When KWin reports that a tracked window is removed:

1. Remove that window from the daemon's per-pid window map.
2. If the pid still has other tracked windows, reconcile as normal.
3. If the pid has zero tracked windows left:
   - Revert the current tier's actions (only effects app-nap applied).
   - Remove the pid from the daemon map.

This covers tray-close apps whose process survives after the last visible
window disappears. If revert fails because the process already exited, treat
that as harmless cleanup failure.

KWin side:

- Use `workspace.windowRemoved`.
- Daemon-side remove handling should stay idempotent so duplicate
  remove notifications are safe.
