use std::io::{self, Read};
use std::path::Path;
use digest::Digest;
use md5::Md5;
use sha1::Sha1;
use sha2::{Sha256, Sha512};

/// Supported hash algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    Md5,
    Sha1,
    Sha256,
    Sha512,
}

impl HashAlgorithm {
    /// Get the display name for the algorithm
    pub fn display_name(&self) -> &'static str {
        match self {
            HashAlgorithm::Md5 => "MD5",
            HashAlgorithm::Sha1 => "SHA-1",
            HashAlgorithm::Sha256 => "SHA-256",
            HashAlgorithm::Sha512 => "SHA-512",
        }
    }

    /// Get the expected hash length in hex characters
    pub fn hash_length(&self) -> usize {
        match self {
            HashAlgorithm::Md5 => 32,
            HashAlgorithm::Sha1 => 40,
            HashAlgorithm::Sha256 => 64,
            HashAlgorithm::Sha512 => 128,
        }
    }

    /// Get all available algorithms
    pub fn all() -> &'static [HashAlgorithm] {
        &[
            HashAlgorithm::Md5,
            HashAlgorithm::Sha1,
            HashAlgorithm::Sha256,
            HashAlgorithm::Sha512,
        ]
    }
}

/// Progress information for hash calculation
#[derive(Debug, Clone)]
pub struct HashProgress {
    pub bytes_processed: u64,
    pub total_bytes: u64,
    pub percentage: f64,
}

impl HashProgress {
    pub fn new(bytes_processed: u64, total_bytes: u64) -> Self {
        let percentage = if total_bytes > 0 {
            (bytes_processed as f64 / total_bytes as f64) * 100.0
        } else {
            100.0
        };
        Self {
            bytes_processed,
            total_bytes,
            percentage,
        }
    }
}

/// Result of a hash calculation
#[derive(Debug, Clone)]
pub struct HashResult {
    pub algorithm: HashAlgorithm,
    pub hash: String,
}

impl HashResult {
    pub fn new(algorithm: HashAlgorithm, hash: String) -> Self {
        Self { algorithm, hash }
    }
}

/// Hash comparison result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashComparisonResult {
    Match,
    Mismatch,
    InvalidFormat,
}

impl HashComparisonResult {
    pub fn display_message(&self) -> &'static str {
        match self {
            HashComparisonResult::Match => "✓ Hashes match",
            HashComparisonResult::Mismatch => "✗ Hashes do not match",
            HashComparisonResult::InvalidFormat => "Invalid hash format",
        }
    }
}

/// Calculate hash of data using the specified algorithm
pub fn calculate_hash_bytes(data: &[u8], algorithm: HashAlgorithm) -> String {
    match algorithm {
        HashAlgorithm::Md5 => {
            let mut hasher = Md5::new();
            hasher.update(data);
            format!("{:x}", hasher.finalize())
        }
        HashAlgorithm::Sha1 => {
            let mut hasher = Sha1::new();
            hasher.update(data);
            format!("{:x}", hasher.finalize())
        }
        HashAlgorithm::Sha256 => {
            let mut hasher = Sha256::new();
            hasher.update(data);
            format!("{:x}", hasher.finalize())
        }
        HashAlgorithm::Sha512 => {
            let mut hasher = Sha512::new();
            hasher.update(data);
            format!("{:x}", hasher.finalize())
        }
    }
}

/// Calculate hash of a file synchronously
pub fn calculate_file_hash(path: &Path, algorithm: HashAlgorithm) -> io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut buffer = vec![0u8; 8192];
    
    match algorithm {
        HashAlgorithm::Md5 => {
            let mut hasher = Md5::new();
            loop {
                let bytes_read = file.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                hasher.update(&buffer[..bytes_read]);
            }
            Ok(format!("{:x}", hasher.finalize()))
        }
        HashAlgorithm::Sha1 => {
            let mut hasher = Sha1::new();
            loop {
                let bytes_read = file.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                hasher.update(&buffer[..bytes_read]);
            }
            Ok(format!("{:x}", hasher.finalize()))
        }
        HashAlgorithm::Sha256 => {
            let mut hasher = Sha256::new();
            loop {
                let bytes_read = file.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                hasher.update(&buffer[..bytes_read]);
            }
            Ok(format!("{:x}", hasher.finalize()))
        }
        HashAlgorithm::Sha512 => {
            let mut hasher = Sha512::new();
            loop {
                let bytes_read = file.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                hasher.update(&buffer[..bytes_read]);
            }
            Ok(format!("{:x}", hasher.finalize()))
        }
    }
}

/// Compare two hash strings (case-insensitive)
pub fn compare_hashes(hash1: &str, hash2: &str) -> HashComparisonResult {
    let h1 = hash1.trim();
    let h2 = hash2.trim();
    
    // Check if both are valid hex strings
    if !is_valid_hex(h1) || !is_valid_hex(h2) {
        return HashComparisonResult::InvalidFormat;
    }
    
    // Case-insensitive comparison
    if h1.eq_ignore_ascii_case(h2) {
        HashComparisonResult::Match
    } else {
        HashComparisonResult::Mismatch
    }
}

/// Check if a string is a valid hexadecimal hash
pub fn is_valid_hex(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Detect the hash algorithm from a hash string based on its length
pub fn detect_algorithm(hash: &str) -> Option<HashAlgorithm> {
    let trimmed = hash.trim();
    if !is_valid_hex(trimmed) {
        return None;
    }
    
    match trimmed.len() {
        32 => Some(HashAlgorithm::Md5),
        40 => Some(HashAlgorithm::Sha1),
        64 => Some(HashAlgorithm::Sha256),
        128 => Some(HashAlgorithm::Sha512),
        _ => None,
    }
}


/// Async hash calculation with progress reporting
pub struct AsyncHashCalculator {
    chunk_size: usize,
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Default for AsyncHashCalculator {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncHashCalculator {
    pub fn new() -> Self {
        Self {
            chunk_size: 64 * 1024, // 64KB chunks for progress reporting
            cancelled: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn with_chunk_size(mut self, size: usize) -> Self {
        self.chunk_size = size;
        self
    }

    /// Get a handle to cancel the operation
    pub fn cancel_handle(&self) -> std::sync::Arc<std::sync::atomic::AtomicBool> {
        self.cancelled.clone()
    }

    /// Cancel the ongoing operation
    pub fn cancel(&self) {
        self.cancelled.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Calculate hash asynchronously with progress callback
    pub async fn calculate_with_progress<F>(
        &self,
        path: &Path,
        algorithm: HashAlgorithm,
        progress_callback: F,
    ) -> io::Result<HashResult>
    where
        F: Fn(HashProgress) + Send + 'static,
    {
        let path = path.to_path_buf();
        let chunk_size = self.chunk_size;
        let cancelled = self.cancelled.clone();

        // Reset cancelled flag
        cancelled.store(false, std::sync::atomic::Ordering::SeqCst);

        // Run in blocking task
        tokio::task::spawn_blocking(move || {
            let file = std::fs::File::open(&path)?;
            let metadata = file.metadata()?;
            let total_bytes = metadata.len();
            let mut reader = std::io::BufReader::new(file);
            let mut buffer = vec![0u8; chunk_size];
            let mut bytes_processed: u64 = 0;

            let hash = match algorithm {
                HashAlgorithm::Md5 => {
                    let mut hasher = Md5::new();
                    loop {
                        if cancelled.load(std::sync::atomic::Ordering::SeqCst) {
                            return Err(io::Error::new(
                                io::ErrorKind::Interrupted,
                                "Hash calculation cancelled",
                            ));
                        }
                        let bytes_read = reader.read(&mut buffer)?;
                        if bytes_read == 0 {
                            break;
                        }
                        hasher.update(&buffer[..bytes_read]);
                        bytes_processed += bytes_read as u64;
                        progress_callback(HashProgress::new(bytes_processed, total_bytes));
                    }
                    format!("{:x}", hasher.finalize())
                }
                HashAlgorithm::Sha1 => {
                    let mut hasher = Sha1::new();
                    loop {
                        if cancelled.load(std::sync::atomic::Ordering::SeqCst) {
                            return Err(io::Error::new(
                                io::ErrorKind::Interrupted,
                                "Hash calculation cancelled",
                            ));
                        }
                        let bytes_read = reader.read(&mut buffer)?;
                        if bytes_read == 0 {
                            break;
                        }
                        hasher.update(&buffer[..bytes_read]);
                        bytes_processed += bytes_read as u64;
                        progress_callback(HashProgress::new(bytes_processed, total_bytes));
                    }
                    format!("{:x}", hasher.finalize())
                }
                HashAlgorithm::Sha256 => {
                    let mut hasher = Sha256::new();
                    loop {
                        if cancelled.load(std::sync::atomic::Ordering::SeqCst) {
                            return Err(io::Error::new(
                                io::ErrorKind::Interrupted,
                                "Hash calculation cancelled",
                            ));
                        }
                        let bytes_read = reader.read(&mut buffer)?;
                        if bytes_read == 0 {
                            break;
                        }
                        hasher.update(&buffer[..bytes_read]);
                        bytes_processed += bytes_read as u64;
                        progress_callback(HashProgress::new(bytes_processed, total_bytes));
                    }
                    format!("{:x}", hasher.finalize())
                }
                HashAlgorithm::Sha512 => {
                    let mut hasher = Sha512::new();
                    loop {
                        if cancelled.load(std::sync::atomic::Ordering::SeqCst) {
                            return Err(io::Error::new(
                                io::ErrorKind::Interrupted,
                                "Hash calculation cancelled",
                            ));
                        }
                        let bytes_read = reader.read(&mut buffer)?;
                        if bytes_read == 0 {
                            break;
                        }
                        hasher.update(&buffer[..bytes_read]);
                        bytes_processed += bytes_read as u64;
                        progress_callback(HashProgress::new(bytes_processed, total_bytes));
                    }
                    format!("{:x}", hasher.finalize())
                }
            };

            Ok(HashResult::new(algorithm, hash))
        })
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?
    }

    /// Calculate multiple hashes for a file
    pub async fn calculate_all_hashes<F>(
        &self,
        path: &Path,
        progress_callback: F,
    ) -> io::Result<Vec<HashResult>>
    where
        F: Fn(HashAlgorithm, HashProgress) + Send + Clone + 'static,
    {
        let mut results = Vec::new();
        
        for algorithm in HashAlgorithm::all() {
            let alg = *algorithm;
            let cb = progress_callback.clone();
            let result = self
                .calculate_with_progress(path, alg, move |p| cb(alg, p))
                .await?;
            results.push(result);
        }
        
        Ok(results)
    }
}

/// Calculate hash synchronously with progress callback (for smaller files or when async is not needed)
pub fn calculate_file_hash_with_progress<F>(
    path: &Path,
    algorithm: HashAlgorithm,
    progress_callback: F,
) -> io::Result<String>
where
    F: Fn(HashProgress),
{
    let file = std::fs::File::open(path)?;
    let metadata = file.metadata()?;
    let total_bytes = metadata.len();
    let mut reader = std::io::BufReader::new(file);
    let mut buffer = vec![0u8; 64 * 1024]; // 64KB chunks
    let mut bytes_processed: u64 = 0;

    match algorithm {
        HashAlgorithm::Md5 => {
            let mut hasher = Md5::new();
            loop {
                let bytes_read = reader.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                hasher.update(&buffer[..bytes_read]);
                bytes_processed += bytes_read as u64;
                progress_callback(HashProgress::new(bytes_processed, total_bytes));
            }
            Ok(format!("{:x}", hasher.finalize()))
        }
        HashAlgorithm::Sha1 => {
            let mut hasher = Sha1::new();
            loop {
                let bytes_read = reader.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                hasher.update(&buffer[..bytes_read]);
                bytes_processed += bytes_read as u64;
                progress_callback(HashProgress::new(bytes_processed, total_bytes));
            }
            Ok(format!("{:x}", hasher.finalize()))
        }
        HashAlgorithm::Sha256 => {
            let mut hasher = Sha256::new();
            loop {
                let bytes_read = reader.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                hasher.update(&buffer[..bytes_read]);
                bytes_processed += bytes_read as u64;
                progress_callback(HashProgress::new(bytes_processed, total_bytes));
            }
            Ok(format!("{:x}", hasher.finalize()))
        }
        HashAlgorithm::Sha512 => {
            let mut hasher = Sha512::new();
            loop {
                let bytes_read = reader.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                hasher.update(&buffer[..bytes_read]);
                bytes_processed += bytes_read as u64;
                progress_callback(HashProgress::new(bytes_processed, total_bytes));
            }
            Ok(format!("{:x}", hasher.finalize()))
        }
    }
}
