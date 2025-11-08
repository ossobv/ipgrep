use std::hint::black_box;

use criterion::measurement::WallTime;
use criterion::{BenchmarkGroup, Criterion, criterion_group, criterion_main};

use regex::bytes::Regex;
use zerocopy::IntoBytes;

use ipgrep::matching::{AcceptSet, InterfaceMode};
use ipgrep::net::Net;
use ipgrep::scanner::{NetCandidate, NetCandidateScanner};

fn bench_netcandidatescanner(c: &mut Criterion) {
    let mut group = c.benchmark_group("NetCandidateScanner");

    // duration | slow  | method              | remarks
    // --------:|------:|---------------------|--------
    //    680ns |   win | Regex               |
    //    860ns |  +25% | NetCandidateScanner |
    bench_dataset(
        &mut group,
        "Find 5 IPs in short line",
        &dup::<1, _>(
            b"
            some random text 192.168.0.1/32 and
            ::1/128, ::ffff:10.0.0.2/127, fd4e:3732:3033::/64
            and ports 100.200.300.400, 40.30.20.10/32
            end
            ",
        ),
        &dup::<1, _>(&vec![
            "192.168.0.1/32",
            "::1/128",
            "::ffff:10.0.0.2/127",
            "fd4e:3732:3033::/64",
            "40.30.20.10/32",
        ]),
    );

    // duration | slow  | method              | remarks
    // --------:|------:|---------------------|--------
    //    660us |   win | Regex               |
    //    850us |  +25% | NetCandidateScanner |
    bench_dataset(
        &mut group,
        "Find 5000 IPs in long line",
        &dup::<1000, _>(
            b"
            some random text 192.168.0.1/32 and
            ::1/128, ::ffff:10.0.0.2/127, fd4e:3732:3033::/64
            and ports 100.200.300.400, 40.30.20.10/32
            end
            ",
        ),
        &dup::<1000, _>(&vec![
            "192.168.0.1/32",
            "::1/128",
            "::ffff:10.0.0.2/127",
            "fd4e:3732:3033::/64",
            "40.30.20.10/32",
        ]),
    );

    // duration | slow  | method              | remarks
    // --------:|------:|---------------------|--------
    //    170us | +900% | Regex               |
    //     17us |   win | NetCandidateScanner | prefilter shines here
    bench_dataset(
        &mut group,
        "Quickly skip line that has no IP-like data",
        &dup::<1000, _>(
            b"
            some random text but.not.stuff
            that looks like IP addresses: this is skipped
            quickly
            ",
        ),
        &vec![],
    );

    // duration | slow  | method              | remarks
    // --------:|------:|---------------------|--------
    //    170us |   win | Regex               |
    //    420us | +150% | NetCandidateScanner | prefilter fails here
    bench_dataset(
        &mut group,
        "This unfortunately does look like an IP",
        &dup::<1000, _>(
            b"
            some random numeric 1.2.3 that
            is not an IP address, but it will trigger the
            match
            ",
        ),
        &vec![],
    );

    group.finish();
}

fn dup<const N: usize, T: Clone>(chunk: &[T]) -> Vec<T> {
    let mut buf = Vec::with_capacity(N * chunk.len());
    for _ in 0..N {
        buf.extend_from_slice(chunk);
    }
    buf
}

fn bench_dataset(
    group: &mut BenchmarkGroup<'_, WallTime>,
    label: &str,
    data: &Vec<u8>,
    expected: &Vec<&str>,
) {
    // Test regex baseline.
    group.bench_function(format!("{label} - Regex baseline"), |b| {
        let data = data.as_bytes();

        // This basic regex is not good enough to handle all our corner
        // cases. But it serves as a nice base line to compare against.
        let re = Regex::new(
            r"(?x)
                (?:
                    # IPv4
                    ((?:\d{1,3}\.){3}(?:\d{1,2}|[12]\d\d)
                    (?:/\d{1,2})?)
                    \b
                )
                |
                (?:
                    # IPv4-mapped IPv6
                    (::[fF]{4}:
                    (?:\d{1,3}\.){3}\d{1,3})
                    (?:/\d{1,3})?
                )
                |
                (?:
                    # IPv6
                    ([0-9a-fA-F:]+:[0-9a-fA-F:]*)
                    (?:/\d{1,3})?
                    \b
                )
            ",
        )
        .unwrap();

        // Setup. This is also slightly less good than the
        // NetCandidateScanner because this one does not record the
        // positions in the string.
        let re_fn = |data: &[u8]| -> Vec<NetCandidate> {
            re.find_iter(data)
                .map(|m| NetCandidate {
                    range: (m.start(), m.end()),
                    net: Net::try_from(m.as_bytes()).unwrap(),
                })
                .collect()
        };

        // Do preliminary test
        let netcandidates = re_fn(&data);
        let net_strs: Vec<String> =
            netcandidates.iter().map(|c| c.net.to_string()).collect();
        assert_eq!(net_strs, *expected);

        // Do timing
        b.iter(|| {
            let netcandidates = re_fn(black_box(data));
            black_box(netcandidates)
        });
    });

    // Test our custom hand-made scanner. When it is not faster than the
    // regex, it is at least more feature complete.
    group.bench_function(format!("{label} - NetCandidateScanner"), |b| {
        let data = data.as_bytes();

        // Setup
        let acc = AcceptSet {
            ip: true,
            net: true,
            oldnet: false,
            iface: true,
        };
        let ncs = NetCandidateScanner::new()
            .set_accept(acc)
            .set_interface_mode(InterfaceMode::TreatAsIp);

        // Do preliminary test
        let netcandidates = ncs.find_all(&data, "(stdin)");
        let net_strs: Vec<String> =
            netcandidates.iter().map(|c| c.net.to_string()).collect();
        assert_eq!(net_strs, *expected);

        // Do timing
        b.iter(|| {
            let netcandidates = ncs.find_all(&black_box(data), "(stdin)");
            black_box(netcandidates);
        });
    });
}

criterion_group!(benches, bench_netcandidatescanner);
criterion_main!(benches);
