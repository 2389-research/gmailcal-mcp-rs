use criterion::{black_box, criterion_group, criterion_main, Criterion};
use mcp_gmailcal::errors::GmailCalError;

fn error_benchmarks(c: &mut Criterion) {
    // Benchmark error creation
    c.bench_function("create_gmail_error", |b| {
        b.iter(|| {
            black_box(GmailCalError::GmailError {
                message: "Test error".into(),
                code: 500,
            })
        })
    });
}

criterion_group!(benches, error_benchmarks);
criterion_main!(benches);
