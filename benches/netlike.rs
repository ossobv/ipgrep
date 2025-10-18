use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use regex::bytes::Regex;

use ipgrep::netlike::NetLikeScanner;

fn make_big_input() -> Vec<u8> {
    // Repeat a realistic blob to amortize startup effects.
    let chunk = b"
        some random text 192.168.0.1 and
        ::1, ::ffff:10.0.0.1/127, fd4e:3732:3033::1/64
        and ports 100.200.300.400:80, 40.30.20.10:443
        end
    ";
    let copies = 10_000;
    let mut buf = Vec::with_capacity(copies * chunk.len());
    for _ in 0..copies {
        buf.extend_from_slice(chunk);
    }
    buf
}

/// During the first tests, I got these:
/// - 4.1037ms for Regex test
/// - 3.4930ms for NetLikeScanner test
/// Not earth shattering, but it's 15%-20% faster (and better),
/// so good enough for now.
fn bench_netlike(c: &mut Criterion) {
    let data = make_big_input();
    let expected_count = 60_000;
    let mut group = c.benchmark_group("NetLikeScanner");

    // Test a basic regex. It is likely not good enough to handle all
    // our corner cases. But it serves as a nice base line.
    let re = Regex::new(
        r"(?x)
            (?:
                (?:\d{1,3}\.){3}\d{1,3}
                (?:/\d{1,2})?
            )
            |
            (?:
                ::[fF][fF][fF][fF]:
                (?:
                    (?:\d{1,3}\.){3}\d{1,3}
                    (?:/\d{1,3})?
                )
            )
            |
            (?:
                [0-9a-fA-F:]+:[0-9a-fA-F:]*
                (?:/\d{1,3})?
            )
        ",
    )
    .unwrap();
    group.bench_function("Regex test (baseline)", |b| {
        let mut result = 0;
        b.iter(|| {
            let mut count = 0;
            for _ in re.find_iter(black_box(&data)) {
                count += 1;
            }
            result = count;
            black_box(count)
        });
        assert_eq!(result, expected_count);
    });

    // Test out custom hand-made scanner. It should at least be faster
    // than the regex.
    group.bench_function("NetLikeScanner test", |b| {
        let mut result = 0;
        b.iter(|| {
            let mut s = NetLikeScanner::new(black_box(&data));
            let mut count = 0;
            while let Some((_s, _e)) = s.next() {
                count += 1;
            }
            result = count;
            black_box(count)
        });
        assert_eq!(result, expected_count);
    });

    group.finish();
}

criterion_group!(benches, bench_netlike);
criterion_main!(benches);
