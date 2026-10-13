<p align="center">
  <img src="logo.png" alt="Genrl Downloader Logo" width="160" height="160" />
</p>

<h1 align="center">Genrl Downloader</h1>

<p align="center">
  <strong>Autonomous, Limit-Proof Anime Ingestion, Torrent Automation & Media Library Orchestrator.</strong><br/>
  <em>Engineered for the high seas of anime hoarding. Zero-guesswork, zero-duplicate storage, memory-safe.</em>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Status-In%20Testing%20Phase%20(App%20Ready)-f59e0b?style=for-the-badge&logo=target&logoColor=white" alt="Testing Phase" />
  <img src="https://img.shields.io/badge/Platform-Linux%20(Arch%20%7C%20Fedora%20%7C%20Debian%20%7C%20Ubuntu)-1793d1?style=for-the-badge&logo=linux&logoColor=white" alt="Linux Platform" />
  <img src="https://img.shields.io/badge/Rust-1.85%2B%20(207%20Tests%20Passing)-DEA584?style=for-the-badge&logo=rust&logoColor=black" alt="Rust 207 Tests" />
  <img src="https://img.shields.io/badge/Flutter-Desktop%20(19%20Tests%20Passing)-02569B?style=for-the-badge&logo=flutter&logoColor=white" alt="Flutter Desktop" />
  <img src="https://img.shields.io/badge/Storage-Hardlink%20%2F%20Reflink%20Instant-10b981?style=for-the-badge" alt="Zero-Copy Storage" />
  <img src="https://img.shields.io/badge/Security-PKCE%20OAuth%20%2B%20System%20Keyring-8b5cf6?style=for-the-badge" alt="Keyring Security" />
</p>

> [!IMPORTANT]
> **🚀 Testing Phase Announcement (v2.4 Beta)**
> Genrl Downloader is now fully assembled and operational! The Rust Core (`genrl`), CLI suite, and the Obsidian Flutter Desktop application are feature-complete and entering the active community testing and verification phase.
> * **207 Rust Core Tests**: 100% Passing.
> * **19 Flutter UI & Integration Tests**: 100% Passing, 0 Analyzer Warnings.
> * **Verified In Real Use**: Full MAL public profile sync (tested with 1,040+ shows), qBittorrent v5.2.4 integration, dynamic season/episode queueing, and hardlink filesystem filing.

---

## 🏴‍☠️ What is Genrl Downloader?

**Genrl Downloader** is a self-hosted, local-first anime automation powerhouse built for collectors who refuse to compromise. It continuously monitors your anime watchlist, autonomously matches upcoming and missing releases across decentralized swarms, downloads them via qBittorrent, and organizes them into an immaculate, Plex/Jellyfin-ready directory hierarchy with zero file duplication, embedded synopses, and native OS desktop folder thumbnails.

Unlike brittle scripts or monolithic web scrapers, Genrl is split into two bulletproof tiers:
1. **The Rust Core (`genrl` / `genrl-core`)**: A blazing fast, memory-safe background engine owning SQLite state (WAL mode), PKCE OAuth, circuit breakers, rate limiters, token buckets, and torrent RPCs.
2. **The Obsidian Flutter Desktop UI**: A sleek, reactive cyberpunk desktop dashboard featuring live telemetry feeds, dynamic download radar animations, interactive question inboxes, and per-show rules management.

```
       [ MyAnimeList / Jikan API ]         [ Nyaa Swarm RSS / Mirrors ]
                    ▲                                   ▲
                    │                                   │
┌───────────────────┴───────────────────────────────────┴───────────────────┐
│                           RUST CORE ENGINE                                │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │                THE GATE (Circuit-Breakers & Token Buckets)          │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│         │                                                    │            │
│         ▼                                                    ▼            │
│   SQLite Database                                    qBittorrent WebUI    │
│   (WAL Mode, Keys, Logs)                             (API 4.x / 5.x)      │
│         │                                                    │            │
│         ▼                                                    ▼            │
│   Action Inbox ◄──── Pure Release Decision Engine ────► Zero-Copy Storage │
│   (Collisions, Ties)                                   (Hardlink/Reflink) │
└───────────────────────────────────┬───────────────────────────────────────┘
                                    │ IPC / Live Telemetry Stream
                                    ▼
┌───────────────────────────────────────────────────────────────────────────┐
│                    FLUTTER DESKTOP APP (OBSIDIAN VIOLET)                  │
│  • Live Pulse Bar & Metric Cards       • Interactive Action Inbox         │
│  • Animated Ingestion Radar & Chevrons • MAL OAuth2 PKCE Connect          │
│  • Anime Library Poster Grid (2:3)     • Real-time Rules & Overrides      │
└───────────────────────────────────────────────────────────────────────────┘
```

---

## ✨ Key Architectural Capabilities

### 1. 🛡️ The Limit-Proof Gate
External network endpoints (MAL, Jikan, Nyaa RSS mirrors, qBittorrent) are strictly shielded behind an infallible traffic gate:
* **Token Bucket Rate Limiting**: Nyaa requests are capped at 1 request per 2 seconds; Jikan throttled to compliant burst limits.
* **5-Strike Circuit Breakers**: If a mirror or API throws 5 consecutive server errors (5xx) or rate limits (429), the circuit breaker opens for 15 minutes. Dependent background jobs automatically pause in a `blocked` state rather than crashing or spamming.
* **SQLite Response Caching**: Responses are stored in SQLite with full `ETag` and `If-None-Match` revalidation.

### 2. ⚡ Zero-Copy Hardlink & Reflink Media Storage
* **Seed Forever Without Burning Disk Space**: Files organized into your TV/Anime library are instantly linked using filesystem hardlinks (`ln`) or Btrfs/ZFS reflinks.
* qBittorrent seeds untouched in `~/Downloads`, while Jellyfin/Plex streams organized files from `/mnt/anime/Show (Year)/Season 01/...`.

### 3. ⚖️ The Action Inbox (Never Guess Silently)
When matching engines encounter ambiguous torrent candidates, thin seed counts, or file collisions:
* The system **never guesses wrong** and never overwrites user data silently.
* Instead, it pauses only that specific episode and posts a rich interactive decision card into your **Action Inbox** with selectable resolutions and an optional *"Always choose this"* rule generator.

### 4. 🔐 Security First (MAL PKCE OAuth + OS Keyring)
* Supports official MyAnimeList OAuth2 with **PKCE (method: plain)**.
* Sensitive refresh tokens are stored exclusively inside the Linux desktop Secret Service / system Keyring (`libsecret` / `gnome-keyring` / `kwallet`). Zero plaintext credentials in configs or logs.
* Fallback to read-only public MAL profiles via Jikan for users without API developer accounts.

---

## 🗺️ Cross-Platform Roadmap

| Platform | Tier | Status | Notes |
|:---|:---:|:---:|:---|
| 🐧 **Linux (X11 / Wayland)** | **Tier 1** | **Testing Phase (App Ready)** | Full desktop app, system keyring, GNOME gio / KDE `.directory` folder posters, CLI companion. |
| 🪟 **Windows (10/11)** | **Tier 1** | **Coming Soon** | Native Windows background service, NTFS hardlinks, Windows Credential Vault, and WinToast alerts. |
| 🍎 **macOS** | **Tier 2** | **Planned** | Native APFS clone-file (copy-on-write) linking and macOS Keychain integration. |

---

## 🧪 Testing & Verification Guide

Genrl Downloader is currently in its active **Testing Phase**. Both the Rust engine and Flutter frontend have complete test suites and real-world verified pipelines.

### Running Test Suites

```bash
# 1. Run all 207 Rust Core tests
cd core
cargo test --workspace

# 2. Run all Flutter Desktop widget and integration tests
cd ../app
flutter test
```

### What to Test During Beta

1. **MAL Connection & Sync**:
   * Connect via official OAuth PKCE or set a public profile username (e.g., `akumasan2`).
   * Verify all watch status counts (Watching, Completed, Plan to Watch, etc.) sync into the SQLite database.
2. **Library View & Episode Matrix**:
   * Browse through thousands of shows smoothly (virtualized sliver rendering).
   * Click any show to view metadata, posters, and the interactive episode grid.
   * Trigger targeted downloads with **"Download Season"** or **"EP N"** buttons.
3. **Queue & Automation**:
   * Verify jobs move from `search` to `downloading` in qBittorrent.
   * Toggle the global pause/resume switch to verify workers stop and start without interrupting active torrent transfers.
4. **Action Inbox**:
   * Verify interactive decision cards appear when low seeds, name collisions, or batch choices are detected.
5. **Storage & Hardlink Verification**:
   * Check **Settings ➔ Storage Locations** to ensure same-filesystem detection passes.
   * Verify completed files are hardlinked into your library with zero duplicate storage used.

### Diagnostics & Submitting Logs

If you encounter unexpected behavior during testing:
```bash
# Inspect system diagnostics
genrl doctor

# View live system and qBittorrent telemetry
genrl status

# Dump the last 50 events from the SQLite audit log
genrl events --limit 50 --json
```

---

## 🚀 Quickstart & Installation (Linux)

### Prerequisites
* **Rust**: `1.85+` (`cargo`, `rustc`)
* **Flutter**: `3.27+`
* **qBittorrent**: `4.x` or `5.x` (Desktop or `qbittorrent-nox`)
* **System Libraries**: `libsecret-1-dev`, `libglib2.0-dev`, `libgtk-3-dev`

```bash
# Ubuntu / Debian
sudo apt install build-essential libsecret-1-dev libgtk-3-dev libglib2.0-dev imagemagick

# Arch Linux / Manjaro
sudo pacman -S base-devel libsecret gtk3 imagemagick
```

### Build & Install

```bash
# Clone the repository
git clone https://github.com/genrl/Genrl-downloader.git
cd Genrl-downloader

# Build and test the Rust core (207 workspace tests)
cd core
cargo test --workspace
cargo install --path crates/genrl-cli --force

# Build and run the Flutter Desktop UI
cd ../app
flutter test
flutter run -d linux
```

### Desktop Integration
Installing places the binary and desktop launcher in standard XDG locations:
* Binary: `~/.local/bin/genrl` & `~/.local/bin/genrl-downloader`
* Desktop Entry: `~/.local/share/applications/genrl-downloader.desktop`
* Icons: `~/.local/share/icons/hicolor/512x512/apps/genrl-downloader.png`

---

## 💻 CLI Swiss-Army Knife

The Rust core ships with a full-featured CLI companion (`genrl`):

```bash
# Live status, transfer speeds, and storage overview
genrl status

# Tail the real-time activity and security log
genrl events --limit 25

# Inspect what is active in the torrent queue
genrl queue --json

# List and answer pending Inbox questions from the terminal
genrl inbox
genrl answer 101 keep --always

# Search Nyaa through the rate limiter and circuit breaker
genrl search "Sousou no Frieren 1080p"

# Force an immediate MyAnimeList sync
genrl mal sync

# Manage matching rules and preferences
genrl rules get
genrl rules set preferred_resolution '"1080p"'
genrl rules set min_seeders 10

# Scan existing anime library folders
genrl library rescan
```

---

## 🎨 The Obsidian Violet Design System

The Genrl Downloader desktop client is rendered in a custom **Obsidian Violet** theme:
* **Deep Low-Light Surface Palette**: Engineered for dark Linux desktop media setups (`#090B10` to `#181C22`).
* **Active Neon Violets & Cyans**: High-visibility transfer telemetry, seeder health meters, and anime status pills.
* **Dynamic Cybernetic Radar**: Animated SVG-inspired canvas rendering that reacts in real-time to swarm speeds and active packet ingestion.

---

## 🔒 Security & Privacy Policy

* **No Telemetry, No Analytics**: Genrl Downloader connects *only* to the endpoints you configure (MyAnimeList and torrent indexers). Zero third-party telemetry, tracking, or telemetry pings.
* **No Plaintext Passwords**: Keyring integration via Secret Service ensures API credentials and passwords never leak into SQLite databases or log files.
* **Safe Filesystem Navigation**: Path sanitization protects against directory traversal (`..`) attacks in torrent filenames and metadata.

---

## 📜 License & Disclaimer

Genrl Downloader is free and open-source software distributed under the **GPL-3.0 License**.

*Disclaimer: Genrl Downloader is an automated indexer and library orchestrator. Users are solely responsible for ensuring their downloads and media consumption comply with local laws and regulations.*
