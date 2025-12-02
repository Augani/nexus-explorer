# Product Overview

A next-generation, high-performance file explorer built with Rust and GPUI.

## Goals
- Achieve sub-16ms response times (physiological limits of human perception)
- Maximize data retrieval efficiency using GPU-accelerated 2D interfaces
- Eliminate UI freezes through aggressive decoupling of UI and file system operations

## Core Philosophy
The UI must never wait for the file system. The file system is treated as an eventually consistent database, with the UI reflecting current known state while background workers reconcile with the physical disk.

## Target Platforms
- Windows (with NTFS USN Journal acceleration)
- macOS (FSEvents integration)
- Linux (io_uring support planned)
