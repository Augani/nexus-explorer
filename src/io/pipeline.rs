use std::time::{Duration, Instant};

use flume::{Receiver, Sender};

use crate::models::FileEntry;

/// Default batch size threshold (number of items)
pub const DEFAULT_BATCH_SIZE: usize = 100;

/// Default time threshold for flushing batches (16ms for 60fps)
pub const DEFAULT_FLUSH_INTERVAL: Duration = Duration::from_millis(16);

/// Configuration for the batch aggregator
#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub batch_size: usize,
    pub flush_interval: Duration,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            batch_size: DEFAULT_BATCH_SIZE,
            flush_interval: DEFAULT_FLUSH_INTERVAL,
        }
    }
}

/// Batch aggregator that collects FileEntry items and delivers them in batches.
/// 
/// Batches are flushed when either:
/// - The batch reaches `batch_size` items (default: 100)
/// - The `flush_interval` has elapsed since the last flush (default: 16ms)
/// 
/// This ensures the UI receives updates at a reasonable rate without being
/// overwhelmed by individual item updates.
pub struct BatchAggregator {
    config: BatchConfig,
    input: Receiver<FileEntry>,
    output: Sender<Vec<FileEntry>>,
}

impl BatchAggregator {
    pub fn new(
        input: Receiver<FileEntry>,
        output: Sender<Vec<FileEntry>>,
        config: BatchConfig,
    ) -> Self {
        Self {
            config,
            input,
            output,
        }
    }

    /// Runs the batch aggregation loop until the input channel is closed.
    /// Returns the total number of items processed.
    pub fn run(self) -> usize {
        let mut batch = Vec::with_capacity(self.config.batch_size);
        let mut last_flush = Instant::now();
        let mut total_items = 0;

        loop {
            // Try to receive with a timeout based on remaining flush interval
            let elapsed = last_flush.elapsed();
            let remaining = self.config.flush_interval.saturating_sub(elapsed);

            match self.input.recv_timeout(remaining) {
                Ok(entry) => {
                    batch.push(entry);
                    total_items += 1;

                    // Flush if batch is full
                    if batch.len() >= self.config.batch_size {
                        if self.flush_batch(&mut batch).is_err() {
                            break;
                        }
                        last_flush = Instant::now();
                    }
                }
                Err(flume::RecvTimeoutError::Timeout) => {
                    // Time-based flush
                    if !batch.is_empty() {
                        if self.flush_batch(&mut batch).is_err() {
                            break;
                        }
                    }
                    last_flush = Instant::now();
                }
                Err(flume::RecvTimeoutError::Disconnected) => {
                    // Input channel closed, flush remaining items
                    if !batch.is_empty() {
                        let _ = self.flush_batch(&mut batch);
                    }
                    break;
                }
            }
        }

        total_items
    }

    fn flush_batch(&self, batch: &mut Vec<FileEntry>) -> Result<(), flume::SendError<Vec<FileEntry>>> {
        let items = std::mem::take(batch);
        *batch = Vec::with_capacity(self.config.batch_size);
        self.output.send(items)
    }
}

/// Creates a batch aggregation pipeline.
/// 
/// Returns a tuple of:
/// - Sender for individual FileEntry items
/// - Receiver for batched Vec<FileEntry>
/// - JoinHandle for the aggregator thread
pub fn create_batch_pipeline(
    config: BatchConfig,
) -> (
    Sender<FileEntry>,
    Receiver<Vec<FileEntry>>,
    std::thread::JoinHandle<usize>,
) {
    let (entry_tx, entry_rx) = flume::unbounded();
    let (batch_tx, batch_rx) = flume::unbounded();

    let aggregator = BatchAggregator::new(entry_rx, batch_tx, config);
    let handle = std::thread::spawn(move || aggregator.run());

    (entry_tx, batch_rx, handle)
}

/// Calculates the maximum number of batches for N items.
/// 
/// Formula: ceil(N / batch_size) + 1 (accounting for time-based flushes)
pub fn max_batches_for_items(item_count: usize, batch_size: usize) -> usize {
    if batch_size == 0 {
        return 0;
    }
    let size_based = (item_count + batch_size - 1) / batch_size; // ceil division
    size_based + 1 // +1 for potential time-based flush
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_batch_aggregator_size_threshold() {
        let config = BatchConfig {
            batch_size: 10,
            flush_interval: Duration::from_secs(10), // Long timeout to test size-based flush
        };

        let (entry_tx, batch_rx, handle) = create_batch_pipeline(config);

        // Send exactly 10 items (should trigger one batch)
        for i in 0..10 {
            let entry = FileEntry::new(
                format!("file_{}.txt", i),
                std::path::PathBuf::from(format!("/file_{}.txt", i)),
                false,
                100,
                std::time::SystemTime::UNIX_EPOCH,
            );
            entry_tx.send(entry).unwrap();
        }

        // Should receive a batch of 10
        let batch = batch_rx.recv_timeout(Duration::from_millis(100)).unwrap();
        assert_eq!(batch.len(), 10);

        drop(entry_tx);
        let total = handle.join().unwrap();
        assert_eq!(total, 10);
    }

    #[test]
    fn test_batch_aggregator_time_threshold() {
        let config = BatchConfig {
            batch_size: 100,
            flush_interval: Duration::from_millis(50),
        };

        let (entry_tx, batch_rx, handle) = create_batch_pipeline(config);

        // Send 5 items (less than batch size)
        for i in 0..5 {
            let entry = FileEntry::new(
                format!("file_{}.txt", i),
                std::path::PathBuf::from(format!("/file_{}.txt", i)),
                false,
                100,
                std::time::SystemTime::UNIX_EPOCH,
            );
            entry_tx.send(entry).unwrap();
        }

        // Wait for time-based flush
        thread::sleep(Duration::from_millis(100));

        // Should receive a batch of 5 due to time threshold
        let batch = batch_rx.recv_timeout(Duration::from_millis(100)).unwrap();
        assert_eq!(batch.len(), 5);

        drop(entry_tx);
        let total = handle.join().unwrap();
        assert_eq!(total, 5);
    }

    #[test]
    fn test_batch_aggregator_final_flush() {
        let config = BatchConfig {
            batch_size: 100,
            flush_interval: Duration::from_secs(10),
        };

        let (entry_tx, batch_rx, handle) = create_batch_pipeline(config);

        // Send 7 items
        for i in 0..7 {
            let entry = FileEntry::new(
                format!("file_{}.txt", i),
                std::path::PathBuf::from(format!("/file_{}.txt", i)),
                false,
                100,
                std::time::SystemTime::UNIX_EPOCH,
            );
            entry_tx.send(entry).unwrap();
        }

        // Close the sender to trigger final flush
        drop(entry_tx);

        // Should receive remaining items
        let batch = batch_rx.recv_timeout(Duration::from_millis(100)).unwrap();
        assert_eq!(batch.len(), 7);

        let total = handle.join().unwrap();
        assert_eq!(total, 7);
    }

    #[test]
    fn test_max_batches_calculation() {
        // 100 items with batch size 100 = 1 batch + 1 potential time flush = 2
        assert_eq!(max_batches_for_items(100, 100), 2);

        // 150 items with batch size 100 = 2 batches + 1 = 3
        assert_eq!(max_batches_for_items(150, 100), 3);

        // 0 items = 0 + 1 = 1
        assert_eq!(max_batches_for_items(0, 100), 1);

        // 99 items with batch size 100 = 1 + 1 = 2
        assert_eq!(max_batches_for_items(99, 100), 2);

        // Edge case: batch size 0
        assert_eq!(max_batches_for_items(100, 0), 0);
    }

    #[test]
    fn test_multiple_batches() {
        let config = BatchConfig {
            batch_size: 10,
            flush_interval: Duration::from_secs(10),
        };

        let (entry_tx, batch_rx, handle) = create_batch_pipeline(config);

        // Send 25 items (should produce 2 full batches + 1 partial)
        for i in 0..25 {
            let entry = FileEntry::new(
                format!("file_{}.txt", i),
                std::path::PathBuf::from(format!("/file_{}.txt", i)),
                false,
                100,
                std::time::SystemTime::UNIX_EPOCH,
            );
            entry_tx.send(entry).unwrap();
        }

        // First batch of 10
        let batch1 = batch_rx.recv_timeout(Duration::from_millis(100)).unwrap();
        assert_eq!(batch1.len(), 10);

        // Second batch of 10
        let batch2 = batch_rx.recv_timeout(Duration::from_millis(100)).unwrap();
        assert_eq!(batch2.len(), 10);

        // Close sender to flush remaining
        drop(entry_tx);

        // Final batch of 5
        let batch3 = batch_rx.recv_timeout(Duration::from_millis(100)).unwrap();
        assert_eq!(batch3.len(), 5);

        let total = handle.join().unwrap();
        assert_eq!(total, 25);
    }

    /// Helper to create a FileEntry for testing
    fn make_test_entry(index: usize) -> FileEntry {
        FileEntry::new(
            format!("file_{}.txt", index),
            std::path::PathBuf::from(format!("/file_{}.txt", index)),
            false,
            100,
            std::time::SystemTime::UNIX_EPOCH,
        )
    }

    proptest! {
        /// **Feature: file-explorer-core, Property 2: Batch Size Bounds**
        /// **Validates: Requirements 1.3, 3.3**
        /// 
        /// For any stream of N file entries from directory traversal, the number of
        /// batch updates delivered to the UI SHALL be at most ceil(N / 100) + 1
        /// (accounting for time-based flushes).
        #[test]
        fn prop_batch_size_bounds(
            item_count in 0usize..1000,
            batch_size in 1usize..200
        ) {
            // Use a very long flush interval to test size-based batching only
            let config = BatchConfig {
                batch_size,
                flush_interval: Duration::from_secs(60),
            };

            let (entry_tx, batch_rx, handle) = create_batch_pipeline(config);

            // Send items
            for i in 0..item_count {
                entry_tx.send(make_test_entry(i)).unwrap();
            }

            // Close sender to trigger final flush
            drop(entry_tx);

            // Collect all batches
            let mut batch_count = 0;
            let mut total_items = 0;
            while let Ok(batch) = batch_rx.recv_timeout(Duration::from_millis(100)) {
                batch_count += 1;
                total_items += batch.len();
            }

            // Wait for aggregator to finish
            let processed = handle.join().unwrap();

            // Verify all items were processed
            prop_assert_eq!(processed, item_count);
            prop_assert_eq!(total_items, item_count);

            // Verify batch count is within bounds: ceil(N / batch_size) + 1
            let max_batches = max_batches_for_items(item_count, batch_size);
            prop_assert!(
                batch_count <= max_batches,
                "batch_count {} exceeded max_batches {} for {} items with batch_size {}",
                batch_count, max_batches, item_count, batch_size
            );
        }
    }
}
