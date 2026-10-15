# Genrl Scaffolding & Templates

Standardized code scaffolding templates for Flutter and Rust.
Every generated file strictly stays under **300 lines** (soft cap) and includes test stubs.

---

## 1. Flutter Mason Bricks (`flutter/`)

Mason bricks generate UI pages and typed bridge calls.

### Available Bricks
1. **`page_bundle`**:
   Generates a full page package following the single-responsibility architecture:
   - Page shell (`lib/pages/<name>/<name>_page.dart`)
   - Split section widgets (`hero_section.dart`, `content_section.dart`)
   - State & Cubit (`lib/services/cubit/<name>_cubit.dart`, `<name>_state.dart`)
   - Data model (`lib/models/<name>_model.dart`)
   - Widget test stub (`test/pages/<name>_page_test.dart`)

2. **`bridge_call`**:
   Generates a typed Rust core bridge invocation:
   - Extension method on `CoreBridge` (`lib/services/calls/<name>_call.dart`)
   - Request and response models (`lib/models/<name>_payload.dart`)
   - Unit test stub (`test/services/<name>_call_test.dart`)

### Usage with Mason CLI
```bash
cd tools/templates/flutter
mason get
mason make page_bundle --name analytics -o ../../../app
mason make bridge_call --name sync_catalog -o ../../../app
```

---

## 2. Rust Templates (`rust/`)

Generate standardized Rust modules, CLI commands, and database migrations.

### Available Templates
1. **`module`**: Folder module with `mod.rs`, `types.rs`, `logic.rs`, `db.rs`, and unit tests in `tests.rs`.
2. **`command`**: Single-file CLI subcommand with Clap arguments, execution handler, and async test stub.
3. **`migration`**: SQL migration script with idempotency, tables, and indices.

### Usage with Python Generator
```bash
# Generate a folder module under core/crates/genrl-core/src/
python3 tools/templates/rust/generate_rust_module.py module telemetry --dest core/crates/genrl-core/src/

# Generate a CLI command file
python3 tools/templates/rust/generate_rust_module.py command telemetry --dest core/crates/genrl-cli/src/commands/

# Generate a migration script
python3 tools/templates/rust/generate_rust_module.py migration add_telemetry_table --dest core/crates/genrl-core/migrations/
```

---

## Rules & Constraints
- **Line caps**: Soft cap 300 lines, hard cap 500 lines per file (enforced by `tools/check_caps.py`).
- **Separation of concerns**: Pages never call `CoreBridge` directly; they consume state through Cubits / Providers.
- **Test coverage**: All templates include corresponding unit or widget test stubs.
