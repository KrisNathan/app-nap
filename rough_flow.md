# Rough Flow

1. Listen to window.active change with kwin script. window.active is false if window is minimized. This is good balance. (in the future we need to figure out how to detect if media is playing)
2. When window.active is false we do os call equivalent to kill -STOP <window.pid>
3. The user can still unminimize the window despite it being frozen. So the state still works.
4. When window.active is back to true we do os call equivalent to kill -CONT <window.pid>

## Handling Media Playback

We can obtain currently "playing" media via mpris dbus and then trace the pid of it.

```
kris@fedora:~$ qdbus | grep org.mpris.MediaPlayer2
 org.mpris.MediaPlayer2.brave.instance2
 org.mpris.MediaPlayer2.firefox.instance_1_602
kris@fedora:~$ qdbus org.mpris.MediaPlayer2.firefox.instance_1_602 /org/mpris/MediaPlayer2 org.freedesktop.DBus.Prop
erties.Get org.mpris.MediaPlayer2.Player PlaybackStatus
Playing
kris@fedora:~$ qdbus org.freedesktop.DBus /org/freedesktop/DBus org.freedesktop.DBus.GetConnectionUnixProcessID org.
mpris.MediaPlayer2.firefox.instance_1_602
67229
```
I checked kwin debug and the pid is correct, it is 67229 for the firefox window

## Handling Apps with Multiple Windows

Such as chromium PWA.

I guess our "appnap" service should take into account of all windows' active state and when all windows of a related pid are active=false, we can safely send SIGSTOP, and if a single window of that pid is suddenly active=true again we send SIGCONT

## DBus Monitor

```sh
dbus-monitor "interface='dev.appnap.AppNap1'"
```

## DBus Call

- service: dev.appnap.AppNap
- path: /dev/appnap/AppNap
- interface: dev.appnap.AppNap1

```sh
busctl --user call dev.appnap.AppNap /dev/appnap/AppNap dev.appnap.AppNap1 UpdateWindow sib "random-uuid" 1234 true
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
const id = "random-uuid"
const pid = 1234
const active = true
callDBus(
  SERVICE,
  PATH,
  IFACE,
  "UpdateWindow",
  id,
  pid,
  active,
);
```

## CPU Throttling as alternate

Solves:
- Process instability
- Unable to close minimized window without unminimizing (because KDE doesn't have before close signal/event)

For now the concrete implementation should just use systemd (in the future we can add a direct cgroupv2 edit for nonsystemd systems):
- Easy
- Systemd covers most linux desktop
- We won't conflict with systemd (apparently it can overwrite our changes on cgroupv2 files; moving the process to a separate cgroup is out of the question as it can get messy very quickly)

We can obtain the cgroup of a process with specific pid:

```sh
ps -o cgroup 89682
```

Output:

```
CGROUP
0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-flatpak-com.brave.Browser-1760258369.scope
```

Then we can throttle with systemd:

```sh
systemctl --user set-property app-flatpak-com.brave.Browser-1760258369.scope CPUQuota=5%
```

To reset:

```sh
systemctl --user set-property app-flatpak-com.brave.Browser-1760258369.scope CPUQuota=
```

Other systemctl scope considerations:
- cpusched
  - `CPUSchedulingPolicy=idle`: If an app is being too aggressive in the background even at 1%, you can also set CPUSchedulingPolicy=idle via systemd. This tells the kernel "only give this app cycles if absolutely no other process on the system wants them."
- io
  - `IOWeight=1` (default is 100): Setting it to 1 means that if any other app needs the disk, it gets 100x more priority than the napping app.
  - `IOSchedulingClass=idle`:  The app will only be allowed to use the disk if the drive is otherwise 100% idle.
