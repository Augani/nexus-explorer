mod pipeline;
mod platform;
mod traversal;

pub use pipeline::{
    create_batch_pipeline, max_batches_for_items, BatchAggregator, BatchConfig, DEFAULT_BATCH_SIZE,
    DEFAULT_FLUSH_INTERVAL,
};
pub use platform::{
    detect_platform, EventCoalescer, LinuxPlatform, LinuxWatcher, MacOsPlatform, MacOsWatcher,
    PlatformFs, Watcher, WindowsPlatform, WindowsWatcher, DEFAULT_COALESCE_WINDOW,
};
pub use traversal::{
    sort_entries, spawn_sorted_traversal, spawn_traversal, traverse_directory,
    traverse_directory_sorted, SortKey, SortOrder, TraversalConfig,
};
