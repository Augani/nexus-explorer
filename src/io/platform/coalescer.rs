use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::models::FsEvent;
use super::watcher::DEFAULT_COALESCE_WINDOW;

/// Event coalescer that debounces rapid file system events on the same path.
/// 
/// When multiple events occur on the same path within the coalescing window,
/// only the most recent event is emitted after the window expires. This prevents
/// update storms from rapid successive changes (e.g., during file saves).
pub struct EventCoalescer {
    pending_events: HashMap<PathBuf, PendingEvent>,
    coalesce_window: Duration,
}

struct PendingEvent {
    event: FsEvent,
    timestamp: Instant,
    count: usize,
}

impl EventCoalescer {
    /// Creates a new EventCoalescer with the default coalescing window (50ms).
    pub fn new() -> Self {
        Self {
            pending_events: HashMap::new(),
            coalesce_window: DEFAULT_COALESCE_WINDOW,
        }
    }

    /// Creates a new EventCoalescer with a custom coalescing window.
    pub fn with_window(window: Duration) -> Self {
        Self {
            pending_events: HashMap::new(),
            coalesce_window: window,
        }
    }

    /// Returns the current coalescing window duration.
    pub fn coalesce_window(&self) -> Duration {
        self.coalesce_window
    }

    /// Sets the coalescing window duration.
    pub fn set_coalesce_window(&mut self, window: Duration) {
        self.coalesce_window = window;
    }

    /// Adds raw events to the coalescer.
    /// 
    /// Events on the same path will be merged, with the most recent event
    /// replacing any previous pending event for that path.
    pub fn add_events(&mut self, events: Vec<FsEvent>) {
        let now = Instant::now();
        
        for event in events {
            let path = event_path(&event);
            
            if let Some(pending) = self.pending_events.get_mut(&path) {
                pending.event = event;
                pending.timestamp = now;
                pending.count += 1;
            } else {
                self.pending_events.insert(path, PendingEvent {
                    event,
                    timestamp: now,
                    count: 1,
                });
            }
        }
    }

    /// Polls for events that have passed the coalescing window.
    /// 
    /// Returns events whose coalescing window has expired, removing them
    /// from the pending set.
    pub fn poll_ready(&mut self) -> Vec<FsEvent> {
        let now = Instant::now();
        let mut ready_events = Vec::new();
        let mut paths_to_remove = Vec::new();

        for (path, pending) in &self.pending_events {
            if now.duration_since(pending.timestamp) >= self.coalesce_window {
                ready_events.push(pending.event.clone());
                paths_to_remove.push(path.clone());
            }
        }

        for path in paths_to_remove {
            self.pending_events.remove(&path);
        }

        ready_events
    }

    /// Forces all pending events to be emitted immediately, regardless of timing.
    /// 
    /// Useful for cleanup or when immediate processing is required.
    pub fn flush_all(&mut self) -> Vec<FsEvent> {
        let events: Vec<FsEvent> = self.pending_events
            .drain()
            .map(|(_, pending)| pending.event)
            .collect();
        events
    }

    /// Returns the number of pending events.
    pub fn pending_count(&self) -> usize {
        self.pending_events.len()
    }

    /// Returns the total number of raw events that were coalesced.
    /// 
    /// This is the sum of all event counts for pending events.
    pub fn total_coalesced_count(&self) -> usize {
        self.pending_events.values().map(|p| p.count).sum()
    }

    /// Clears all pending events without emitting them.
    pub fn clear(&mut self) {
        self.pending_events.clear();
    }
}

impl Default for EventCoalescer {
    fn default() -> Self {
        Self::new()
    }
}

fn event_path(event: &FsEvent) -> PathBuf {
    match event {
        FsEvent::Created(p) | FsEvent::Modified(p) | FsEvent::Deleted(p) => p.clone(),
        FsEvent::Renamed { to, .. } => to.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use proptest::prelude::*;

    #[test]
    fn test_coalescer_merges_events_on_same_path() {
        let mut coalescer = EventCoalescer::with_window(Duration::from_millis(10));
        let path = PathBuf::from("/test/file.txt");
        
        coalescer.add_events(vec![
            FsEvent::Created(path.clone()),
            FsEvent::Modified(path.clone()),
            FsEvent::Modified(path.clone()),
        ]);
        
        assert_eq!(coalescer.pending_count(), 1);
        assert_eq!(coalescer.total_coalesced_count(), 3);
    }

    #[test]
    fn test_coalescer_keeps_separate_paths() {
        let mut coalescer = EventCoalescer::with_window(Duration::from_millis(10));
        
        coalescer.add_events(vec![
            FsEvent::Created(PathBuf::from("/test/file1.txt")),
            FsEvent::Created(PathBuf::from("/test/file2.txt")),
        ]);
        
        assert_eq!(coalescer.pending_count(), 2);
    }

    #[test]
    fn test_coalescer_emits_after_window() {
        let mut coalescer = EventCoalescer::with_window(Duration::from_millis(50));
        let path = PathBuf::from("/test/file.txt");
        
        coalescer.add_events(vec![FsEvent::Created(path.clone())]);
        
        // Immediately after adding, events should still be pending
        let ready = coalescer.poll_ready();
        assert!(ready.is_empty(), "Events should not be ready immediately");
        
        // Wait for the coalescing window to pass
        sleep(Duration::from_millis(60));
        
        let ready = coalescer.poll_ready();
        assert_eq!(ready.len(), 1);
        assert_eq!(coalescer.pending_count(), 0);
    }

    #[test]
    fn test_coalescer_flush_all() {
        let mut coalescer = EventCoalescer::with_window(Duration::from_secs(60));
        
        coalescer.add_events(vec![
            FsEvent::Created(PathBuf::from("/test/file1.txt")),
            FsEvent::Created(PathBuf::from("/test/file2.txt")),
        ]);
        
        let flushed = coalescer.flush_all();
        assert_eq!(flushed.len(), 2);
        assert_eq!(coalescer.pending_count(), 0);
    }

    fn arb_fs_event(path: PathBuf) -> impl Strategy<Value = FsEvent> {
        prop_oneof![
            Just(FsEvent::Created(path.clone())),
            Just(FsEvent::Modified(path.clone())),
            Just(FsEvent::Deleted(path.clone())),
        ]
    }

    fn arb_event_sequence_same_path() -> impl Strategy<Value = (PathBuf, Vec<FsEvent>)> {
        "[a-z]{1,10}".prop_flat_map(|filename| {
            let path = PathBuf::from(format!("/test/{}.txt", filename));
            let path_clone = path.clone();
            (2..50usize).prop_flat_map(move |count| {
                let path_inner = path_clone.clone();
                proptest::collection::vec(arb_fs_event(path_inner.clone()), count)
                    .prop_map(move |events| (path_inner.clone(), events))
            })
        })
    }

    // **Feature: file-explorer-core, Property 17: Event Coalescing**
    // **Validates: Requirements 6.5**
    //
    // *For any* sequence of N events on the same path within a coalescing window,
    // the number of UI updates triggered SHALL be less than N.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_event_coalescing_reduces_updates((_path, events) in arb_event_sequence_same_path()) {
            let n = events.len();
            prop_assume!(n >= 2);
            
            let mut coalescer = EventCoalescer::with_window(Duration::from_secs(60));
            
            coalescer.add_events(events);
            
            let flushed = coalescer.flush_all();
            
            prop_assert!(
                flushed.len() < n,
                "Expected fewer than {} events after coalescing, got {}",
                n,
                flushed.len()
            );
            
            prop_assert_eq!(
                flushed.len(),
                1,
                "All events on same path should coalesce to exactly 1 event"
            );
        }
    }

    // Additional property: events on different paths are not coalesced together
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]
        
        #[test]
        fn prop_different_paths_not_coalesced(
            filenames in proptest::collection::vec("[a-z]{1,10}", 2..10)
        ) {
            let unique_filenames: Vec<_> = filenames.into_iter()
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .collect();
            
            prop_assume!(unique_filenames.len() >= 2);
            
            let mut coalescer = EventCoalescer::with_window(Duration::from_secs(60));
            
            let events: Vec<FsEvent> = unique_filenames.iter()
                .map(|name| FsEvent::Created(PathBuf::from(format!("/test/{}.txt", name))))
                .collect();
            
            let num_unique_paths = unique_filenames.len();
            
            coalescer.add_events(events);
            
            let flushed = coalescer.flush_all();
            
            prop_assert_eq!(
                flushed.len(),
                num_unique_paths,
                "Events on {} different paths should produce {} events, got {}",
                num_unique_paths,
                num_unique_paths,
                flushed.len()
            );
        }
    }
}
