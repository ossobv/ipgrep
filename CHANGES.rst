Changelog
=========

All notable changes to this project will be documented here.

The format is inspired by Keep a Changelog and adheres to
[Semantic Versioning](https://semver.org/).

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
