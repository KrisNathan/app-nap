# Debugging KWin Script

```sh
journalctl -f _COMM=kwin_wayland -o cat
```

Must use `console.info`. Nothing else appears somehow.

```js
console.info('Hi')
```

