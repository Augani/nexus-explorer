use std::time::{Duration, Instant};

use flume::{Receiver, Sender};

use crate::models::FileEntry;

/
pub const DEFAULT_BATCH_SIZE: usize = 100;

/
pub const DEFAULT_FLUSH_INTERVAL: Duration = Duration::from_millis(16);

/
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

/
/
/
/
/
/
/
/
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

    /
    /
    pub fn run(self) -> usize {
        let mut batch = Vec::with_capacity(self.config.batch_size);
        let mut last_flush = Instant::now();
        let mut total_items = 0;

        loop {
            let elapsed = last_flush.elapsed();
            let remaining = self.config.flush_interval.saturating_sub(elapsed);

            match self.input.recv_timeout(remaining) {
                Ok(entry) => {
                    batch.push(entry);
                    total_items += 1;

                    if batch.len() >= self.config.batch_size {
                        if self.flush_batch(&mut batch).is_err() {
                            break;
                        }
                        last_flush = Instant::now();
                    }
                }
                Err(flume::RecvTimeoutError::Timeout) => {
                    if !batch.is_empty() {
                        if self.flush_batch(&mut batch).is_err() {
                            break;
                        }
                    }
                    last_flush = Instant::now();
                }
                Err(flume::RecvTimeoutError::Disconnected) => {
                    if !batch.is_empty() {
                        let _ = self.flush_batch(&mut batch);
                    }
                    break;
                }
            }
        }

        total_items
    }

    fn flush_batch(
        &self,
        batch: &mut Vec<FileEntry>,
    ) -> Result<(), flume::SendError<Vec<FileEntry>>> {
        let items = std::mem::take(batch);
        *batch = Vec::with_capacity(self.config.batch_size);
        self.output.send(items)
    }
}

/
/
/
/
/
/
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

/
/
/
pub fn max_batches_for_items(item_count: usize, batch_size: usize) -> usize {
    if batch_size == 0 {
        return 0;
    }
    let size_based = (item_count + batch_size - 1) / batch_size;
    size_based + 1
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
            flush_interval: Duration::from_secs(10),
        };

        let (entry_tx, batch_rx, handle) = create_batch_pipeline(config);

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

        thread::sleep(Duration::from_millis(100));

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

        drop(entry_tx);

        let batch = batch_rx.recv_timeout(Duration::from_millis(100)).unwrap();
        assert_eq!(batch.len(), 7);

        let total = handle.join().unwrap();
        assert_eq!(total, 7);
    }

    #[test]
    fn test_max_batches_calculation() {
        assert_eq!(max_batches_for_items(100, 100), 2);

        assert_eq!(max_batches_for_items(150, 100), 3);

        assert_eq!(max_batches_for_items(0, 100), 1);

        assert_eq!(max_batches_for_items(99, 100), 2);

        assert_eq!(max_batches_for_items(100, 0), 0);
    }

    #[test]
    fn test_multiple_batches() {
        let config = BatchConfig {
            batch_size: 10,
            flush_interval: Duration::from_secs(10),
        };

        let (entry_tx, batch_rx, handle) = create_batch_pipeline(config);

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

        let batch1 = batch_rx.recv_timeout(Duration::from_millis(100)).unwrap();
        assert_eq!(batch1.len(), 10);

        let batch2 = batch_rx.recv_timeout(Duration::from_millis(100)).unwrap();
        assert_eq!(batch2.len(), 10);

        drop(entry_tx);

        let batch3 = batch_rx.recv_timeout(Duration::from_millis(100)).unwrap();
        assert_eq!(batch3.len(), 5);

        let total = handle.join().unwrap();
        assert_eq!(total, 25);
    }

    /
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
        /
        /
        /
        /
        /
        /
        #[test]
        fn prop_batch_size_bounds(
            item_count in 0usize..1000,
            batch_size in 1usize..200
        ) {
            let config = BatchConfig {
                batch_size,
                flush_interval: Duration::from_secs(60),
            };

            let (entry_tx, batch_rx, handle) = create_batch_pipeline(config);

            for i in 0..item_count {
                entry_tx.send(make_test_entry(i)).unwrap();
            }

            drop(entry_tx);

            let mut batch_count = 0;
            let mut total_items = 0;
            while let Ok(batch) = batch_rx.recv_timeout(Duration::from_millis(100)) {
                batch_count += 1;
                total_items += batch.len();
            }

            let processed = handle.join().unwrap();

            prop_assert_eq!(processed, item_count);
            prop_assert_eq!(total_items, item_count);

            let max_batches = max_batches_for_items(item_count, batch_size);
            prop_assert!(
                batch_count <= max_batches,
                "batch_count {} exceeded max_batches {} for {} items with batch_size {}",
                batch_count, max_batches, item_count, batch_size
            );
        }
    }
}
