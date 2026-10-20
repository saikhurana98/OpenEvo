# Notices

OpenEvo is independent open-source software. Audient and EVO are names of
their respective owners. The project is not affiliated with or endorsed by
Audient. The installed Audient driver and device firmware remain dependencies
and are not included in the portable package.

Application source is offered under GPL-3.0-only; see LICENSE. Slint is used
under its GPLv3 option. Cargo.lock records the exact dependency versions.
Dependency source and license metadata can be fetched with Cargo.

- Slint: https://github.com/slint-ui/slint (GPLv3 option selected)
- Rust libloading: https://github.com/nagisa/rust_libloading (ISC)
- Serde / serde_json: https://github.com/serde-rs (MIT or Apache-2.0)
- sha2: https://github.com/RustCrypto/hashes (MIT or Apache-2.0)

The test-only ASIO host uses Steinberg SDK headers under the GPLv3 option.
Preserved notices, exact header hashes, and source provenance are in
tools/asio/README.md and tools/asio/LICENSE.txt. These headers and the ASIO test
host are not compiled into OpenEvo. The SDK mirror is identified as a
mirror, not represented as an official Steinberg repository.

Protocol research references:

- https://github.com/vanzaho/audient-evo-py
- https://github.com/soerenbnoergaard/evoctl
- https://github.com/vijay-prema/audient-evo-linux-tools

The native application's implementation and UI assets are original. Installed
OEM binaries were inspected with REA/Ghidra to establish API behavior. The
evidence ledgers distinguish static reconstruction from runtime observations.
