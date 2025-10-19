use std::hint::black_box;

use criterion::measurement::WallTime;
use criterion::{BenchmarkGroup, Criterion, criterion_group, criterion_main};
use regex::bytes::Regex;

use ipgrep::netlike::NetLikeScanner;

fn bench_netlike(c: &mut Criterion) {
    let mut group = c.benchmark_group("NetLikeScanner");

    // For the timings below, we have to consider two things:
    // - The actual search code might not reach it because we use
    //   prefiltering by "[0-9][.][0-9]" or "[0-9a-fA-F:]:". If those
    //   aren't found, we won't scan at all.
    // - The regular expression test is incomplete. The NetLikeScanner
    //   will do better matching, which I haven't been able to reproduce
    //   using pure regexes.  (Possibly because they require negative
    //   matches which are not supported.)

    // duration |  slow | method          | remarks
    // --------:|------:|-----------------|--------
    //    4.2ms |  +15% | Regex           |
    //    3.6ms |   win | NetLikeScanner  | faster and better
    bench_dataset(&mut group, "big", &make_big_input(), 60_000);

    // duration | slow  | method          | remarks
    // --------:|------:|-----------------|--------
    //    100us | +170% | Regex           |
    //     37us |   win | NetLikeScanner  | faster and better
    bench_dataset(&mut group, "bogus-ipv4", &make_bogus_ipv4_input(), 0);

    // duration | slow  | method          | remarks
    // --------:|------:|-----------------|--------
    //    190us |   win | Regex           | we'll need SIMD to match this
    //    470us | +150% | NetLikeScanner  |
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
        some random text without any ipv4 or ipv6 anywhere; although
        there are some 1.2.3. numbers and maybe \":a colon:\" or two
    ";
    let copies = 1_000;
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
    // Test regex baseline.
    group.bench_function(format!("{label} - Regex baseline"), |b| {
        // This basic regex is not good enough to handle all our corner
        // cases. But it serves as a nice base line to compare against.
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
