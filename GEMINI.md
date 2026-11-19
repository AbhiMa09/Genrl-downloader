# Genrl Downloader — Project Memory & Low-Token Rules

Complements [AGENTS.md](AGENTS.md) without duplicating it, and inherits global Ponytail / token-economy rules from `~/.gemini/GEMINI.md`.

## Architecture & Token Guards
- Enforce `AGENTS.md`: UI pages/widgets never import `core_bridge.dart`, single-source contract (`core/contract/contract.json`), `DESIGN.md` tokens, zero warnings (`flutter analyze`, `cargo clippy -- -D warnings`), 300 soft / 500 hard line cap (`tools/check_caps.py`), and never regenerate goldens to fix refactors.
- Search with `rg -n` first and read <=120-line slices; split oversized Flutter pages or Rust folder modules cleanly instead of reading or editing monoliths.

## Persistent Project Memory
- **Universal auto-download engine**: `tools/universal_autodownload.py` enforces Priority 1 batch/season packs (fitting free disk space), Priority 2 single-episode fallback, 1080p -> 720p/x265 preference, and >= 4 seeders auto-accept, logging to `~/.local/share/genrl-downloader/autodownload.log`.
- **Known limits**: Tracked in `KNOWN_ISSUES.md` (live `api.jikan.moe` reliability, unverified live MAL cover download, fixed 1.4 GB/ep assumption, undecided dual-audio scoring rule, stale default release groups). Never claim live network or qBittorrent behavior works without fresh verification.
- **Safety**: Never commit/push without explicit instruction. Never wipe live SQLite state, qBittorrent queues, or download paths (`/mnt/hhd/GenrlTest/Downloads`, `/mnt/new-volume/Downloads`) without explicit user approval.
