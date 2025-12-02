# Requirements Document

## Introduction

This document specifies the requirements for a next-generation, high-performance file explorer built with Rust and GPUI. The system aims to achieve sub-16ms response times by treating the file system as an eventually consistent database, ensuring the UI never waits for disk operations. The architecture leverages GPU-accelerated rendering, parallel directory traversal, platform-specific acceleration (NTFS USN Journal, FSEvents, io_uring), and virtualized list rendering to handle directories with millions of files.

## Glossary

- **GPUI**: GPU-accelerated 2D UI framework from the Zed editor
- **Model**: A GPUI entity representing logic or state (e.g., FileSystem, IconCache)
- **View**: A GPUI entity that renders state onto the screen
- **USN Journal**: Update Sequence Number Journal - NTFS change tracking mechanism
- **MFT**: Master File Table - NTFS metadata database containing records for all files
- **FSEvents**: macOS file system event notification framework
- **io_uring**: Linux asynchronous I/O interface
- **Virtualization**: Rendering technique that only creates UI elements for visible items
- **LRU Cache**: Least Recently Used cache eviction strategy
- **Generational ID**: Request tracking mechanism to prevent stale data display
- **jwalk**: Rust crate for parallel directory traversal
- **nucleo**: High-performance fuzzy search engine from Helix editor
- **FileEntry**: Data structure representing a file or directory with metadata
- **Texture Atlas**: Single GPU texture containing multiple icon sprites

## Requirements

### Requirement 1: Directory Navigation

**User Story:** As a user, I want to navigate through directories instantly, so that I can browse my file system without perceiving any delay.

#### Acceptance Criteria

1. WHEN a user clicks on a directory, THE FileSystem model SHALL initiate an asynchronous load operation and display cached results within 16ms if available
2. WHEN directory contents are being loaded, THE FileList view SHALL display a loading indicator while continuing to accept user input
3. WHEN directory traversal completes, THE FileSystem model SHALL batch file entries (100 items or 16ms threshold) before updating the UI
4. WHEN a user navigates to a previously visited directory, THE FileSystem model SHALL display LRU-cached data immediately while revalidating in the background
5. IF a directory load operation is superseded by a new navigation request, THEN THE FileSystem model SHALL discard stale results using generational ID tracking

### Requirement 2: File List Rendering

**User Story:** As a user, I want to view directories containing millions of files without UI lag, so that I can work with large file collections efficiently.

#### Acceptance Criteria

1. WHILE rendering a file list, THE FileList view SHALL use virtualization to render only visible items plus a small buffer
2. WHEN scrolling through a large directory, THE FileList view SHALL maintain 60fps (16ms frame budget) regardless of total file count
3. WHEN displaying file entries, THE FileList view SHALL show file name, size, modification date, and file type icon
4. WHEN the viewport changes due to scrolling or resizing, THE FileList view SHALL recalculate visible items and request only necessary renders

### Requirement 3: Parallel Directory Traversal

**User Story:** As a user, I want directory scanning to utilize all CPU cores, so that large directories load as fast as my hardware allows.

#### Acceptance Criteria

1. WHEN scanning a directory recursively, THE traversal system SHALL use parallel workers via jwalk to saturate available I/O bandwidth
2. WHEN traversing directories, THE traversal system SHALL perform sorting on worker threads before delivering results to the UI
3. WHEN streaming results, THE traversal system SHALL send batched updates through async channels to prevent UI thread flooding
4. WHILE traversal is in progress, THE UI thread SHALL remain responsive to user input without blocking

### Requirement 4: Icon Loading Pipeline

**User Story:** As a user, I want to see file type icons without experiencing UI stutters, so that I can visually identify files quickly.

#### Acceptance Criteria

1. WHEN an icon is not cached, THE IconCache model SHALL return a default placeholder icon immediately and queue an async fetch
2. WHEN fetching icons, THE icon pipeline SHALL decode images on background threads and convert RGBA to BGRA format
3. WHEN an icon fetch completes, THE IconCache model SHALL update the cache and trigger a re-render of affected rows only
4. WHILE managing icon textures, THE IconCache model SHALL use LRU eviction to bound VRAM usage
5. WHEN rendering common icons (folder, generic file), THE IconCache model SHALL use a pre-loaded texture atlas to minimize draw calls

### Requirement 5: Fuzzy Search

**User Story:** As a user, I want to search files by typing partial names, so that I can quickly locate files without remembering exact names.

#### Acceptance Criteria

1. WHEN a user types in the search input, THE SearchEngine model SHALL update the nucleo pattern and begin matching immediately
2. WHEN search results are available, THE SearchEngine model SHALL return matched indices with character positions for highlighting
3. WHEN displaying search results, THE FileList view SHALL highlight matching characters in file names
4. WHILE the user continues typing, THE SearchEngine model SHALL cancel previous searches and start new ones without accumulating latency
5. WHEN files are discovered during directory traversal, THE SearchEngine model SHALL inject them into the nucleo index incrementally

### Requirement 6: Platform-Specific File System Monitoring

**User Story:** As a user, I want file changes to appear instantly in the explorer, so that I always see the current state of my file system.

#### Acceptance Criteria

1. WHERE the platform is Windows, THE monitoring system SHALL use the NTFS USN Journal to detect file changes in real-time
2. WHERE the platform is macOS, THE monitoring system SHALL use FSEvents via the notify crate to detect directory changes
3. WHERE the platform is Linux, THE monitoring system SHALL use inotify via the notify crate to detect file changes
4. WHEN a file change event is received, THE FileSystem model SHALL update the affected entries and trigger a UI refresh within 100ms
5. WHEN monitoring a directory, THE monitoring system SHALL coalesce rapid successive events to prevent update storms

### Requirement 7: Windows NTFS Acceleration

**User Story:** As a Windows user, I want instant whole-drive search capability, so that I can find any file on my system in milliseconds.

#### Acceptance Criteria

1. WHERE the platform is Windows, THE MFT parser SHALL build an in-memory index of all files on NTFS volumes at startup
2. WHEN parsing the MFT, THE parser SHALL construct a HashMap mapping FileReferenceNumber to FileNode for O(1) path reconstruction
3. WHEN the USN Journal reports a file change, THE monitoring system SHALL update the in-memory index immediately
4. WHEN searching across the entire drive, THE SearchEngine model SHALL query the in-memory MFT index instead of traversing the file system

### Requirement 8: State Management

**User Story:** As a user, I want consistent UI state even during rapid navigation, so that I never see outdated or mixed results.

#### Acceptance Criteria

1. WHEN a navigation request is made, THE FileSystem model SHALL increment a generational request ID
2. WHEN an async operation completes, THE FileSystem model SHALL validate the request ID before applying updates
3. IF the request ID does not match the current generation, THEN THE FileSystem model SHALL discard the stale results silently
4. WHEN caching directory state, THE FileSystem model SHALL store the cache generation for staleness detection during revalidation

### Requirement 9: Application Initialization

**User Story:** As a user, I want the application to start quickly and be immediately usable, so that I can begin working without waiting.

#### Acceptance Criteria

1. WHEN the application starts, THE Workspace view SHALL render the initial UI within 500ms
2. WHEN initializing, THE application SHALL spawn the Tokio runtime on a dedicated thread for I/O operations
3. WHEN initializing, THE application SHALL pre-load default icons and common textures into the GPU
4. WHEN the user's home directory is detected, THE FileSystem model SHALL begin loading it asynchronously during startup

### Requirement 10: Data Serialization

**User Story:** As a developer, I want file entries and cache state to be serializable, so that the system can persist and restore state efficiently.

#### Acceptance Criteria

1. WHEN serializing FileEntry structures, THE system SHALL encode them using a binary format (e.g., bincode) for compact storage
2. WHEN deserializing cached data, THE system SHALL validate the data integrity before use
3. WHEN persisting the MFT index, THE system SHALL serialize the HashMap to disk for faster subsequent startups
4. WHEN loading serialized data, THE parser SHALL produce equivalent in-memory structures to the original data
