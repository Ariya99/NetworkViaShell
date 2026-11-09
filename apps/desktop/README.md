# Desktop app foundation

Tauri 2 + React + TypeScript + Vite shell.

## Requirements
- Node.js 22+
- pnpm 9+
- Rust stable
- Tauri platform prerequisites: https://v2.tauri.app/start/prerequisites/

## Run
```sh
cd apps/desktop
pnpm install
pnpm tauri dev
```

## Build frontend
```sh
pnpm build
```

The current UI is a static foundation screen. It has no connection commands, engine process management, profile storage or secret handling yet.
