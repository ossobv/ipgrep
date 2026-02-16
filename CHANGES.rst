Changelog
=========

All notable changes to this project will be documented here.

The format is inspired by Keep a Changelog and adheres to
[Semantic Versioning](https://semver.org/).

0.2.0 (2026-02-16)
------------------
**Added**
- IP classes like "global", "ip4", "ip6", "rfc1918".
- By using '!' before a needle, you can exclude certain matches/networks.
- Alternatively ipgrep -v inverts the match, exactly like regular grep.
- The -O option truncates IP output to the specified prefix. Useful when
  grouping IPs by /24 for instance.

**Changed**
- Previously match-mode was 'contains' by default. Now it's 'auto', which
  chooses the mode based off the needles. If any needle is larger than a
  single IP, match-mode 'auto' will select 'within' instead.

0.1.3 (2025-10-19)
------------------
**Fixed**
- Corrected 1.2.3.4:443 IP matching (skipping port) by customizing the
  scanner. After prefilter, the scanner has slowed down 50% though. Some
  future work on this might be warranted. Benchmark code has been added
  to the source to aid with this.

0.1.2 (2025-10-16)
------------------
**Added**
- build-info.json to Dockerized deb build. This should make it easier
  to create a reproducible build.

0.1.1 (2025-10-16)
------------------
**Added**
- Initial release!
