use codspeed_criterion_compat::{black_box, criterion_group, criterion_main, Criterion};
use gamacros_workspace::parse_profile;

fn bench_parse_profile(c: &mut Criterion) {
    // Use a real-world profile from the repository root
    let yaml: &str = include_str!("../../../gc_profile.yaml");

    c.bench_function("workspace_parse_profile_gc_profile", |b| {
        b.iter(|| {
            let input = black_box(yaml);
            let profile = parse_profile(input).expect("profile should parse");
            black_box(profile);
        })
    });
}

criterion_group!(benches, bench_parse_profile);
criterion_main!(benches);
