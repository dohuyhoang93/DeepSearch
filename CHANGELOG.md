# Changelog

All notable changes to DeepSearch will be documented in this file.

---

## [1.2.1] - 2026-04-21

### ⚠️ Breaking Change — Index Compatibility

> **Indexes created with redb 2.6.x are no longer compatible.**
>
> This release upgrades the embedded database from redb 2.6.3 to redb 4.1.0, which introduces
> a new file format (v3). Existing index files (`deepsearch_index.redb`) built with a previous
> version of DeepSearch **will not be read correctly** and may cause a startup error.
>
> **Action required:** Delete the old index file and re-index your folders from scratch.

---

### ✨ New Features

#### Live Search
In addition to the traditional "Indexed Search", users can now search directly inside any
folder without building an index first.

- **Activation:** In the "Search" tab, check "Live Search in Folder".
- **Two modes:**
  1. **Filename search (default):** Ultra-fast, matches only against file names.
  2. **Content search:** Check "Search in file content" to search inside file contents.

#### Multi-format Content Search
- **PDF** (`pdf-extract`): Results include page number `[Page X]`.
- **Microsoft Word** (`.docx`): Supported via `docx-rs`.
- **Microsoft Excel** (`.xlsx`): Supported via `calamine`.
- Binary files (`.jpg`, `.exe`, `.zip`, …) are automatically skipped to avoid garbage results.

---

### 🚀 Improvements & Refactoring

#### Upgrade redb 2.6.3 → 4.1.0
- **~15% faster** concurrent reads from multiple threads.
- **~1.5× faster** general write throughput.
- Minimum database file size reduced from ~2.5 MB to ~50 KB.
- Numerous critical data-corruption and memory-leak bugs patched upstream.

#### File scan architecture
- **Unified 2-phase scan:** `walkdir` for directory discovery + `rayon` for parallel processing,
  used consistently across all tasks (Initial Scan, Rescan, Live Search).
- **Safer Rescan:** Three-step workflow (`scan → write temp table → atomic swap`) ensures the
  existing index is never corrupted if the process is interrupted mid-way.

#### Consistent search logic
- Live Search filename matching now uses the same **token-based** algorithm as Indexed Search,
  producing consistent results across both modes.
- `contains_all_tokens` extracted as shared utility.

#### Code quality — Clippy pedantic
All 63 warnings from `cargo clippy -D clippy::all -D clippy::pedantic` resolved:
- `ref_option`: Changed `&Option<T>` → `Option<&T>` in utility function signatures.
- `non_std_lazy_statics`: Removed `once_cell` dependency; replaced with `std::sync::LazyLock`
  (stable since Rust 1.80).
- Additional lints fixed: `unnested_or_patterns`, `uninlined_format_args`, `map_unwrap_or`,
  `implicit_clone`, `redundant_closure`, `if_not_else`, `derivable_impls`, `manual_string_new`,
  `default_trait_access`, `doc_markdown`, `case_sensitive_file_extension_comparisons`.
- Removed `once_cell` from `Cargo.toml`.

---

### 🐞 Bug Fixes

- Live Search no longer accumulates results across separate search sessions.
- Fixed filename search results not appearing in the UI.
- Fixed PDF result display formatting.
- Fixed case-sensitive file extension comparison — `.PDF`, `.Pdf` etc. are now correctly matched.

---

## [1.2.0]

- Previous stable release.

## [1.1.0]

- See git log for details.

## [1.0.0]

- Initial release.
