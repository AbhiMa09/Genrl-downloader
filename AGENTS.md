# AGENTS.md — Contributor & AI Agent Engineering Guide

Welcome to the **Genrl Downloader** codebase. This guide details the architectural layers, file organization, hard rules, scaffolding templates, command workflows, and design system protocols for developers and AI agents working on this project.

---

## 1. System Architecture & Layers

Genrl is built as an ultra-reliable, automated media manager split into clear decoupled layers:

```
┌─────────────────────────────────────────────────────────────┐
│                       Flutter UI                            │
│  app/lib/pages/               app/lib/widgets/              │
│  (Page Shells + Sections)     (Loading, Postcards, Banners) │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼ (Strict Lint: Pages NEVER import bridge)
┌─────────────────────────────────────────────────────────────┐
│                    App State / Cubits                       │
│  app/lib/state/ (queue, library, show_detail, settings...)  │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                 Typed CoreBridge Interface                  │
│  app/lib/services/core_bridge.dart                          │
│  app/lib/services/generated_contract.dart                   │
└──────────────────────────────┬──────────────────────────────┘
                               │ (IPC / Process Pipe)
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                     Single-Source Contract                  │
│  core/contract/contract.json                                │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                     Rust Core Daemon                        │
│  core/crates/genrl-core/ (Queue, MAL, Nyaa, QBit, SQLite)   │
│  core/crates/genrl-cli/  (CLI Dispatch & Command Handlers)  │
└─────────────────────────────────────────────────────────────┘
```

### Architectural Guardrails:
1. **UI Pages never import `core_bridge.dart` directly**: Pages must only depend on state management classes (`cubits` or `state/`). Enforced by automated architecture test (`app/test/architecture_guardrails_test.dart`).
2. **Single-Source Contract**: All core commands, queries, and events are declared once in `core/contract/contract.json`. Dart contracts and bridge interfaces are generated via `tools/generate_contract.py`. Every command must either be marked `internal: true` or have an active UI bridge binding.
3. **Design Tokens**: The design system in `assets/obsidian_violet_media_core/DESIGN.md` is the single source of truth for colors, typography, spacing, and radii. Dart themes are generated via `tools/generate_theme.py`.

---

## 2. Repository Map & Where Things Live

| Directory | Purpose | Key Files |
| :--- | :--- | :--- |
| `core/` | Rust workspace | `Cargo.toml`, `crates/genrl-core`, `crates/genrl-cli` |
| `core/contract/` | Single-source API contract | `contract.json` |
| `core/crates/genrl-core/` | Core business logic & database | `db/`, `http/gate.rs`, `queue/`, `mal/`, `filename/` |
| `core/crates/genrl-cli/` | Command line interface | `src/main.rs`, `src/commands/` |
| `app/` | Flutter desktop application | `pubspec.yaml`, `analysis_options.yaml` |
| `app/lib/pages/` | Application pages | `dashboard/`, `queue/`, `library/`, `show_detail/`, `settings/` |
| `app/lib/state/` | State management cubits | `queue_cubit.dart`, `library_cubit.dart`, `app_state.dart` |
| `app/lib/services/` | Core bridge & IPC services | `core_bridge.dart`, `generated_contract.dart` |
| `app/lib/theme/` | Generated design tokens & theme | `colors.dart`, `spacing.dart`, `typography.dart` |
| `app/lib/widgets/` | Reusable shared widgets | `cyber_badge.dart`, `loading/cyber_loader.dart`, etc. |
| `assets/` | Design tokens & exported screens | `obsidian_violet_media_core/DESIGN.md`, screen mock exports |
| `tools/` | Scaffolding templates & guardrails | `templates/`, `check_caps.py`, `generate_contract.py`, `generate_theme.py`, `pre_commit.sh` |
| `.github/workflows/` | Continuous Integration | `ci.yml` |

---

## 3. The File Size Rule

All production source code files in `app/lib/` and `core/` strictly enforce line limits:
- **Soft Cap**: **300 lines** per file. Aim to keep files under this limit.
- **Hard Cap**: **500 lines** per file (strictly enforced by `tools/check_caps.py` in pre-commit and CI; test files excepted).

### How to Split Oversized Files:
- **Flutter Pages**: Never let a page grow into a giant monolith. Split into a folder module:
  - `page_name_page.dart` (lightweight shell and layout orchestrator)
  - `hero_section.dart`
  - `content_section.dart`
  - `tab_*.dart` / dialog widgets
- **Rust Modules**: Never write multi-hundred-line monolithic files. Convert to a folder module:
  - `mod.rs` (public re-exports and high-level orchestrator)
  - `types.rs` (structs, enums, error types)
  - `logic.rs` (core business algorithms)
  - `db.rs` (database access and SQL queries)
  - `tests.rs` (unit tests)

---

## 4. Scaffolding Templates

Always use the provided templates when creating new pages, bridge calls, or Rust modules. Each template produces code under 300 lines with pre-wired test stubs.

### A. Flutter Templates (Mason Bricks)
Located in `tools/templates/flutter/`:
1. **New Page Bundle (`page_bundle`)**:
   Generates a complete page folder with shell, hero, content section, cubit, state, model, and widget test stub:
   ```bash
   cd tools/templates/flutter
   mason make page_bundle --name episode_guide
   ```
2. **New Bridge Call (`bridge_call`)**:
   Generates a typed extension method on `CoreBridge`, request/response models, and unit test stub:
   ```bash
   cd tools/templates/flutter
   mason make bridge_call --name purge_cache --method POST
   ```

### B. Rust Module & Command Generator
Located in `tools/templates/rust/`:
1. **New Rust Folder Module**:
   Generates a 5-file module (`mod.rs`, `types.rs`, `logic.rs`, `db.rs`, `tests.rs`):
   ```bash
   python3 tools/templates/rust/generate_rust_module.py telemetry
   ```
2. **New CLI Command**:
   Copy `tools/templates/rust/cli_command/command.rs` into `core/crates/genrl-cli/src/commands/<name>.rs` and register in `commands/mod.rs`.
3. **New SQL Migration**:
   Use `tools/templates/rust/migration/migration.sql` as the baseline schema migration template.

---

## 5. Development Commands Cheat Sheet

Run these commands regularly during development:

```bash
# 1. Format code
cargo fmt --manifest-path core/Cargo.toml --all
dart format app/

# 2. Lint and Static Analysis (Zero warnings enforced)
cargo clippy --manifest-path core/Cargo.toml --workspace -- -D warnings
(cd app && flutter analyze)

# 3. Size Cap Verification
python3 tools/check_caps.py

# 4. Contract & Design Token Synchronization
python3 tools/generate_contract.py --check
python3 tools/generate_theme.py --check

# 5. Run Test Suites
cargo test --manifest-path core/Cargo.toml --workspace
(cd app && flutter test)

# 6. Complete Pre-Commit Verification (runs all above checks)
./tools/pre_commit.sh
```

---

## 6. Core Development Rules

1. **Refactor and Features in Separate Commits**:
   Never mix architectural refactorings or file splits with feature changes or behavior adjustments. Each must be committed and verified independently.
2. **Goldens Never Regenerated to Make a Test Pass**:
   Golden tests verify visual pixel regressions. If a golden test fails during a refactor, **the refactor is wrong: revert and fix the code**. Goldens are only regenerated when an intentional UI redesign has been approved.
3. **Zero Warnings Policy**:
   Both `flutter analyze` and `cargo clippy -- -D warnings` must exit with zero warnings. Treat warnings as compile errors.

---

## 7. Stitch (Design MCP) Guidelines

Stitch is the design source for the UI. Flutter implements the designs; Stitch never touches backend logic or data wiring.

### Source of Truth
- `assets/` holds `DESIGN.md` (tokens) and every exported screen (`genrl_downloader_dashboard`, `_inbox`, `_queue`, `_library`, `_search`, `_activity`, `_rules`, `_settings`, `_show_detail`, `_setup_wizard`, `_loading_ingestion_state`, `_tray_menu_notifications`, `_app_logo`).
- **Conflict Rule**: If `assets/` and Stitch disagree, **`assets/` wins** until a new export replaces it.
- Start from the "Stitch prompts for each page" section in the design documentation.

### When to Use Stitch
- A new page, a redesign, or a missing UI state (empty, loading, error, hover).
- New icons, logo variants, tray or notification visuals.
- Comparing two layout options before coding.
- **Do NOT use Stitch for**: logic, data wiring, bug fixes, small spacing/color tweaks already in `DESIGN.md`, or anything Flutter code can do directly.

### Workflow
1. Read `DESIGN.md` and the existing export for the target page.
2. List the Stitch MCP tools and inspect existing projects. Reuse the existing project; ask before creating a new one.
3. Generate or edit the screen using `DESIGN.md` tokens in the prompt (Obsidian Violet theme, Inter + JetBrains Mono, 8/12/16px radii, surface elevation levels).
4. Export to `assets/<screen_name>/` (PNG + HTML) as a **NEW version**; never overwrite an existing export. Save the prompt and Stitch screen ID in `assets/<screen_name>/README.md`.
5. Implement in Flutter using tokens and shared components, **not** by pasting HTML. Replace every mock value with live data from the core.
6. Run the pixel audit against the export and fix mismatches.
7. Commit the design assets and the Flutter implementation in separate commits.

### Security & Operational Rules
- **API Key Security**: Never write the Stitch API key into any file or logs. It comes solely from the `STITCH_API_KEY` environment variable.
- **Resilience**: If a Stitch call fails or is rate limited, continue working from existing assets, report it, and do not invent arbitrary design tokens.
- **Dynamic Data**: Designs show placeholder data; the Flutter app must use dynamic data only.
