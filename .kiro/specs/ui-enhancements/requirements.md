# Requirements Document

## Introduction

This document specifies the requirements for UI enhancements and missing functionality in the Nexus Explorer file manager. The system currently has core file navigation implemented but lacks functional search, clickable breadcrumbs, sorting, sidebar drag-and-drop, grid view, terminal integration, file preview capabilities, tools section, proper tab management, and needs UI polish for spacing and alignment. These enhancements will transform the application from a basic file browser into a full-featured development-oriented file explorer.

## Glossary

- **Terminal Emulator**: An embedded command-line interface allowing users to execute shell commands within the application
- **Preview Panel**: A third-column panel displaying file contents, metadata, and quick actions
- **Tools Section**: A sidebar area providing quick access to common file operations and utilities
- **Tab Bar**: A horizontal bar displaying multiple open directories as switchable tabs
- **Status Bar**: A bottom bar showing current state, file counts, and quick action buttons
- **PTY (Pseudo-Terminal)**: A virtual terminal device for terminal emulation
- **ANSI Escape Codes**: Control sequences for terminal text formatting and colors
- **Adabaraka UI**: The GPUI component library used for consistent UI styling
- **Breadcrumb Navigation**: A path display showing hierarchical location with clickable segments
- **Fuzzy Search**: A search algorithm that matches partial and approximate strings
- **Grid View**: A file display mode showing items as icons in a grid layout
- **List View**: A file display mode showing items as rows with columns for metadata

## Requirements

### Requirement 1: Search Functionality

**User Story:** As a user, I want to search files in the current folder by typing in the search bar, so that I can quickly find files without scrolling.

#### Acceptance Criteria

1. WHEN a user clicks the search input in the title bar, THE Search component SHALL focus the input and enable typing
2. WHEN a user types in the search input, THE Search component SHALL filter the current directory's files in real-time within 16ms
3. WHEN search results are available, THE FileList SHALL display only matching files with highlighted match positions
4. WHEN the search query matches file names partially, THE Search component SHALL use fuzzy matching to include approximate matches
5. WHEN the search input is cleared or empty, THE FileList SHALL display all files in the current directory
6. WHEN a user presses Escape while searching, THE Search component SHALL clear the search and restore the full file list
7. WHEN a user presses Enter on a search result, THE FileList SHALL select and optionally open the first matching item
8. WHEN no files match the search query, THE FileList SHALL display an empty state with "No matching files" message

### Requirement 2: Breadcrumb Navigation

**User Story:** As a user, I want to click on any part of the breadcrumb path to navigate directly to that location, so that I can quickly jump to parent directories.

#### Acceptance Criteria

1. WHEN the breadcrumb path is displayed, THE Breadcrumb component SHALL render each path segment as a clickable element
2. WHEN a user clicks a breadcrumb segment, THE Workspace SHALL navigate to that directory immediately
3. WHEN hovering over a breadcrumb segment, THE Breadcrumb component SHALL display a hover state with underline or background change
4. WHEN the path is too long to display, THE Breadcrumb component SHALL truncate middle segments with an ellipsis dropdown
5. WHEN a user clicks the ellipsis, THE Breadcrumb component SHALL display a dropdown menu with hidden path segments
6. WHEN the current directory changes, THE Breadcrumb component SHALL update to reflect the new path
7. WHEN a user right-clicks a breadcrumb segment, THE Breadcrumb component SHALL display a context menu with "Copy Path" option
8. WHEN the root segment is clicked, THE Workspace SHALL navigate to the root directory (/ on Unix, drive root on Windows)

### Requirement 3: Column Sorting

**User Story:** As a user, I want to sort files by clicking column headers, so that I can organize the file list by name, date, type, or size.

#### Acceptance Criteria

1. WHEN a user clicks the "NAME" column header, THE FileList SHALL sort entries alphabetically by name
2. WHEN a user clicks the "DATE" column header, THE FileList SHALL sort entries by modification date (newest first by default)
3. WHEN a user clicks the "TYPE" column header, THE FileList SHALL sort entries by file extension alphabetically
4. WHEN a user clicks the "SIZE" column header, THE FileList SHALL sort entries by file size (largest first by default)
5. WHEN a user clicks the same column header twice, THE FileList SHALL reverse the sort order (ascending/descending toggle)
6. WHEN sorting is active, THE column header SHALL display a sort direction indicator (arrow up or down)
7. WHEN sorting by any column, THE FileList SHALL always keep directories grouped before files (or after, based on preference)
8. WHEN the directory contents change, THE FileList SHALL maintain the current sort order for new entries

### Requirement 4: Sidebar Favorites with Drag-and-Drop

**User Story:** As a user, I want to drag folders to the sidebar to add them as favorites, so that I can quickly access frequently used directories.

#### Acceptance Criteria

1. WHEN a user drags a folder from the file list, THE Sidebar SHALL display a drop zone indicator in the Favorites section
2. WHEN a folder is dropped on the Favorites section, THE Sidebar SHALL add the folder as a new favorite with its name and icon
3. WHEN a favorite is added, THE Sidebar SHALL persist the favorite to user settings for future sessions
4. WHEN a user clicks a favorite, THE Workspace SHALL navigate to that directory
5. WHEN a user right-clicks a favorite, THE Sidebar SHALL display a context menu with "Remove from Favorites" option
6. WHEN a user drags a favorite within the Favorites section, THE Sidebar SHALL allow reordering favorites
7. WHEN a favorite's target directory is deleted or moved, THE Sidebar SHALL display the favorite with a warning indicator
8. IF the maximum number of favorites (10) is reached, THEN THE Sidebar SHALL display a message and prevent adding more

### Requirement 5: Grid View Mode

**User Story:** As a user, I want to switch between list and grid view modes, so that I can view files as icons when browsing images or prefer visual layouts.

#### Acceptance Criteria

1. WHEN a user clicks the grid view toggle button, THE FileList SHALL switch to displaying files as a grid of icons
2. WHEN in grid view, THE FileList SHALL display file icons at 64x64 pixels with the file name below
3. WHEN in grid view, THE FileList SHALL arrange items in a responsive grid that adjusts to window width
4. WHEN a user clicks the list view toggle button, THE FileList SHALL switch back to the columnar list view
5. WHEN in grid view, THE FileList SHALL support the same selection, navigation, and context menu features as list view
6. WHEN in grid view with image files, THE FileList SHALL display thumbnail previews instead of generic icons
7. WHEN the view mode changes, THE FileList SHALL preserve the current selection and scroll position
8. WHEN the application restarts, THE FileList SHALL restore the last used view mode from settings

### Requirement 6: Integrated Terminal

**User Story:** As a developer, I want an integrated terminal in the file explorer, so that I can execute commands without switching applications.

#### Acceptance Criteria

1. WHEN a user clicks the terminal toggle button, THE Workspace SHALL display or hide the terminal panel with smooth animation
2. WHEN the terminal panel is visible, THE Terminal component SHALL spawn a shell process (zsh on macOS, bash on Linux, cmd/powershell on Windows)
3. WHEN a user types in the terminal, THE Terminal component SHALL send keystrokes to the PTY and display output within 16ms
4. WHEN the terminal receives output with ANSI escape codes, THE Terminal component SHALL render colored and styled text correctly
5. WHEN the terminal panel opens, THE Terminal component SHALL set the working directory to the current file explorer path
6. WHEN a user navigates to a different directory in the explorer, THE Terminal component SHALL offer to change the terminal working directory
7. WHEN the terminal process exits, THE Terminal component SHALL display an exit message and allow spawning a new shell
8. IF the terminal output exceeds the visible area, THEN THE Terminal component SHALL provide smooth scrollback with virtualized rendering

### Requirement 7: File Preview Panel

**User Story:** As a user, I want to preview file contents without opening external applications, so that I can quickly inspect files.

#### Acceptance Criteria

1. WHEN a user selects a file in the file list, THE Preview panel SHALL display file metadata (name, size, type, modified date, permissions)
2. WHEN a text file is selected, THE Preview panel SHALL display syntax-highlighted content with line numbers
3. WHEN an image file is selected, THE Preview panel SHALL display a thumbnail preview with dimensions and format info
4. WHEN a code file is selected, THE Preview panel SHALL provide an "Explain Code" action button for AI assistance
5. WHEN a binary or unsupported file is selected, THE Preview panel SHALL display a hex dump preview of the first 256 bytes
6. WHEN a directory is selected, THE Preview panel SHALL display directory statistics (item count, total size, subdirectory count)
7. WHEN the preview content exceeds the visible area, THE Preview panel SHALL provide smooth scrolling
8. IF file reading fails due to permissions, THEN THE Preview panel SHALL display an appropriate error message

### Requirement 8: Tools Section

**User Story:** As a user, I want quick access to common file operations, so that I can perform tasks efficiently without navigating menus.

#### Acceptance Criteria

1. WHEN the sidebar is visible, THE Tools section SHALL display a collapsible panel with categorized actions
2. WHEN a user clicks "New File", THE Tools section SHALL create a new file in the current directory with a name input dialog
3. WHEN a user clicks "New Folder", THE Tools section SHALL create a new directory with a name input dialog
4. WHEN files are selected, THE Tools section SHALL enable batch operations (copy, move, delete, compress)
5. WHEN a user clicks "Open Terminal Here", THE Tools section SHALL open the terminal panel with the current directory as working directory
6. WHEN a user clicks "Copy Path", THE Tools section SHALL copy the current directory path to the system clipboard
7. WHEN a user clicks "Refresh", THE Tools section SHALL reload the current directory contents
8. WHEN a user clicks "Show Hidden Files", THE Tools section SHALL toggle visibility of hidden files (dotfiles)

### Requirement 9: Tab Management

**User Story:** As a user, I want to open multiple directories in tabs, so that I can quickly switch between locations.

#### Acceptance Criteria

1. WHEN the application starts, THE Tab bar SHALL display a single tab for the initial directory
2. WHEN a user middle-clicks a directory or uses Cmd/Ctrl+T, THE Tab bar SHALL open a new tab for that directory
3. WHEN a user clicks a tab, THE Workspace SHALL switch to display that tab's directory contents
4. WHEN a user clicks the close button on a tab, THE Tab bar SHALL close that tab and switch to an adjacent tab
5. WHEN multiple tabs are open, THE Tab bar SHALL display tab titles with directory names and close buttons
6. WHEN a tab's directory is modified externally, THE Tab bar SHALL indicate the tab needs refresh with a visual indicator
7. WHEN tabs exceed the available width, THE Tab bar SHALL provide horizontal scrolling or a dropdown menu
8. IF the last tab is closed, THEN THE Tab bar SHALL open a new tab with the home directory

### Requirement 10: Status Bar

**User Story:** As a user, I want a status bar showing current state and quick actions, so that I can see context and access common functions.

#### Acceptance Criteria

1. WHEN the application is running, THE Status bar SHALL display at the bottom of the window
2. WHEN viewing a directory, THE Status bar SHALL show the total item count and selected item count
3. WHEN files are selected, THE Status bar SHALL show the combined size of selected files
4. WHEN a user clicks the terminal icon in the status bar, THE Status bar SHALL toggle the terminal panel
5. WHEN a user clicks the view mode toggle, THE Status bar SHALL switch between list and grid views
6. WHEN a background operation is in progress, THE Status bar SHALL display a progress indicator
7. WHEN the current directory has a git repository, THE Status bar SHALL display the current branch name
8. WHEN hovering over status bar items, THE Status bar SHALL display tooltips with additional information

### Requirement 11: UI Spacing, Typography and Visual Design

**User Story:** As a user, I want a visually distinctive and polished interface with RPG-inspired aesthetics, so that the application feels premium and delightful to use.

#### Acceptance Criteria

1. WHEN rendering display text (headers, titles), THE typography system SHALL use Crimson Pro serif font loaded from Google Fonts with weight extremes (200 thin vs 900 black)
2. WHEN rendering body text (file names, labels), THE typography system SHALL use IBM Plex Sans with 400/600 weights for technical clarity
3. WHEN rendering code and terminal text, THE typography system SHALL use JetBrains Mono monospace font exclusively
4. WHEN rendering font sizes, THE typography system SHALL use dramatic 3x+ size jumps (12px → 36px → 72px) rather than incremental scaling
5. WHEN rendering the sidebar, THE layout system SHALL use 280px width with 16px item padding, 24px section gaps, and ornate section dividers
6. WHEN rendering file list rows, THE layout system SHALL use 40px row height with 20px icons, 12px icon-to-text gap, and subtle hover glow effects
7. WHEN rendering the toolbar, THE layout system SHALL use 52px height with 36px buttons, decorative corner flourishes, and themed dividers
8. WHEN rendering panels and cards, THE layout system SHALL apply fantasy-inspired decorations (ornate borders, parchment textures, ember glows, crystalline effects)
9. WHEN rendering backgrounds, THE layout system SHALL use layered atmospheric depth with gradients, noise textures, or themed patterns (volcanic, frost, ancient)
10. WHEN the window is resized, THE layout system SHALL maintain proportional spacing with smooth 250ms transitions

### Requirement 12: Terminal Visual Polish

**User Story:** As a developer, I want the terminal to look and feel like a native terminal application, so that I can work comfortably.

#### Acceptance Criteria

1. WHEN rendering the terminal, THE Terminal component SHALL use a monospace font (JetBrains Mono or system monospace)
2. WHEN displaying the prompt, THE Terminal component SHALL render with colored segments (user, path, git branch)
3. WHEN the cursor is active, THE Terminal component SHALL display a blinking block or line cursor
4. WHEN text is selected in the terminal, THE Terminal component SHALL highlight with a distinct selection color
5. WHEN a user copies text from the terminal, THE Terminal component SHALL copy to the system clipboard
6. WHEN a user pastes into the terminal, THE Terminal component SHALL insert clipboard contents at cursor
7. WHEN the terminal tab bar is visible, THE Terminal component SHALL use proper tab styling from Adabaraka UI
8. WHEN switching between Terminal and Output tabs, THE Terminal component SHALL preserve scroll position and content

### Requirement 13: Keyboard Navigation

**User Story:** As a power user, I want comprehensive keyboard shortcuts, so that I can navigate and operate efficiently without a mouse.

#### Acceptance Criteria

1. WHEN a user presses Up/Down arrows in the file list, THE FileList SHALL move selection to adjacent items
2. WHEN a user presses Enter on a selected directory, THE FileList SHALL navigate into that directory
3. WHEN a user presses Backspace or Cmd/Ctrl+Up, THE Workspace SHALL navigate to the parent directory
4. WHEN a user presses Cmd/Ctrl+T, THE Workspace SHALL open a new tab
5. WHEN a user presses Cmd/Ctrl+W, THE Workspace SHALL close the current tab
6. WHEN a user presses Cmd/Ctrl+`, THE Workspace SHALL toggle the terminal panel
7. WHEN a user presses Cmd/Ctrl+Shift+P, THE Workspace SHALL open the command palette
8. WHEN a user presses Cmd/Ctrl+F, THE Workspace SHALL focus the search input


### Requirement 14: Theme Switching with RPG Aesthetics

**User Story:** As a user, I want to switch between distinctive RPG-inspired themes, so that I can customize the appearance to match my style and mood.

#### Acceptance Criteria

1. WHEN the application starts, THE Theme system SHALL load the user's previously selected theme from settings
2. WHEN a user opens the theme picker, THE Theme system SHALL display available themes with animated live previews showing colors, typography, decorations, and background effects
3. WHEN a user selects a theme, THE Theme system SHALL apply the theme with a smooth orchestrated transition (250ms with staggered element reveals)
4. WHEN a theme is applied, THE Theme system SHALL update all visual elements: colors, typography weights, border styles, corner flourishes, divider styles, shadows, glows, and background textures
5. WHEN a user selects "Dragon Forge" theme (default), THE Theme system SHALL apply deep crimson (#d43f3f) and molten gold (#f4b842) palette with warm parchment text (#f4e8dc), volcanic backgrounds, ember glow effects, and ornate gold-trimmed borders
6. WHEN a user selects "Frost Haven" theme, THE Theme system SHALL apply ice blues (#6bd4ff) and aurora purples (#b48aff) with crystalline borders, northern lights glow effects, and frosted glass backgrounds
7. WHEN a user selects "Ancient Tome" theme, THE Theme system SHALL apply parchment textures, leather browns (#8b4513), gold leaf accents (#d4af37), weathered paper backgrounds, and medieval serif typography emphasis
8. WHEN a user selects "Shadow Realm" theme, THE Theme system SHALL apply deep purples (#4a0080), ethereal glows (#9966ff), void blacks (#050508), and mystical particle effects
9. WHEN a user selects "Elven Glade" theme, THE Theme system SHALL apply forest greens (#228b22), moonlight silver (#c0c0c0), bark browns, and organic flowing borders
10. WHEN the theme changes, THE Theme system SHALL persist the selection and apply theme-specific font weights, letter spacing, and decorative elements


### Requirement 15: Quick Look / File Preview Shortcut

**User Story:** As a user, I want to press Space to quickly preview any file, so that I can inspect files without opening them (like macOS Quick Look).

#### Acceptance Criteria

1. WHEN a user presses Space with a file selected, THE Quick Look panel SHALL display a full preview overlay
2. WHEN Quick Look is open, THE overlay SHALL display the file at maximum readable size
3. WHEN a user presses Space again or Escape, THE Quick Look panel SHALL close immediately
4. WHEN Quick Look is open for an image, THE overlay SHALL display the full-resolution image with zoom controls
5. WHEN Quick Look is open for a video, THE overlay SHALL play the video with playback controls
6. WHEN Quick Look is open for a document, THE overlay SHALL render the document content
7. WHEN a user presses arrow keys while Quick Look is open, THE overlay SHALL preview the next/previous file
8. WHEN Quick Look is open, THE overlay SHALL display file name, size, and modification date in a header

### Requirement 16: Multi-Window Support

**User Story:** As a user, I want to open multiple explorer windows, so that I can work with files in different locations simultaneously.

#### Acceptance Criteria

1. WHEN a user presses Cmd/Ctrl+N, THE application SHALL open a new window with the current directory
2. WHEN a new window opens, THE window SHALL be fully independent with its own tabs and state
3. WHEN multiple windows are open, THE application SHALL allow drag-and-drop between windows
4. WHEN a window is closed, THE application SHALL continue running if other windows exist
5. WHEN the last window is closed, THE application SHALL quit (or minimize to tray based on settings)
6. WHEN opening a new window, THE window SHALL appear offset from existing windows for visibility
7. WHEN a user right-clicks a folder, THE context menu SHALL include "Open in New Window" option
8. WHEN the application starts, THE application SHALL restore previously open windows if enabled in settings

### Requirement 17: File Operations with Progress

**User Story:** As a user, I want to see progress when copying, moving, or deleting files, so that I know how long operations will take.

#### Acceptance Criteria

1. WHEN a file operation starts, THE Progress panel SHALL display operation type and file count
2. WHEN copying files, THE Progress panel SHALL show current file, bytes transferred, and estimated time remaining
3. WHEN moving files, THE Progress panel SHALL show progress and handle cross-volume moves correctly
4. WHEN deleting files, THE Progress panel SHALL show deletion progress with file count
5. WHEN an operation encounters an error, THE Progress panel SHALL display the error and offer Skip/Retry/Cancel options
6. WHEN a user clicks Cancel, THE operation SHALL stop gracefully and leave completed files in place
7. WHEN multiple operations run simultaneously, THE Progress panel SHALL show all operations in a queue
8. WHEN an operation completes, THE Progress panel SHALL show a completion notification and auto-dismiss

### Requirement 18: Undo/Redo for File Operations

**User Story:** As a user, I want to undo accidental file operations, so that I can recover from mistakes without using the trash.

#### Acceptance Criteria

1. WHEN a user presses Cmd/Ctrl+Z after a file operation, THE Undo system SHALL reverse the last operation
2. WHEN undoing a move, THE Undo system SHALL move files back to their original location
3. WHEN undoing a copy, THE Undo system SHALL delete the copied files
4. WHEN undoing a rename, THE Undo system SHALL restore the original file name
5. WHEN undoing a delete, THE Undo system SHALL restore files from trash to original location
6. WHEN a user presses Cmd/Ctrl+Shift+Z, THE Undo system SHALL redo the last undone operation
7. WHEN the application restarts, THE Undo history SHALL be cleared (operations are session-only)
8. IF an undo operation fails, THEN THE Undo system SHALL display an error and preserve the undo history

### Requirement 19: Batch Rename

**User Story:** As a user, I want to rename multiple files at once with patterns, so that I can organize files efficiently.

#### Acceptance Criteria

1. WHEN multiple files are selected and user presses F2 or clicks Rename, THE Batch Rename dialog SHALL open
2. WHEN the dialog opens, THE Batch Rename dialog SHALL show a preview of renamed files in real-time
3. WHEN a user enters a pattern with {n}, THE Batch Rename dialog SHALL replace with sequential numbers
4. WHEN a user enters a pattern with {date}, THE Batch Rename dialog SHALL replace with file modification date
5. WHEN a user enters a pattern with {ext}, THE Batch Rename dialog SHALL preserve the original extension
6. WHEN a user clicks Find/Replace, THE Batch Rename dialog SHALL allow text substitution in file names
7. WHEN a user clicks Apply, THE Batch Rename dialog SHALL rename all files and show completion status
8. IF a rename would cause a conflict, THEN THE Batch Rename dialog SHALL highlight the conflict and prevent apply

### Requirement 20: Bookmarks and Quick Access

**User Story:** As a user, I want to bookmark frequently accessed folders and access them quickly, so that I can navigate efficiently.

#### Acceptance Criteria

1. WHEN a user presses Cmd/Ctrl+D, THE Bookmark system SHALL add the current directory to bookmarks
2. WHEN bookmarks exist, THE Sidebar SHALL display a Bookmarks section above Favorites
3. WHEN a user clicks a bookmark, THE Workspace SHALL navigate to that directory
4. WHEN a user assigns a keyboard shortcut to a bookmark, THE Workspace SHALL navigate on that shortcut
5. WHEN a user right-clicks a bookmark, THE context menu SHALL offer Remove, Rename, and Assign Shortcut options
6. WHEN bookmarks are modified, THE Bookmark system SHALL persist changes immediately
7. WHEN the Go menu is opened, THE menu SHALL list recent locations and bookmarks
8. WHEN a user presses Cmd/Ctrl+G, THE Go to Folder dialog SHALL open for direct path entry

### Requirement 21: File Tags and Labels

**User Story:** As a user, I want to tag files with colors and labels, so that I can organize and find files by category.

#### Acceptance Criteria

1. WHEN a user right-clicks a file, THE context menu SHALL include a Tags submenu with color options
2. WHEN a tag is applied, THE FileList SHALL display a colored dot indicator next to the file name
3. WHEN multiple tags are applied, THE FileList SHALL display multiple colored dots
4. WHEN a user clicks a tag in the sidebar, THE FileList SHALL filter to show only files with that tag
5. WHEN searching, THE Search component SHALL support "tag:red" syntax to filter by tag
6. WHEN a user creates a custom tag, THE Tag system SHALL allow custom name and color
7. WHEN tags are applied, THE Tag system SHALL store tags in extended file attributes (xattr) where supported
8. IF extended attributes are not supported, THEN THE Tag system SHALL store tags in a local database

### Requirement 22: Dual Pane Mode

**User Story:** As a user, I want to view two directories side by side, so that I can easily compare and transfer files.

#### Acceptance Criteria

1. WHEN a user presses Cmd/Ctrl+Shift+D, THE Workspace SHALL split into dual pane mode
2. WHEN in dual pane mode, THE Workspace SHALL display two independent file lists side by side
3. WHEN a user presses Tab, THE focus SHALL switch between the two panes
4. WHEN a user drags files between panes, THE operation SHALL copy or move based on modifier keys
5. WHEN a user presses F5, THE Workspace SHALL copy selected files from active pane to inactive pane
6. WHEN a user presses F6, THE Workspace SHALL move selected files from active pane to inactive pane
7. WHEN a user presses Cmd/Ctrl+Shift+D again, THE Workspace SHALL exit dual pane mode
8. WHEN in dual pane mode, THE breadcrumbs and toolbar SHALL reflect the active pane

### Requirement 23: Column View (Miller Columns)

**User Story:** As a user, I want a column view like macOS Finder, so that I can see the directory hierarchy at a glance.

#### Acceptance Criteria

1. WHEN a user selects Column View mode, THE FileList SHALL display directories as cascading columns
2. WHEN a directory is selected, THE next column SHALL display its contents
3. WHEN navigating deeper, THE columns SHALL scroll horizontally to show the current path
4. WHEN a file is selected in column view, THE Preview panel SHALL display on the right
5. WHEN the window is resized, THE column widths SHALL adjust proportionally
6. WHEN a user double-clicks a file, THE file SHALL open with the default application
7. WHEN a user presses Right arrow, THE focus SHALL move into the selected directory
8. WHEN a user presses Left arrow, THE focus SHALL move to the parent column

### Requirement 24: Smart Folders / Saved Searches

**User Story:** As a user, I want to save search queries as smart folders, so that I can quickly access files matching specific criteria.

#### Acceptance Criteria

1. WHEN a user performs a search and clicks "Save Search", THE Smart Folder dialog SHALL open
2. WHEN creating a smart folder, THE dialog SHALL allow naming and choosing save location
3. WHEN a smart folder is clicked, THE FileList SHALL display files matching the saved query
4. WHEN files change on disk, THE smart folder results SHALL update automatically
5. WHEN a user right-clicks a smart folder, THE context menu SHALL offer Edit and Delete options
6. WHEN editing a smart folder, THE dialog SHALL show the current query for modification
7. WHEN smart folders exist, THE Sidebar SHALL display them in a Smart Folders section
8. WHEN a smart folder query includes date criteria, THE results SHALL update as time passes

### Requirement 25: Network and Cloud Storage

**User Story:** As a user, I want to browse network drives and cloud storage, so that I can access all my files in one place.

#### Acceptance Criteria

1. WHEN the sidebar loads, THE Network section SHALL display available network locations
2. WHEN a user clicks "Connect to Server", THE connection dialog SHALL open for SMB/FTP/SFTP entry
3. WHEN connected to a network location, THE FileList SHALL display remote files with appropriate icons
4. WHEN browsing network files, THE status bar SHALL indicate the connection type and latency
5. WHEN a network operation is slow, THE FileList SHALL display loading indicators per item
6. WHEN cloud storage is configured, THE Sidebar SHALL display cloud providers (iCloud, Dropbox, etc.)
7. WHEN browsing cloud storage, THE FileList SHALL show sync status indicators
8. IF a network connection fails, THEN THE FileList SHALL display an error and offer reconnect option


### Requirement 26: External Drives and Mounted Filesystems

**User Story:** As a user, I want to see all connected drives, USB devices, and mounted filesystems in the sidebar, so that I can easily access external storage.

#### Acceptance Criteria

1. WHEN a USB drive is connected, THE Sidebar SHALL display the drive in a "Devices" section within 2 seconds
2. WHEN an external hard drive is mounted, THE Sidebar SHALL display it with the volume name and appropriate icon
3. WHEN a network drive is mounted, THE Sidebar SHALL display it in the Devices section with a network icon
4. WHERE the platform is Windows, THE Sidebar SHALL display all drive letters (C:, D:, etc.) and WSL distributions
5. WHERE the platform is macOS, THE Sidebar SHALL display mounted volumes from /Volumes including disk images
6. WHERE the platform is Linux, THE Sidebar SHALL display mounted filesystems from /media and /mnt directories
7. WHEN a device is ejected or unmounted, THE Sidebar SHALL remove it from the Devices section immediately
8. WHEN a user right-clicks a removable device, THE context menu SHALL include "Eject" or "Unmount" option

### Requirement 27: WSL Integration (Windows)

**User Story:** As a Windows developer, I want to browse WSL (Windows Subsystem for Linux) filesystems, so that I can manage Linux files from the explorer.

#### Acceptance Criteria

1. WHERE the platform is Windows with WSL installed, THE Sidebar SHALL display installed WSL distributions
2. WHEN a user clicks a WSL distribution, THE FileList SHALL display the Linux filesystem starting at /home
3. WHEN browsing WSL files, THE FileList SHALL display Linux permissions and ownership information
4. WHEN copying files between Windows and WSL, THE operation SHALL handle path translation correctly
5. WHEN a WSL distribution is running, THE Sidebar SHALL indicate its running status with a green indicator
6. WHEN a user right-clicks a WSL distribution, THE context menu SHALL include "Open Terminal Here" option
7. WHEN browsing WSL, THE breadcrumb SHALL show the distribution name as the root (e.g., "Ubuntu > home > user")
8. IF WSL is not installed, THEN THE Sidebar SHALL not display the WSL section

### Requirement 28: Device Monitoring and Hot-Plug

**User Story:** As a user, I want the explorer to automatically detect when devices are connected or disconnected, so that I always see current available storage.

#### Acceptance Criteria

1. WHEN the application starts, THE Device Monitor SHALL enumerate all currently connected devices
2. WHEN a new device is connected, THE Device Monitor SHALL detect it and notify the Sidebar within 2 seconds
3. WHEN a device is disconnected, THE Device Monitor SHALL detect removal and update the Sidebar immediately
4. WHEN a device is disconnected while browsing it, THE FileList SHALL display an error and offer to navigate elsewhere
5. WHEN monitoring devices, THE Device Monitor SHALL use platform-native APIs for efficient detection
6. WHEN a device has a custom label, THE Sidebar SHALL display the label instead of generic "USB Drive"
7. WHEN a device is read-only, THE Sidebar SHALL display a lock icon indicator
8. WHEN multiple partitions exist on a device, THE Sidebar SHALL display each partition separately

