# Plan for Breaking Up lib.rs

## Current Structure Analysis
The project's lib.rs currently contains all module implementations in a single file, covering:
- Configuration management (environment variables, app settings)
- Gmail API client (email operations)
- People API client (contacts)
- Calendar API client (events, calendars)
- Authentication handling (OAuth, token management)
- Server functionality (MCP protocol implementation)
- Logging utilities (setup, formatting)
- Prompt templates (analysis prompts for emails)
- Error handling (custom error types)
- Utility functions (helpers, formatters)

## Refactoring Goals
1. Separate concerns into individual module files
2. Maintain all existing functionality
3. Preserve the public API surface
4. Keep the application working at each step
5. Make no changes to functionality or behavior

## Module Interdependency Analysis
- Error types are used by most other modules
- Config is used by API clients
- API clients depend on authentication
- Server depends on all API clients
- Logging is used throughout

## Testing Strategy
For each module extraction:
1. Run comprehensive tests: `cargo test`
2. Run targeted tests for the specific module
3. Verify no regressions with `cargo clippy`
4. Test with MCP inspector (for relevant modules)
5. Document any unique considerations or edge cases

## Independent Module Extraction Steps

### ✅ Step 1: Error Handling (`errors.rs`) - COMPLETED
- Extract all error types, implementations, and conversion functions
- Update re-exports in lib.rs
- Testing:
  - Focus on `error_tests.rs`
  - Check error propagation between modules
  - Verify custom error types can be created and used

### ✅ Step 2: Configuration Management (`config.rs`) - COMPLETED
- Extract Config struct, parsing logic, and environment variable handling
- Update re-exports in lib.rs
- Testing:
  - Test with various environment configurations
  - Verify default values load correctly
  - Check environment variable parsing

### ✅ Step 3: Logging Utilities (`logging.rs`) - COMPLETED
- Extract logger setup, formatting, and log level configuration
- Update re-exports in lib.rs
- Testing:
  - Check log capture (stdout and file logging)
  - Test with `--memory-only` flag
  - Verify log levels are respected

### ✅ Step 4: Utility Functions (`utils.rs`) - COMPLETED
- Extract helper functions, formatters, and reusable code
- Update re-exports in lib.rs
- Testing:
  - Run unit tests for individual functions
  - Ensure utility functions can be imported and used elsewhere
  - Verify no behavior changes

### ✅ Step 5: Authentication Handling (`auth.rs`) - COMPLETED
- Extract token management, refresh logic, and OAuth flows
- Update re-exports in lib.rs
- Testing:
  - Focus on `token_gmail_tests.rs`
  - Verify token refresh works properly
  - Test authentication failures and recovery

### ✅ Step 6: Gmail API Client (`gmail_api.rs`) - COMPLETED
- Extract all Gmail API interaction, types, and methods
- Update re-exports in lib.rs
- Testing:
  - Focus on `gmail_message_tests.rs` and `gmail_draft_tests.rs`
  - Test email listing, searching, and retrieval
  - Verify analysis functions work correctly

### ✅ Step 7: People API Client (`people_api.rs`) - COMPLETED
- Extract all People API interaction, types, and methods
- Update re-exports in lib.rs
- Testing:
  - Focus on `people_api_tests.rs`
  - Test contact listing, searching, and retrieval
  - Verify contact data parsing

### ✅ Step 8: Calendar API Client (`calendar_api.rs`) - COMPLETED
- Extract all Calendar API interaction, types, and methods
- Update re-exports in lib.rs
- Testing:
  - Focus on `calendar_api_tests.rs`
  - Test calendar event listing, retrieval, and creation
  - Verify date/time handling

### ✅ Step 9: Prompt Templates (`prompts.rs`) - COMPLETED
- Extract all prompt templates and analysis helpers
- Update re-exports in lib.rs
- Testing:
  - Test email analysis with different types
  - Verify prompt templates are correctly formatted
  - Check email parsing for analysis

### ✅ Step 10: Server Functionality (`server.rs`) - COMPLETED
- Extract MCP server, endpoint handlers, and protocol implementation
- Update re-exports in lib.rs
- Testing:
  - Focus on `server_tests.rs` and `integration_tests.rs`
  - Verify all MCP endpoints work correctly
  - Test with the MCP inspector tool

### Step 11: Clean Up lib.rs
- Organize module declarations
- Clean up re-exports
- Add documentation
- Testing:
  - Run all tests
  - Verify public API is fully preserved
  - Check compilation with no warnings

## Detailed Process for Each Module Extraction

### Pre-extraction
1. Identify module boundaries in lib.rs
2. List all public items that need re-export
3. Map internal dependencies between this module and others
4. Create a backup of the current working state

### Extraction
1. Create the new module file (e.g., `src/errors.rs`)
2. Copy the relevant code from lib.rs to the new file
3. Add necessary imports in the new file
4. Update visibility modifiers (`pub`, `pub(crate)`) as needed
5. Add the module declaration in lib.rs: `mod module_name;`
6. Add re-exports in lib.rs: `pub use module_name::{Item1, Item2};`
7. Remove the old code from lib.rs

### Post-extraction Validation
1. Build: `cargo build`
2. Run linter: `cargo clippy`
3. Run tests: `cargo test` 
4. Run specific tests related to the module
5. Test the application: `cargo run`
6. If relevant, test with MCP: `npx @modelcontextprotocol/inspector cargo run`

### Troubleshooting Common Issues
- **Visibility issues**: Check if items are properly marked as `pub` or `pub(crate)`
- **Import errors**: Ensure correct paths for imports between modules
- **Re-export problems**: Verify all public items are re-exported correctly
- **Module structure**: Check if nested modules are declared properly

## Module Boundary Guidelines
- **Ensure proper encapsulation**: Keep module-specific implementation details private
- **Maintain a clean public API**: Only re-export what's necessary
- **Respect existing patterns**: Follow the codebase's conventions
- **Preserve doc comments**: Move documentation with the code

## Final Verification Checklist
1. Code compiles with no errors or warnings: `cargo build` and `cargo clippy`
2. All tests pass with no regressions: `cargo test`
3. Integration tests validate end-to-end functionality: `cargo test --test integration_tests`
4. Application runs correctly with all features: `cargo run`
5. MCP server works with inspector: `npx @modelcontextprotocol/inspector cargo run`
6. File structure matches the design in README.md
7. Public API remains unchanged

By following this structured approach, we'll systematically extract each module while maintaining functionality and ensuring the application remains fully operational throughout the refactoring process.