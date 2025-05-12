# Development Guide

## Setup

1. Clone the repository
2. Install Rust and Cargo (if not already installed)
3. Install development dependencies:
   ```bash
   cargo install cargo-audit cargo-tarpaulin
   ```

## Development Workflow

### Building
```bash
cargo build
```

For release builds:
```bash
cargo build --release
```

### Running
```bash
cargo run
```

With MCP inspector:
```bash
npx @modelcontextprotocol/inspector cargo run
```

### Testing

Run all tests:
```bash
cargo test
```

Run specific test:
```bash
cargo test test_name
```

Run integration tests only:
```bash
cargo test --test integration_tests
```

### Code Coverage
```bash
cargo tarpaulin
```

### Documentation

Generate API documentation:
```bash
cargo doc --no-deps --open
```

### Security Audit
```bash
cargo audit
```

### Benchmarking
```bash
cargo bench
```

## Code Style

- Follow Rust standard formatting (use `cargo fmt`)
- Run linter before committing (`cargo clippy`)
- Write tests for all new functionality
- Document public API with doc comments
- Use appropriate error handling with `thiserror`
- Follow the naming conventions in CLAUDE.md
