# Project Structure

## Planned Architecture

```
src/
├── main.rs              # App initialization, GPUI setup
├── app/
│   └── workspace.rs     # Root View, layout management
├── models/
│   ├── file_system.rs   # FileSystem Model, I/O coordination
│   ├── icon_cache.rs    # GPU texture management, LRU eviction
│   ├── search_engine.rs # Nucleo integration, SearchSession
│   └── settings.rs      # GlobalSettings
├── views/
│   ├── file_list.rs     # Virtualized list, ListDelegate impl
│   ├── sidebar.rs       # Navigation tree
│   └── preview.rs       # File preview pane
├── io/
│   ├── traversal.rs     # jwalk integration, parallel scanning
│   ├── pipeline.rs      # Channel batching, debouncing
│   └── platform/
│       ├── windows.rs   # USN Journal, MFT parser
│       ├── macos.rs     # FSEvents watcher
│       └── linux.rs     # io_uring, inotify
└── utils/
    ├── icons.rs         # Icon decoding, BGRA swizzling
    └── cache.rs         # LRU cache implementation
```

## Key Entities
| Entity | Type | Purpose |
|--------|------|---------|
| Workspace | View | Root container, layout panes |
| FileSystem | Model | File data, path state, I/O tasks |
| FileList | View | Virtualized rendering |
| IconCache | Model | GPU textures, VRAM management |
| SearchEngine | Model | Nucleo fuzzy matcher |

## Data Flow
1. User action triggers `FileSystem::load_path()`
2. Task dispatched to Background Executor
3. I/O offloaded to Tokio pool (jwalk)
4. Results streamed via flume channel
5. Batched updates (100 items or 16ms) sent to UI
6. Model updated, View auto-rerenders
