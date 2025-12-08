use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use thiserror::Error;

/// Supported archive formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    Zip,
    TarGz,
    TarBz2,
    TarXz,
    SevenZip,
}

impl ArchiveFormat {
    pub fn from_extension(path: &Path) -> Option<Self> {
        let name = path.file_name()?.to_str()?.to_lowercase();
        
        if name.ends_with(".zip") {
            Some(ArchiveFormat::Zip)
        } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
            Some(ArchiveFormat::TarGz)
        } else if name.ends_with(".tar.bz2") || name.ends_with(".tbz2") {
            Some(ArchiveFormat::TarBz2)
        } else if name.ends_with(".tar.xz") || name.ends_with(".txz") {
            Some(ArchiveFormat::TarXz)
        } else if name.ends_with(".7z") {
            Some(ArchiveFormat::SevenZip)
        } else {
            None
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            ArchiveFormat::Zip => ".zip",
            ArchiveFormat::TarGz => ".tar.gz",
            ArchiveFormat::TarBz2 => ".tar.bz2",
            ArchiveFormat::TarXz => ".tar.xz",
            ArchiveFormat::SevenZip => ".7z",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ArchiveFormat::Zip => "ZIP Archive",
            ArchiveFormat::TarGz => "TAR.GZ Archive",
            ArchiveFormat::TarBz2 => "TAR.BZ2 Archive",
            ArchiveFormat::TarXz => "TAR.XZ Archive",
            ArchiveFormat::SevenZip => "7-Zip Archive",
        }
    }
}

/// Archive creation options
#[derive(Debug, Clone)]
pub struct CompressOptions {
    pub format: ArchiveFormat,
    pub compression_level: u8,
    pub password: Option<String>,
}

impl Default for CompressOptions {
    fn default() -> Self {
        Self {
            format: ArchiveFormat::Zip,
            compression_level: 6,
            password: None,
        }
    }
}

/// Overwrite mode for extraction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverwriteMode {
    Skip,
    Replace,
    ReplaceIfNewer,
}

/// Archive extraction options
#[derive(Debug, Clone)]
pub struct ExtractOptions {
    pub destination: PathBuf,
    pub password: Option<String>,
    pub overwrite: OverwriteMode,
}

/// Entry in an archive
#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub compressed_size: u64,
    pub modified: Option<SystemTime>,
    pub is_encrypted: bool,
}

/// Progress information for archive operations
#[derive(Debug, Clone)]
pub struct ArchiveProgress {
    pub current_file: String,
    pub current_file_index: usize,
    pub total_files: usize,
    pub bytes_processed: u64,
    pub total_bytes: u64,
    pub percentage: f64,
}

impl ArchiveProgress {
    pub fn new(total_files: usize, total_bytes: u64) -> Self {
        Self {
            current_file: String::new(),
            current_file_index: 0,
            total_files,
            bytes_processed: 0,
            total_bytes,
            percentage: 0.0,
        }
    }

    pub fn update(&mut self, file: &str, index: usize, bytes: u64) {
        self.current_file = file.to_string();
        self.current_file_index = index;
        self.bytes_processed = bytes;
        if self.total_bytes > 0 {
            self.percentage = (bytes as f64 / self.total_bytes as f64) * 100.0;
        } else if self.total_files > 0 {
            self.percentage = (index as f64 / self.total_files as f64) * 100.0;
        }
    }
}

/// Errors that can occur during archive operations
#[derive(Debug, Error)]
pub enum ArchiveError {
    #[error("Invalid archive: {0}")]
    InvalidArchive(String),

    #[error("Password required")]
    PasswordRequired,

    #[error("Wrong password")]
    WrongPassword,

    #[error("Extraction failed: {0}")]
    ExtractionFailed(String),

    #[error("Compression failed: {0}")]
    CompressionFailed(String),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("ZIP error: {0}")]
    ZipError(#[from] zip::result::ZipError),
}

/// Manages archive operations (compress/extract)
pub struct ArchiveManager {
    supported_formats: Vec<ArchiveFormat>,
}

impl Default for ArchiveManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ArchiveManager {
    pub fn new() -> Self {
        Self {
            supported_formats: vec![
                ArchiveFormat::Zip,
                ArchiveFormat::TarGz,
                ArchiveFormat::SevenZip,
            ],
        }
    }

    pub fn supported_formats(&self) -> &[ArchiveFormat] {
        &self.supported_formats
    }

    pub fn is_archive(&self, path: &Path) -> bool {
        ArchiveFormat::from_extension(path).is_some()
    }


    /// List contents of an archive without extracting
    pub fn list_contents(&self, archive_path: &Path) -> Result<Vec<ArchiveEntry>, ArchiveError> {
        let format = ArchiveFormat::from_extension(archive_path)
            .ok_or_else(|| ArchiveError::UnsupportedFormat(
                archive_path.extension().map(|e| e.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            ))?;

        match format {
            ArchiveFormat::Zip => self.list_zip_contents(archive_path),
            ArchiveFormat::TarGz => self.list_tar_gz_contents(archive_path),
            ArchiveFormat::SevenZip => self.list_7z_contents(archive_path),
            _ => Err(ArchiveError::UnsupportedFormat(format.display_name().to_string())),
        }
    }

    fn list_zip_contents(&self, archive_path: &Path) -> Result<Vec<ArchiveEntry>, ArchiveError> {
        let file = File::open(archive_path)?;
        let reader = BufReader::new(file);
        let mut archive = zip::ZipArchive::new(reader)?;
        
        let mut entries = Vec::with_capacity(archive.len());
        
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            entries.push(ArchiveEntry {
                path: file.name().to_string(),
                is_dir: file.is_dir(),
                size: file.size(),
                compressed_size: file.compressed_size(),
                modified: file.last_modified().and_then(|dt| {
                    use std::time::{Duration, UNIX_EPOCH};
                    let secs = dt.to_time()
                        .map(|t| t.unix_timestamp() as u64)
                        .unwrap_or(0);
                    if secs > 0 {
                        Some(UNIX_EPOCH + Duration::from_secs(secs))
                    } else {
                        None
                    }
                }),
                is_encrypted: file.encrypted(),
            });
        }
        
        Ok(entries)
    }

    fn list_tar_gz_contents(&self, archive_path: &Path) -> Result<Vec<ArchiveEntry>, ArchiveError> {
        let file = File::open(archive_path)?;
        let reader = BufReader::new(file);
        let decoder = GzDecoder::new(reader);
        let mut archive = tar::Archive::new(decoder);
        
        let mut entries = Vec::new();
        
        for entry_result in archive.entries()? {
            let entry = entry_result?;
            let header = entry.header();
            
            entries.push(ArchiveEntry {
                path: entry.path()?.to_string_lossy().to_string(),
                is_dir: header.entry_type().is_dir(),
                size: header.size()?,
                compressed_size: header.size()?, // TAR doesn't store compressed size per file
                modified: header.mtime().ok().map(|secs| {
                    use std::time::{Duration, UNIX_EPOCH};
                    UNIX_EPOCH + Duration::from_secs(secs)
                }),
                is_encrypted: false,
            });
        }
        
        Ok(entries)
    }

    fn list_7z_contents(&self, archive_path: &Path) -> Result<Vec<ArchiveEntry>, ArchiveError> {
        let mut entries = Vec::new();
        
        sevenz_rust::decompress_file_with_extract_fn(archive_path, ".", |entry, _, _| {
            entries.push(ArchiveEntry {
                path: entry.name().to_string(),
                is_dir: entry.is_directory(),
                size: entry.size(),
                compressed_size: entry.compressed_size,
                modified: None, // 7z-rust doesn't expose modification time easily
                is_encrypted: entry.has_stream() && entry.compressed_size > 0,
            });
            Ok(false) // Don't actually extract
        }).map_err(|e| ArchiveError::InvalidArchive(e.to_string()))?;
        
        Ok(entries)
    }

    /// Get total uncompressed size of archive contents
    pub fn get_total_uncompressed_size(&self, entries: &[ArchiveEntry]) -> u64 {
        entries.iter().filter(|e| !e.is_dir).map(|e| e.size).sum()
    }

    /// Compress files into an archive
    pub fn compress<F>(
        &self,
        paths: &[PathBuf],
        output: &Path,
        options: &CompressOptions,
        progress_callback: F,
    ) -> Result<(), ArchiveError>
    where
        F: Fn(ArchiveProgress),
    {
        match options.format {
            ArchiveFormat::Zip => self.compress_zip(paths, output, options, progress_callback),
            ArchiveFormat::TarGz => self.compress_tar_gz(paths, output, options, progress_callback),
            ArchiveFormat::SevenZip => self.compress_7z(paths, output, options, progress_callback),
            _ => Err(ArchiveError::UnsupportedFormat(options.format.display_name().to_string())),
        }
    }

    fn collect_files(&self, paths: &[PathBuf]) -> io::Result<Vec<(PathBuf, PathBuf)>> {
        let mut files = Vec::new();
        
        for path in paths {
            if path.is_file() {
                let name = path.file_name()
                    .map(|n| PathBuf::from(n))
                    .unwrap_or_else(|| path.clone());
                files.push((path.clone(), name));
            } else if path.is_dir() {
                let base_name = path.file_name()
                    .map(|n| PathBuf::from(n))
                    .unwrap_or_else(|| PathBuf::from("archive"));
                self.collect_dir_files(path, &base_name, &mut files)?;
            }
        }
        
        Ok(files)
    }

    fn collect_dir_files(
        &self,
        dir: &Path,
        base: &Path,
        files: &mut Vec<(PathBuf, PathBuf)>,
    ) -> io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let relative = base.join(entry.file_name());
            
            if path.is_file() {
                files.push((path, relative));
            } else if path.is_dir() {
                self.collect_dir_files(&path, &relative, files)?;
            }
        }
        Ok(())
    }


    fn compress_zip<F>(
        &self,
        paths: &[PathBuf],
        output: &Path,
        options: &CompressOptions,
        progress_callback: F,
    ) -> Result<(), ArchiveError>
    where
        F: Fn(ArchiveProgress),
    {
        let files = self.collect_files(paths)?;
        let total_bytes: u64 = files.iter()
            .filter_map(|(p, _)| fs::metadata(p).ok())
            .map(|m| m.len())
            .sum();
        
        let mut progress = ArchiveProgress::new(files.len(), total_bytes);
        
        let file = File::create(output)?;
        let writer = BufWriter::new(file);
        let mut zip = zip::ZipWriter::new(writer);
        
        let zip_options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .compression_level(Some(options.compression_level as i64));
        
        let mut bytes_written = 0u64;
        
        for (index, (source_path, archive_path)) in files.iter().enumerate() {
            let archive_name = archive_path.to_string_lossy();
            progress.update(&archive_name, index, bytes_written);
            progress_callback(progress.clone());
            
            zip.start_file(&*archive_name, zip_options)?;
            
            let mut source_file = File::open(source_path)?;
            let mut buffer = vec![0u8; 8192];
            
            loop {
                let bytes_read = source_file.read(&mut buffer)?;
                if bytes_read == 0 {
                    break;
                }
                zip.write_all(&buffer[..bytes_read])?;
                bytes_written += bytes_read as u64;
            }
        }
        
        zip.finish()?;
        
        progress.update("Complete", files.len(), total_bytes);
        progress.percentage = 100.0;
        progress_callback(progress);
        
        Ok(())
    }

    fn compress_tar_gz<F>(
        &self,
        paths: &[PathBuf],
        output: &Path,
        options: &CompressOptions,
        progress_callback: F,
    ) -> Result<(), ArchiveError>
    where
        F: Fn(ArchiveProgress),
    {
        let files = self.collect_files(paths)?;
        let total_bytes: u64 = files.iter()
            .filter_map(|(p, _)| fs::metadata(p).ok())
            .map(|m| m.len())
            .sum();
        
        let mut progress = ArchiveProgress::new(files.len(), total_bytes);
        
        let file = File::create(output)?;
        let writer = BufWriter::new(file);
        let level = match options.compression_level {
            0..=3 => Compression::fast(),
            4..=6 => Compression::default(),
            _ => Compression::best(),
        };
        let encoder = GzEncoder::new(writer, level);
        let mut tar = tar::Builder::new(encoder);
        
        let mut bytes_written = 0u64;
        
        for (index, (source_path, archive_path)) in files.iter().enumerate() {
            let archive_name = archive_path.to_string_lossy();
            progress.update(&archive_name, index, bytes_written);
            progress_callback(progress.clone());
            
            let metadata = fs::metadata(source_path)?;
            bytes_written += metadata.len();
            
            tar.append_path_with_name(source_path, archive_path)
                .map_err(|e| ArchiveError::CompressionFailed(e.to_string()))?;
        }
        
        tar.finish()?;
        
        progress.update("Complete", files.len(), total_bytes);
        progress.percentage = 100.0;
        progress_callback(progress);
        
        Ok(())
    }

    fn compress_7z<F>(
        &self,
        paths: &[PathBuf],
        output: &Path,
        _options: &CompressOptions,
        progress_callback: F,
    ) -> Result<(), ArchiveError>
    where
        F: Fn(ArchiveProgress),
    {
        let files = self.collect_files(paths)?;
        let total_bytes: u64 = files.iter()
            .filter_map(|(p, _)| fs::metadata(p).ok())
            .map(|m| m.len())
            .sum();
        
        let mut progress = ArchiveProgress::new(files.len(), total_bytes);
        progress_callback(progress.clone());
        
        // Create a temporary directory structure for 7z compression
        let temp_dir = std::env::temp_dir().join(format!("nexus_7z_{}", std::process::id()));
        fs::create_dir_all(&temp_dir)?;
        
        let mut bytes_copied = 0u64;
        
        for (index, (source_path, archive_path)) in files.iter().enumerate() {
            let dest_path = temp_dir.join(archive_path);
            
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }
            
            fs::copy(source_path, &dest_path)?;
            
            let metadata = fs::metadata(source_path)?;
            bytes_copied += metadata.len();
            
            progress.update(&archive_path.to_string_lossy(), index, bytes_copied);
            progress_callback(progress.clone());
        }
        
        // Compress the temp directory
        sevenz_rust::compress_to_path(&temp_dir, output)
            .map_err(|e| ArchiveError::CompressionFailed(e.to_string()))?;
        
        // Clean up temp directory
        let _ = fs::remove_dir_all(&temp_dir);
        
        progress.update("Complete", files.len(), total_bytes);
        progress.percentage = 100.0;
        progress_callback(progress);
        
        Ok(())
    }


    /// Extract an archive to a destination
    pub fn extract<F>(
        &self,
        archive_path: &Path,
        options: &ExtractOptions,
        progress_callback: F,
    ) -> Result<(), ArchiveError>
    where
        F: Fn(ArchiveProgress),
    {
        let format = ArchiveFormat::from_extension(archive_path)
            .ok_or_else(|| ArchiveError::UnsupportedFormat(
                archive_path.extension().map(|e| e.to_string_lossy().to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            ))?;

        // Create destination directory if it doesn't exist
        fs::create_dir_all(&options.destination)?;

        match format {
            ArchiveFormat::Zip => self.extract_zip(archive_path, options, progress_callback),
            ArchiveFormat::TarGz => self.extract_tar_gz(archive_path, options, progress_callback),
            ArchiveFormat::SevenZip => self.extract_7z(archive_path, options, progress_callback),
            _ => Err(ArchiveError::UnsupportedFormat(format.display_name().to_string())),
        }
    }

    fn extract_zip<F>(
        &self,
        archive_path: &Path,
        options: &ExtractOptions,
        progress_callback: F,
    ) -> Result<(), ArchiveError>
    where
        F: Fn(ArchiveProgress),
    {
        let file = File::open(archive_path)?;
        let reader = BufReader::new(file);
        let mut archive = zip::ZipArchive::new(reader)?;
        
        let total_files = archive.len();
        let mut total_bytes: u64 = 0;
        for i in 0..total_files {
            if let Ok(f) = archive.by_index(i) {
                total_bytes += f.size();
            }
        }
        
        let mut progress = ArchiveProgress::new(total_files, total_bytes);
        let mut bytes_extracted = 0u64;
        
        for i in 0..total_files {
            let mut file = archive.by_index(i)?;
            let file_name = file.name().to_string();
            
            progress.update(&file_name, i, bytes_extracted);
            progress_callback(progress.clone());
            
            // Check for password-protected files
            if file.encrypted() {
                if options.password.is_some() {
                    // Note: zip crate handles password internally when opening
                    // For now, we'll skip encrypted files if no password support
                    return Err(ArchiveError::PasswordRequired);
                } else {
                    return Err(ArchiveError::PasswordRequired);
                }
            }
            
            let out_path = options.destination.join(&file_name);
            
            // Handle overwrite mode
            if out_path.exists() {
                match options.overwrite {
                    OverwriteMode::Skip => {
                        bytes_extracted += file.size();
                        continue;
                    }
                    OverwriteMode::ReplaceIfNewer => {
                        if let (Ok(existing_meta), Some(archive_time)) = 
                            (fs::metadata(&out_path), file.last_modified()) 
                        {
                            if let Ok(existing_time) = existing_meta.modified() {
                                use std::time::{Duration, UNIX_EPOCH};
                                let archive_secs = archive_time.to_time()
                                    .map(|t| t.unix_timestamp() as u64)
                                    .unwrap_or(0);
                                let archive_system_time = UNIX_EPOCH + Duration::from_secs(archive_secs);
                                if existing_time >= archive_system_time {
                                    bytes_extracted += file.size();
                                    continue;
                                }
                            }
                        }
                    }
                    OverwriteMode::Replace => {}
                }
            }
            
            if file.is_dir() {
                fs::create_dir_all(&out_path)?;
            } else {
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                
                let mut out_file = File::create(&out_path)?;
                io::copy(&mut file, &mut out_file)?;
            }
            
            bytes_extracted += file.size();
        }
        
        progress.update("Complete", total_files, total_bytes);
        progress.percentage = 100.0;
        progress_callback(progress);
        
        Ok(())
    }

    fn extract_tar_gz<F>(
        &self,
        archive_path: &Path,
        options: &ExtractOptions,
        progress_callback: F,
    ) -> Result<(), ArchiveError>
    where
        F: Fn(ArchiveProgress),
    {
        // First pass: count entries and total size
        let file = File::open(archive_path)?;
        let reader = BufReader::new(file);
        let decoder = GzDecoder::new(reader);
        let mut archive = tar::Archive::new(decoder);
        
        let entries_info: Vec<(String, u64, bool)> = archive.entries()?
            .filter_map(|e| e.ok())
            .map(|e| {
                let path = e.path().ok().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                let size = e.header().size().unwrap_or(0);
                let is_dir = e.header().entry_type().is_dir();
                (path, size, is_dir)
            })
            .collect();
        
        let total_files = entries_info.len();
        let total_bytes: u64 = entries_info.iter().map(|(_, s, _)| *s).sum();
        
        // Second pass: extract
        let file = File::open(archive_path)?;
        let reader = BufReader::new(file);
        let decoder = GzDecoder::new(reader);
        let mut archive = tar::Archive::new(decoder);
        
        let mut progress = ArchiveProgress::new(total_files, total_bytes);
        let mut bytes_extracted = 0u64;
        
        for (index, entry_result) in archive.entries()?.enumerate() {
            let mut entry = entry_result?;
            let path = entry.path()?.to_string_lossy().to_string();
            let size = entry.header().size().unwrap_or(0);
            
            progress.update(&path, index, bytes_extracted);
            progress_callback(progress.clone());
            
            let out_path = options.destination.join(&path);
            
            // Handle overwrite mode
            if out_path.exists() {
                match options.overwrite {
                    OverwriteMode::Skip => {
                        bytes_extracted += size;
                        continue;
                    }
                    OverwriteMode::ReplaceIfNewer => {
                        if let (Ok(existing_meta), Ok(archive_mtime)) = 
                            (fs::metadata(&out_path), entry.header().mtime()) 
                        {
                            if let Ok(existing_time) = existing_meta.modified() {
                                use std::time::{Duration, UNIX_EPOCH};
                                let archive_system_time = UNIX_EPOCH + Duration::from_secs(archive_mtime);
                                if existing_time >= archive_system_time {
                                    bytes_extracted += size;
                                    continue;
                                }
                            }
                        }
                    }
                    OverwriteMode::Replace => {}
                }
            }
            
            entry.unpack_in(&options.destination)
                .map_err(|e| ArchiveError::ExtractionFailed(e.to_string()))?;
            
            bytes_extracted += size;
        }
        
        progress.update("Complete", total_files, total_bytes);
        progress.percentage = 100.0;
        progress_callback(progress);
        
        Ok(())
    }

    fn extract_7z<F>(
        &self,
        archive_path: &Path,
        options: &ExtractOptions,
        progress_callback: F,
    ) -> Result<(), ArchiveError>
    where
        F: Fn(ArchiveProgress),
    {
        // First, list contents to get total size
        let entries = self.list_7z_contents(archive_path)?;
        let total_files = entries.len();
        let total_bytes: u64 = entries.iter().map(|e| e.size).sum();
        
        let mut progress = ArchiveProgress::new(total_files, total_bytes);
        progress_callback(progress.clone());
        
        let mut current_index = 0usize;
        let mut bytes_extracted = 0u64;
        
        sevenz_rust::decompress_file_with_extract_fn(
            archive_path,
            &options.destination,
            |entry, reader, dest| {
                let file_name = entry.name().to_string();
                progress.update(&file_name, current_index, bytes_extracted);
                progress_callback(progress.clone());
                
                let out_path = dest.join(entry.name());
                
                // Handle overwrite mode
                if out_path.exists() {
                    match options.overwrite {
                        OverwriteMode::Skip => {
                            bytes_extracted += entry.size();
                            current_index += 1;
                            return Ok(false);
                        }
                        _ => {}
                    }
                }
                
                if entry.is_directory() {
                    fs::create_dir_all(&out_path)?;
                } else {
                    if let Some(parent) = out_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    
                    let mut out_file = File::create(&out_path)?;
                    io::copy(reader, &mut out_file)?;
                }
                
                bytes_extracted += entry.size();
                current_index += 1;
                Ok(true)
            },
        ).map_err(|e| ArchiveError::ExtractionFailed(e.to_string()))?;
        
        progress.update("Complete", total_files, total_bytes);
        progress.percentage = 100.0;
        progress_callback(progress);
        
        Ok(())
    }

    /// Extract a single file from an archive
    pub fn extract_file(
        &self,
        archive_path: &Path,
        file_path: &str,
        destination: &Path,
    ) -> Result<(), ArchiveError> {
        let format = ArchiveFormat::from_extension(archive_path)
            .ok_or_else(|| ArchiveError::UnsupportedFormat("unknown".to_string()))?;

        match format {
            ArchiveFormat::Zip => self.extract_zip_file(archive_path, file_path, destination),
            _ => Err(ArchiveError::UnsupportedFormat(
                "Single file extraction only supported for ZIP".to_string()
            )),
        }
    }

    fn extract_zip_file(
        &self,
        archive_path: &Path,
        file_path: &str,
        destination: &Path,
    ) -> Result<(), ArchiveError> {
        let file = File::open(archive_path)?;
        let reader = BufReader::new(file);
        let mut archive = zip::ZipArchive::new(reader)?;
        
        let mut zip_file = archive.by_name(file_path)?;
        
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let mut out_file = File::create(destination)?;
        io::copy(&mut zip_file, &mut out_file)?;
        
        Ok(())
    }
}
