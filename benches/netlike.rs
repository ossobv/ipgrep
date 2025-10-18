use std::hint::black_box;

use criterion::measurement::WallTime;
use criterion::{BenchmarkGroup, Criterion, criterion_group, criterion_main};
use regex::bytes::Regex;

use ipgrep::netlike::NetLikeScanner;

fn bench_netlike(c: &mut Criterion) {
    let mut group = c.benchmark_group("NetLikeScanner");

    // During the first tests, I got these:
    // - 4.0ms for Regex test
    // - 3.5ms for NetLikeScanner test
    // Not earth shattering, but it's about 15% (and better),
    // so good enough for now.
    bench_dataset(&mut group, "big", &make_big_input(), 60_000);

    // - 100us for Regex test
    // -  40us for NetLikeScanner test
    bench_dataset(&mut group, "bogus-ipv4", &make_bogus_ipv4_input(), 0);
    bench_dataset(&mut group, "just-one", &make_just_one_hit(), 1);

    // These are not realistic, as we're using the prefilter code to
    // find either ":[0-9a-fA-:]" or "[0-9].[0-9]".
    // The Regex matcher here wins on all except the a's, because the
    // (currently used test) regex is unbounded.
    //bench_dataset(&mut group, "periods", &vec![b'.'; 100_000], 0);
    //bench_dataset(&mut group, "z's", &vec![b'z'; 100_000], 0);
    //bench_dataset(&mut group, "a's", &vec![b'a'; 100_000], 0);

    group.finish();
}

fn make_big_input() -> Vec<u8> {
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

fn make_bogus_ipv4_input() -> Vec<u8> {
    let chunk = b"1.2.3.4.";
    let copies = 10_000;
    let mut buf = Vec::with_capacity(copies * chunk.len());
    for _ in 0..copies {
        buf.extend_from_slice(chunk);
    }
    buf
}

fn make_just_one_hit() -> Vec<u8> {
    let chunk = b"
        some random text without any ipv4 or ipv6 anywhere;
        although there are some 1.2 numbers and maybe \":a colon:\" or two
    ";
    let copies = 1; //5_001;
    let mut buf = Vec::with_capacity(copies * chunk.len());
    for _ in 0..copies {
        buf.extend_from_slice(chunk);
    }
    // Add a single result at the end.
    buf.extend_from_slice(b"::");
    buf
}

fn bench_dataset(
    group: &mut BenchmarkGroup<'_, WallTime>,
    label: &str,
    data: &[u8],
    expected_count: usize,
) {
    // This basic regex is likely not good enough to handle all our
    // corner cases. But it serves as a nice base line to compare
    // against.
    let re = Regex::new(
        r"(?x)
            (?:
                # IPv4
                ((?:\d{1,3}\.){3}\d{1,3})
                (?:\b|[^0-9.])
                (?:/\d{1,2})?
                # quickly enforce no 1.2.3.4.5
                [^.]
            )
            |
            (?:
                # IPv4-mapped IPv6
                (::[fF]{4}:
                (?:\d{1,3}\.){3}\d{1,3})
                (?:\b|[^0-9.])
                (?:/\d{1,3})?
            )
            |
            (?:
                # IPv6
                ([0-9a-fA-F:]+:[0-9a-fA-F:]*)
                (?:/\d{1,3})?
            )
        ",
    )
    .unwrap();

    // Test regex baseline.
    group.bench_function(format!("{label} - Regex baseline"), |b| {
        let mut result = 0;
        b.iter(|| {
            let mut count = 0;
            for _ in re.find_iter(black_box(data)) {
                count += 1;
            }
            result = count;
            black_box(count)
        });
        assert_eq!(result, expected_count);
    });

    // Test our custom hand-made scanner. It should be faster than the
    // regex for many data sets.
    group.bench_function(format!("{label} - NetLikeScanner"), |b| {
        let mut result = 0;
        b.iter(|| {
            let mut s = NetLikeScanner::new(black_box(data));
            let mut count = 0;
            while let Some((_s, _e)) = s.next() {
                count += 1;
            }
            result = count;
            black_box(count)
        });
        assert_eq!(result, expected_count);
    });
}

criterion_group!(benches, bench_netlike);
criterion_main!(benches);
