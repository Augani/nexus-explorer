# Tech Stack

## Language
- Rust

## Core Framework
- GPUI (from Zed editor) - GPU-accelerated 2D UI framework

## Key Dependencies
- `tokio` - Async runtime for heavy I/O operations
- `jwalk` - Parallel directory traversal (uses rayon internally)
- `flume` - Async channels for data pipeline
- `nucleo` - Fuzzy search engine (from Helix editor)
- `notify` - Cross-platform file system watching
- `image` - Image decoding for icons
- `systemicons` - OS-native file icon retrieval

## Platform-Specific
- Windows: NTFS USN Journal, MFT parsing
- macOS: FSEvents, Grand Central Dispatch
- Linux: io_uring (Phase 2), inotify

## Architecture Patterns
- Entity-Component ownership model (GPUI's Model/View separation)
- Handle-based state management (`Model<T>`, `View<T>`)
- Generational ID tracking for async request validation
- LRU caching for directory states

## Threading Model
- Main Thread: UI only (max 8ms operations)
- Background Executor: GPUI's built-in task scheduler
- Tokio Thread Pool: Heavy I/O via `spawn_blocking`

## Build & Run
```bash
cargo build --release
cargo run
cargo test
```
