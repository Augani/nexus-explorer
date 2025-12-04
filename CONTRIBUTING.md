# Contributing to Nexus Explorer

Thank you for your interest in contributing to Nexus Explorer! This document provides guidelines and information for contributors.

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment for everyone.

## How to Contribute

### Reporting Bugs

Before creating a bug report, please check existing issues to avoid duplicates.

When filing a bug report, include:
- Your operating system and version
- Rust version (`rustc --version`)
- Steps to reproduce the issue
- Expected vs actual behavior
- Screenshots if applicable

### Suggesting Features

Feature requests are welcome! Please provide:
- A clear description of the feature
- The problem it solves
- Potential implementation approach (optional)

### Pull Requests

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests (`cargo test`)
5. Run formatter (`cargo fmt`)
6. Run linter (`cargo clippy`)
7. Commit your changes (`git commit -m 'Add amazing feature'`)
8. Push to the branch (`git push origin feature/amazing-feature`)
9. Open a Pull Request

## Development Setup

### Prerequisites

- Rust 1.75 or later
- Git

### Building

```bash
# Clone the repository
git clone https://github.com/yourusername/nexus-explorer.git
cd nexus-explorer

# Build
cargo build

# Run tests
cargo test

# Run the application
cargo run
```

### Project Structure

```
src/
├── main.rs              # Application entry point
├── app/
│   └── workspace.rs     # Root view and layout
├── models/
│   ├── file_system.rs   # File system state management
│   ├── icon_cache.rs    # Icon texture caching
│   ├── search_engine.rs # Fuzzy search integration
│   └── ...
├── views/
│   ├── file_list.rs     # List view component
│   ├── grid_view.rs     # Grid view component
│   ├── sidebar.rs       # Navigation sidebar
│   └── ...
├── io/
│   ├── traversal.rs     # Directory traversal
│   ├── pipeline.rs      # Async data pipeline
│   └── platform/        # Platform-specific code
└── utils/
    ├── icons.rs         # Icon utilities
    └── cache.rs         # LRU cache implementation
```

## Coding Guidelines

### Rust Style

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `rustfmt` for formatting
- Address all `clippy` warnings
- Write documentation for public APIs

### Architecture Principles

1. **No blocking on main thread** - All I/O must be async or in background threads
2. **Keep renders fast** - Render functions should complete in < 8ms
3. **Use GPUI patterns** - Follow the Entity/Model/View separation
4. **Batch updates** - Don't flood the UI with individual updates

### Testing

- Write unit tests for new functionality
- Use `proptest` for property-based testing where appropriate
- Test edge cases (empty directories, permission errors, etc.)

### Commit Messages

Use clear, descriptive commit messages:

```
feat: add column view mode
fix: prevent crash on permission denied
refactor: simplify file entry sorting
docs: update installation instructions
test: add tests for search engine
```

## Getting Help

- Open an issue for questions
- Join our Discord (coming soon)
- Check existing documentation

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
