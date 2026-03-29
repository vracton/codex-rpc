# Discord Social SDK Presence Bridge

This repository now includes a small native bridge for Rich Presence plus a Node wrapper. The goal is to keep the Discord Social SDK in its supported native form while making it easy to drive from Node or Electron.

## What was added

- `src/discord_presence_bridge.cpp`: native bridge process
- `node/index.js`: Node client wrapper
- `node/example.js`: minimal usage example
- `CMakeLists.txt`: native build

## Why this shape

Discord's Social SDK is distributed as native binaries and headers. Rich Presence can be set without `Client::Connect()` by calling `SetApplicationId()` and `UpdateRichPresence()` against a running Discord desktop client, so a small native bridge is much lower risk than a full Node port.

Relevant docs:

- https://docs.discord.com/developers/discord-social-sdk/development-guides/setting-rich-presence
- https://docs.discord.com/developers/discord-social-sdk/getting-started/using-c%2B%2B#runtime-dependencies

## Repo vendoring

This repo vendors only the Windows helper inputs needed for Codex Rich Presence:

- `include/cdiscord.h`
- `windows/bin/discord_partner_sdk.dll`
- `windows/lib/discord_partner_sdk.lib`
- this README and `License-Notices.txt`

Windows-specific Rust code should stay outside this folder and outside the main WSL/Linux Codex crates.

## Build

```bash
cmake -S . -B build
cmake --build build
```

The build copies the required Discord runtime library next to the helper binary.

## Node usage

```js
const { DiscordPresenceClient } = require('./node');

const client = new DiscordPresenceClient();
await client.start(process.env.DISCORD_APPLICATION_ID);

await client.setPresence({
  type: 'playing',
  details: 'In Competitive Match',
  state: 'Rank: Diamond II',
  statusDisplayType: 'details',
  timestamps: {
    start: Date.now()
  },
  assets: {
    largeImage: 'map-mainframe',
    largeText: 'Mainframe'
  }
});
```

Supported commands from Node:

- `start(applicationId)`
- `setPresence(activity)`
- `clearPresence()`
- `stop()`

Supported activity fields:

- `type`: `playing`, `streaming`, `listening`, `watching`, `customStatus`, `competing`, `hangStatus`
- `details`
- `state`
- `statusDisplayType`: `name`, `state`, `details`
- `timestamps.start`
- `timestamps.end`
- `assets.largeImage`
- `assets.largeText`
- `assets.largeUrl`
- `assets.smallImage`
- `assets.smallText`
- `assets.smallUrl`
- `assets.inviteCoverImage`

## Notes

- Rich Presence without authentication only works with a running Discord desktop client.
- The safest deployment model is to run the helper on the same OS/session as Discord itself. If Discord is running on Windows, treat WSL as a development environment, not the runtime target for the bridge.
