# Genrl Downloader

A Linux desktop app that reads your MAL list, finds the right torrent on Nyaa for
each missing episode, downloads it through qBittorrent, and files it with a
synopsis and a folder icon.

**Flutter UI · Rust core · SQLite.** The UI never touches the network or the
disk: it sends commands to the Rust core and receives progress events. The core
owns the database and every call to MAL, Jikan, Nyaa and qBittorrent, which is
also what lets it keep working from the system tray while the window is closed.

## Six rules the whole design follows

1. **Never guess silently.** Uncertain match or torrent choice becomes a question
   in the Inbox instead of a wrong download.
2. **Every external call is budgeted.** MAL, Jikan, Nyaa and qBittorrent each get
   a rate limiter, a cache and a backoff, so the app cannot get you banned.
3. **Everything survives a crash.** All state is in SQLite; on restart the app
   resumes from the last known state and re-checks qBittorrent.
4. **One source of truth per fact.** The MAL id is the identity of a show; names,
   synonyms and folders hang off it.
5. **Automation is optional and visible.** Every automatic action appears in the
   activity log, and one switch pauses everything.
6. **Degrade, don't die.** If MAL, Jikan or Nyaa is down, the app keeps using its
   cache, keeps downloads going, and shows what is stale.

## Repository layout

```
core/                    Rust workspace
  crates/genrl-core      the core library (this is where all state lives)
    src/db.rs            SQLite, WAL mode, versioned migrations
    src/settings.rs      every default number, exportable to JSON
    src/rules.rs         group priorities, resolution, seeds — global and per show
    src/events.rs        the Activity log
    src/logging.rs       optional debug log, rotates by size
    src/http/            the limit-proof client layer (see below)
  crates/genrl-cli       `genrl` command-line companion
app/                     Flutter UI (phase 7)
assets/                  Stitch design screens + the Obsidian Violet design system
```

## The limit-proof HTTP layer

No module talks to the network directly; they ask a `Gate` for one of five
`Service`s. Each request passes through:

1. **Circuit breaker** — five consecutive failures open the service for 15
   minutes; jobs that need it wait in `blocked` instead of failing.
2. **Response cache** — fresh answers come from SQLite; stale entries revalidate
   with `If-None-Match`.
3. **Token-bucket limiter** — Jikan 2/s + 60/min, Nyaa one request per 2 seconds
   globally, MAL and qBittorrent gentler, all with jitter where it matters.
4. **The request** — timeout, 5 MB body cap, TLS verified, redirects capped at 3.
5. **Backoff** — 5 s, 30 s, 2 min, 10 min, 1 hour with random jitter, or the
   server's `Retry-After` when it sends one.

## Building

```bash
# Rust toolchain (rustup)
cd core
cargo build --workspace
cargo test  --workspace     # includes the mock-server 429/timeout suite
./target/debug/genrl init   # create the data folder + database
./target/debug/genrl doctor # check database, permissions, settings, breakers
```

Data lives in `$XDG_DATA_HOME/genrl-downloader/` (`~/.local/share/…`), directory
mode `0700`, database mode `0600`; the app refuses to start if either is
group- or world-writable.

## Roadmap

Each phase ends with a check that must pass before the next one starts.

- [x] **Foundation** — Rust workspace, SQLite schema and migrations, settings,
      logging, shared HTTP layer with limiter, cache and backoff.
      *Check: mock-server tests for 429 and timeout pass.* ✅ 56 tests green
- [ ] **qBittorrent and Nyaa** — client for qBittorrent, RSS search, filename
      parser, scorer. *Check: a CLI tool downloads one chosen episode end to end.*
- [ ] **Matching and seed logic** — name sets, scoring, thresholds, questions.
- [ ] **Queue and workers** — job states, retries, stall handling, crash recovery.
- [ ] **MAL sync** — OAuth, list diff, episode generation, first-sync preview.
- [ ] **Library step** — rename/move, synopsis.txt, posters, folder icons.
- [ ] **Flutter UI** — nine pages in order, styled from `assets/`.
- [ ] **Polish and release** — tray, autostart, notifications, backup, AppImage.

## License

For personal use only. Please respect copyright laws and only download content
you have the right to access.
