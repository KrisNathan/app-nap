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
