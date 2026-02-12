# rai!connect

Local HTTPS proxy enabling osu!direct through the [rai.moe](https://rai.moe) beatmap mirror.

## How It Works

rai!connect runs a local HTTPS proxy on port 443. When osu! is launched with `-devserver localhost`, all game traffic is routed through this proxy.

- **osu!direct requests** (search, download, thumbnails) are redirected to `direct.rai.moe`
- **All other requests** (login, scores, multiplayer) are forwarded to official `*.ppy.sh` servers

This approach keeps your gameplay on official servers while enabling beatmap downloads through the mirror.

## Features

- **HTTPS proxy** on port 443 with automatic TLS certificate handling
- **Auto-setup** on first launch:
  - Generates and installs a self-signed certificate to the Windows trust store
  - Adds localhost subdomain entries (`osu.localhost`, `c.localhost`, etc.) to the hosts file
- **One-click launch** - starts the proxy and opens osu! with the correct flags
- **Supporter injection** - enables osu!direct in the client without an active subscription

## Requirements

- Windows 10/11
- osu! (stable client)

## Installation

Download from [releases](https://github.com/rai-osu/connect/releases) or build from source:

```bash
pnpm install
pnpm tauri build
```

## Usage

1. Launch rai!connect (accepts Windows UAC prompt)
2. Click **Connect & Launch osu!**
3. osu!direct is now enabled

The proxy automatically configures your system on first run. No manual setup required.

## FAQ

### Is this safe?

Yes. Only beatmap-related requests are intercepted. All gameplay traffic (login, scores, multiplayer) passes through to official servers unchanged.

### Will I get banned?

No. This doesn't modify the game client. It's equivalent to using any beatmap mirror website.

### Why does it need admin privileges?

The app requires administrator access to:
- Bind to port 443 (HTTPS)
- Modify the Windows hosts file
- Install the certificate to the trust store

### Does this work with osu!lazer?

No. osu!lazer has its own beatmap download system.

## Tech Stack

- [Tauri v2](https://v2.tauri.app/) (Rust + SvelteKit)
- [rustls](https://github.com/rustls/rustls) for TLS
- [hyper](https://hyper.rs/) for HTTP

## License

MIT
