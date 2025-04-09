# Test Suite Improvement TODO List

## Runtime Conflict Resolution
- [x] Identify tests with runtime conflicts between tokio and mockito
- [x] Apply temporary workaround using `#[ignore]` attributes
- [x] Document the issue and proposed solutions in `FIXING_RUNTIME_CONFLICTS.md`
- [ ] Implement Option 1 from `FIXING_RUNTIME_CONFLICTS.md` to fix tests in:
  - [ ] `gmail_message_tests.rs`
  - [ ] `gmail_draft_tests.rs`
  - [ ] `gmail_api_integration_tests.rs`
  - [ ] `gmail_simple_test.rs`

## Coverage Improvements
- [ ] Complete remaining coverage for Calendar API (target: 95%)
  - [ ] Add tests for recurring events
  - [ ] Test additional calendar edge cases
- [ ] Complete remaining coverage for Gmail API (target: 95%)
  - [ ] Test edge cases for email parsing
  - [ ] Test large attachments
- [ ] Complete remaining coverage for People API (target: 95%)
  - [ ] Test internationalization features
  - [ ] Test edge cases for contact formats

## Infrastructure and Complex Modules
- [ ] Implement OAuth Module Testing (Phase 4.1)
- [ ] Implement Error Handling Testing (Phase 4.2)
- [ ] Implement Server Module Testing (Phase 5.1)
- [ ] Implement Main Function Testing (Phase 5.2)

## Advanced Testing Techniques
- [ ] Complete Property-Based Testing for data structures
- [ ] Add benchmarks for any remaining critical operations

## Code Quality Improvements
- [ ] Fix compiler warnings in test modules
- [ ] Clean up unused code in test helpers

## Documentation
- [ ] Update `COVERAGE_TRACKING.md` as new tests are implemented
- [ ] Document any additional complexities discovered during testing