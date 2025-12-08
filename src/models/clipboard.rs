use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use flume::{Receiver, Sender};

/// Clipboard operation type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardOperation {
    Copy { paths: Vec<PathBuf> },
    Cut { paths: Vec<PathBuf> },
}

impl ClipboardOperation {
    pub fn paths(&self) -> &[PathBuf] {
        match self {
            ClipboardOperation::Copy { paths } => paths,
            ClipboardOperation::Cut { paths } => paths,
        }
    }

    pub fn is_cut(&self) -> bool {
        matches!(self, ClipboardOperation::Cut { .. })
    }

    pub fn is_copy(&self) -> bool {
        matches!(self, ClipboardOperation::Copy { .. })
    }
}

/// Progress information for paste operations
#[derive(Debug, Clone, Default)]
pub struct PasteProgress {
    pub current_file: PathBuf,
    pub current_file_progress: f64,
    pub total_files: usize,
    pub completed_files: usize,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub speed_bytes_per_sec: u64,
    pub estimated_remaining: Duration,
}

impl PasteProgress {
    pub fn new(total_files: usize, total_bytes: u64) -> Self {
        Self {
            current_file: PathBuf::new(),
            current_file_progress: 0.0,
            total_files,
            completed_files: 0,
            bytes_transferred: 0,
            total_bytes,
            speed_bytes_per_sec: 0,
            estimated_remaining: Duration::ZERO,
        }
    }

    pub fn percentage(&self) -> f64 {
        if self.total_bytes == 0 {
            if self.total_files == 0 {
                return 100.0;
            }
            return (self.completed_files as f64 / self.total_files as f64) * 100.0;
        }
        (self.bytes_transferred as f64 / self.total_bytes as f64) * 100.0
    }
}


/// Conflict resolution options for paste operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictResolution {
    Skip,
    Replace,
    KeepBoth,
    ReplaceIfNewer,
    ReplaceIfLarger,
}

/// Result of a paste operation
#[derive(Debug, Clone)]
pub struct PasteResult {
    pub successful_files: Vec<PathBuf>,
    pub skipped_files: Vec<PathBuf>,
    pub failed_files: Vec<(PathBuf, String)>,
    pub total_bytes_transferred: u64,
    pub duration: Duration,
}

impl PasteResult {
    pub fn new() -> Self {
        Self {
            successful_files: Vec::new(),
            skipped_files: Vec::new(),
            failed_files: Vec::new(),
            total_bytes_transferred: 0,
            duration: Duration::ZERO,
        }
    }

    pub fn is_success(&self) -> bool {
        self.failed_files.is_empty()
    }

    pub fn total_processed(&self) -> usize {
        self.successful_files.len() + self.skipped_files.len() + self.failed_files.len()
    }
}

impl Default for PasteResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Clipboard entry for history tracking
#[derive(Debug, Clone)]
pub struct ClipboardEntry {
    pub operation: ClipboardOperation,
    pub timestamp: Instant,
}

impl ClipboardEntry {
    pub fn new(operation: ClipboardOperation) -> Self {
        Self {
            operation,
            timestamp: Instant::now(),
        }
    }
}

/// Cancellation token for paste operations
#[derive(Debug, Clone)]
pub struct PasteCancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl PasteCancellationToken {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

impl Default for PasteCancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Progress update messages for paste operations
#[derive(Debug, Clone)]
pub enum PasteProgressUpdate {
    Started { total_files: usize, total_bytes: u64 },
    FileStarted { file: PathBuf, file_size: u64 },
    BytesTransferred { bytes: u64 },
    FileCompleted { file: PathBuf },
    FileSkipped { file: PathBuf, reason: String },
    FileFailed { file: PathBuf, error: String },
    Completed { result: PasteResult },
    Cancelled { partial_result: PasteResult },
    ConflictDetected { source: PathBuf, destination: PathBuf },
}

/// Maximum clipboard history size
const MAX_CLIPBOARD_HISTORY: usize = 10;


/// Manages clipboard operations with progress tracking
pub struct ClipboardManager {
    operation: Option<ClipboardOperation>,
    history: VecDeque<ClipboardEntry>,
    active_paste: Option<PasteCancellationToken>,
    progress_sender: Option<Sender<PasteProgressUpdate>>,
    progress_receiver: Option<Receiver<PasteProgressUpdate>>,
}

impl ClipboardManager {
    pub fn new() -> Self {
        Self {
            operation: None,
            history: VecDeque::new(),
            active_paste: None,
            progress_sender: None,
            progress_receiver: None,
        }
    }

    /// Copy files to clipboard
    pub fn copy(&mut self, paths: Vec<PathBuf>) {
        let operation = ClipboardOperation::Copy { paths };
        self.set_operation(operation);
    }

    /// Cut files to clipboard
    pub fn cut(&mut self, paths: Vec<PathBuf>) {
        let operation = ClipboardOperation::Cut { paths };
        self.set_operation(operation);
    }

    fn set_operation(&mut self, operation: ClipboardOperation) {
        // Add current operation to history before replacing
        if let Some(current) = self.operation.take() {
            self.history.push_front(ClipboardEntry::new(current));
            while self.history.len() > MAX_CLIPBOARD_HISTORY {
                self.history.pop_back();
            }
        }
        self.operation = Some(operation);
    }

    /// Check if clipboard has content
    pub fn has_content(&self) -> bool {
        self.operation.is_some()
    }

    /// Get clipboard operation type
    pub fn operation_type(&self) -> Option<&ClipboardOperation> {
        self.operation.as_ref()
    }

    /// Get the paths in the clipboard
    pub fn paths(&self) -> Option<&[PathBuf]> {
        self.operation.as_ref().map(|op| op.paths())
    }

    /// Check if the current operation is a cut
    pub fn is_cut(&self) -> bool {
        self.operation.as_ref().map_or(false, |op| op.is_cut())
    }

    /// Check if the current operation is a copy
    pub fn is_copy(&self) -> bool {
        self.operation.as_ref().map_or(false, |op| op.is_copy())
    }

    /// Clear the clipboard
    pub fn clear(&mut self) {
        if let Some(current) = self.operation.take() {
            self.history.push_front(ClipboardEntry::new(current));
            while self.history.len() > MAX_CLIPBOARD_HISTORY {
                self.history.pop_back();
            }
        }
    }

    /// Get clipboard history
    pub fn history(&self) -> &VecDeque<ClipboardEntry> {
        &self.history
    }

    /// Check if a path is in the clipboard (for visual feedback on cut items)
    pub fn contains_path(&self, path: &PathBuf) -> bool {
        self.operation
            .as_ref()
            .map_or(false, |op| op.paths().contains(path))
    }

    /// Check if a path is cut (for reduced opacity display)
    pub fn is_path_cut(&self, path: &PathBuf) -> bool {
        match &self.operation {
            Some(ClipboardOperation::Cut { paths }) => paths.contains(path),
            _ => false,
        }
    }

    /// Setup progress channels for paste operation
    pub fn setup_progress_channels(&mut self) -> Receiver<PasteProgressUpdate> {
        let (tx, rx) = flume::unbounded();
        self.progress_sender = Some(tx);
        self.progress_receiver = Some(rx.clone());
        rx
    }

    /// Get progress sender for paste operation
    pub fn progress_sender(&self) -> Option<Sender<PasteProgressUpdate>> {
        self.progress_sender.clone()
    }

    /// Start a paste operation and return cancellation token
    pub fn start_paste(&mut self) -> PasteCancellationToken {
        let token = PasteCancellationToken::new();
        self.active_paste = Some(token.clone());
        token
    }

    /// Cancel the active paste operation
    pub fn cancel_paste(&mut self) {
        if let Some(token) = &self.active_paste {
            token.cancel();
        }
    }

    /// Check if a paste operation is active
    pub fn is_paste_active(&self) -> bool {
        self.active_paste
            .as_ref()
            .map_or(false, |t| !t.is_cancelled())
    }

    /// Complete the paste operation (clears cut items from clipboard)
    pub fn complete_paste(&mut self, was_cut: bool) {
        self.active_paste = None;
        if was_cut {
            // Clear clipboard after successful cut-paste
            self.operation = None;
        }
    }

    /// Get the number of items in clipboard
    pub fn item_count(&self) -> usize {
        self.operation.as_ref().map_or(0, |op| op.paths().len())
    }
}

impl Default for ClipboardManager {
    fn default() -> Self {
        Self::new()
    }
}


/// Paste executor for handling file copy/move operations with progress
pub struct PasteExecutor {
    cancellation_token: PasteCancellationToken,
    progress_sender: Sender<PasteProgressUpdate>,
}

impl PasteExecutor {
    pub fn new(
        cancellation_token: PasteCancellationToken,
        progress_sender: Sender<PasteProgressUpdate>,
    ) -> Self {
        Self {
            cancellation_token,
            progress_sender,
        }
    }

    /// Execute paste operation with progress tracking
    pub fn execute(
        &self,
        sources: &[PathBuf],
        destination: &PathBuf,
        is_cut: bool,
        conflict_handler: impl Fn(&PathBuf, &PathBuf) -> ConflictResolution,
    ) -> Result<PasteResult, String> {
        let start_time = Instant::now();
        let mut result = PasteResult::new();

        // Calculate total size and file count
        let (total_files, total_bytes) = self.calculate_totals(sources);
        
        let _ = self.progress_sender.send(PasteProgressUpdate::Started {
            total_files,
            total_bytes,
        });

        let mut bytes_transferred: u64 = 0;
        let mut completed_files: usize = 0;
        let mut speed_tracker = SpeedTracker::new();

        for source in sources {
            if self.cancellation_token.is_cancelled() {
                result.duration = start_time.elapsed();
                let _ = self.progress_sender.send(PasteProgressUpdate::Cancelled {
                    partial_result: result.clone(),
                });
                return Ok(result);
            }

            let dest_path = self.compute_destination(source, destination);

            // Check for conflicts
            if dest_path.exists() {
                let _ = self.progress_sender.send(PasteProgressUpdate::ConflictDetected {
                    source: source.clone(),
                    destination: dest_path.clone(),
                });

                let resolution = conflict_handler(source, &dest_path);
                match resolution {
                    ConflictResolution::Skip => {
                        result.skipped_files.push(source.clone());
                        let _ = self.progress_sender.send(PasteProgressUpdate::FileSkipped {
                            file: source.clone(),
                            reason: "User chose to skip".to_string(),
                        });
                        completed_files += 1;
                        continue;
                    }
                    ConflictResolution::KeepBoth => {
                        // Generate unique name
                        let unique_dest = self.generate_unique_name(&dest_path);
                        match self.copy_with_progress(
                            source,
                            &unique_dest,
                            &mut bytes_transferred,
                            total_bytes,
                            &mut speed_tracker,
                        ) {
                            Ok(bytes) => {
                                result.successful_files.push(unique_dest);
                                result.total_bytes_transferred += bytes;
                            }
                            Err(e) => {
                                result.failed_files.push((source.clone(), e.clone()));
                                let _ = self.progress_sender.send(PasteProgressUpdate::FileFailed {
                                    file: source.clone(),
                                    error: e,
                                });
                            }
                        }
                    }
                    ConflictResolution::Replace
                    | ConflictResolution::ReplaceIfNewer
                    | ConflictResolution::ReplaceIfLarger => {
                        // Check conditions for conditional replacements
                        let should_replace = match resolution {
                            ConflictResolution::ReplaceIfNewer => {
                                self.is_source_newer(source, &dest_path)
                            }
                            ConflictResolution::ReplaceIfLarger => {
                                self.is_source_larger(source, &dest_path)
                            }
                            _ => true,
                        };

                        if should_replace {
                            // Remove existing file first
                            if dest_path.is_dir() {
                                let _ = std::fs::remove_dir_all(&dest_path);
                            } else {
                                let _ = std::fs::remove_file(&dest_path);
                            }

                            match self.copy_with_progress(
                                source,
                                &dest_path,
                                &mut bytes_transferred,
                                total_bytes,
                                &mut speed_tracker,
                            ) {
                                Ok(bytes) => {
                                    result.successful_files.push(dest_path.clone());
                                    result.total_bytes_transferred += bytes;
                                }
                                Err(e) => {
                                    result.failed_files.push((source.clone(), e.clone()));
                                    let _ = self.progress_sender.send(PasteProgressUpdate::FileFailed {
                                        file: source.clone(),
                                        error: e,
                                    });
                                }
                            }
                        } else {
                            result.skipped_files.push(source.clone());
                            let _ = self.progress_sender.send(PasteProgressUpdate::FileSkipped {
                                file: source.clone(),
                                reason: "Condition not met".to_string(),
                            });
                        }
                    }
                }
            } else {
                // No conflict, proceed with copy
                match self.copy_with_progress(
                    source,
                    &dest_path,
                    &mut bytes_transferred,
                    total_bytes,
                    &mut speed_tracker,
                ) {
                    Ok(bytes) => {
                        result.successful_files.push(dest_path);
                        result.total_bytes_transferred += bytes;
                    }
                    Err(e) => {
                        result.failed_files.push((source.clone(), e.clone()));
                        let _ = self.progress_sender.send(PasteProgressUpdate::FileFailed {
                            file: source.clone(),
                            error: e,
                        });
                    }
                }
            }

            completed_files += 1;
            let _ = self.progress_sender.send(PasteProgressUpdate::FileCompleted {
                file: source.clone(),
            });
        }

        // If this was a cut operation and successful, delete source files
        if is_cut && result.failed_files.is_empty() {
            for source in sources {
                if !result.skipped_files.contains(source) {
                    if source.is_dir() {
                        let _ = std::fs::remove_dir_all(source);
                    } else {
                        let _ = std::fs::remove_file(source);
                    }
                }
            }
        }

        result.duration = start_time.elapsed();
        let _ = self.progress_sender.send(PasteProgressUpdate::Completed {
            result: result.clone(),
        });

        Ok(result)
    }

    fn calculate_totals(&self, sources: &[PathBuf]) -> (usize, u64) {
        let mut total_files = 0usize;
        let mut total_bytes = 0u64;

        for source in sources {
            if source.is_dir() {
                // Use recursive directory traversal
                self.count_dir_contents(source, &mut total_files, &mut total_bytes);
            } else if let Ok(meta) = source.metadata() {
                total_files += 1;
                total_bytes += meta.len();
            }
        }

        (total_files, total_bytes)
    }

    fn count_dir_contents(&self, dir: &PathBuf, total_files: &mut usize, total_bytes: &mut u64) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    self.count_dir_contents(&path, total_files, total_bytes);
                } else if let Ok(meta) = entry.metadata() {
                    *total_files += 1;
                    *total_bytes += meta.len();
                }
            }
        }
    }

    fn compute_destination(&self, source: &PathBuf, destination: &PathBuf) -> PathBuf {
        if let Some(file_name) = source.file_name() {
            destination.join(file_name)
        } else {
            destination.clone()
        }
    }

    fn generate_unique_name(&self, path: &PathBuf) -> PathBuf {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
        let ext = path.extension().and_then(|e| e.to_str());
        let parent = path.parent().unwrap_or(path);

        let mut counter = 1;
        loop {
            let new_name = if let Some(ext) = ext {
                format!("{} ({}). {}", stem, counter, ext)
            } else {
                format!("{} ({})", stem, counter)
            };
            let new_path = parent.join(&new_name);
            if !new_path.exists() {
                return new_path;
            }
            counter += 1;
        }
    }

    fn is_source_newer(&self, source: &PathBuf, dest: &PathBuf) -> bool {
        let source_time = source.metadata().and_then(|m| m.modified()).ok();
        let dest_time = dest.metadata().and_then(|m| m.modified()).ok();
        
        match (source_time, dest_time) {
            (Some(s), Some(d)) => s > d,
            _ => true, // Default to replacing if we can't determine
        }
    }

    fn is_source_larger(&self, source: &PathBuf, dest: &PathBuf) -> bool {
        let source_size = source.metadata().map(|m| m.len()).ok();
        let dest_size = dest.metadata().map(|m| m.len()).ok();
        
        match (source_size, dest_size) {
            (Some(s), Some(d)) => s > d,
            _ => true,
        }
    }

    fn copy_with_progress(
        &self,
        source: &PathBuf,
        dest: &PathBuf,
        bytes_transferred: &mut u64,
        total_bytes: u64,
        speed_tracker: &mut SpeedTracker,
    ) -> Result<u64, String> {
        if source.is_dir() {
            self.copy_dir_with_progress(source, dest, bytes_transferred, total_bytes, speed_tracker)
        } else {
            self.copy_file_with_progress(source, dest, bytes_transferred, total_bytes, speed_tracker)
        }
    }

    fn copy_file_with_progress(
        &self,
        source: &PathBuf,
        dest: &PathBuf,
        bytes_transferred: &mut u64,
        total_bytes: u64,
        speed_tracker: &mut SpeedTracker,
    ) -> Result<u64, String> {
        use std::io::{Read, Write};

        // Ensure parent directory exists
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }

        let file_size = source.metadata().map(|m| m.len()).unwrap_or(0);
        
        let _ = self.progress_sender.send(PasteProgressUpdate::FileStarted {
            file: source.clone(),
            file_size,
        });

        let mut src_file = std::fs::File::open(source)
            .map_err(|e| format!("Failed to open source: {}", e))?;
        let mut dst_file = std::fs::File::create(dest)
            .map_err(|e| format!("Failed to create destination: {}", e))?;

        let mut buffer = vec![0u8; 64 * 1024]; // 64KB buffer
        let mut file_bytes_copied = 0u64;

        loop {
            if self.cancellation_token.is_cancelled() {
                // Clean up partial file
                drop(dst_file);
                let _ = std::fs::remove_file(dest);
                return Err("Operation cancelled".to_string());
            }

            let bytes_read = src_file.read(&mut buffer)
                .map_err(|e| format!("Read error: {}", e))?;
            
            if bytes_read == 0 {
                break;
            }

            dst_file.write_all(&buffer[..bytes_read])
                .map_err(|e| format!("Write error: {}", e))?;

            file_bytes_copied += bytes_read as u64;
            *bytes_transferred += bytes_read as u64;

            speed_tracker.update(bytes_read as u64);

            let _ = self.progress_sender.send(PasteProgressUpdate::BytesTransferred {
                bytes: bytes_read as u64,
            });
        }

        Ok(file_bytes_copied)
    }

    fn copy_dir_with_progress(
        &self,
        source: &PathBuf,
        dest: &PathBuf,
        bytes_transferred: &mut u64,
        total_bytes: u64,
        speed_tracker: &mut SpeedTracker,
    ) -> Result<u64, String> {
        std::fs::create_dir_all(dest)
            .map_err(|e| format!("Failed to create directory: {}", e))?;

        let mut total_copied = 0u64;

        let entries = std::fs::read_dir(source)
            .map_err(|e| format!("Failed to read directory: {}", e))?;

        for entry in entries {
            if self.cancellation_token.is_cancelled() {
                return Err("Operation cancelled".to_string());
            }

            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let src_path = entry.path();
            let dst_path = dest.join(entry.file_name());

            if src_path.is_dir() {
                total_copied += self.copy_dir_with_progress(
                    &src_path,
                    &dst_path,
                    bytes_transferred,
                    total_bytes,
                    speed_tracker,
                )?;
            } else {
                total_copied += self.copy_file_with_progress(
                    &src_path,
                    &dst_path,
                    bytes_transferred,
                    total_bytes,
                    speed_tracker,
                )?;
            }
        }

        Ok(total_copied)
    }
}

/// Tracks transfer speed for ETA calculation
struct SpeedTracker {
    start_time: Instant,
    bytes_transferred: u64,
    samples: VecDeque<(Instant, u64)>,
}

impl SpeedTracker {
    fn new() -> Self {
        Self {
            start_time: Instant::now(),
            bytes_transferred: 0,
            samples: VecDeque::with_capacity(10),
        }
    }

    fn update(&mut self, bytes: u64) {
        self.bytes_transferred += bytes;
        let now = Instant::now();
        
        self.samples.push_back((now, bytes));
        
        // Keep only last 10 samples for moving average
        while self.samples.len() > 10 {
            self.samples.pop_front();
        }
    }

    fn speed_bytes_per_sec(&self) -> u64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            (self.bytes_transferred as f64 / elapsed) as u64
        } else {
            0
        }
    }

    fn estimated_remaining(&self, remaining_bytes: u64) -> Duration {
        let speed = self.speed_bytes_per_sec();
        if speed > 0 {
            Duration::from_secs(remaining_bytes / speed)
        } else {
            Duration::ZERO
        }
    }
}
