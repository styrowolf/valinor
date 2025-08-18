# valhalla-graphtile

This crate exposes a safe interface for reading Valhalla graph tiles.

## Goals

- Zero-copy (using the eponymous crate from Google for most of the heavy lifting)
- Safe public interface
- Maintainability and testability improvements over the original C++ implementation
- Portability, where reasonable (e.g. this is, as far as we know, implementation that will work on a Big Endian CPU architecture)

## Testing

This crate heavily leverages snapshot tests in addition to regular unit tests.
The test suite covers reading graph tiles from a test fixture (a real tile) and exercises the entire public interface
to minimize the risk of non-obvious bugs being introduced.

Additionally, all tests are run in CI, both normally and under Miri to detect any latent undefined behavior (since this sort of packed binary format casting is rather tricky).
Tests are also run under the `s390x-unknown-linux-gnu` target (via QEMU; both regular tests and Miri)
to ensure that everything works on a big endian architecture.

## TODO list

- A lot of fields aren't really fully "mapped" yet because the initial use cases don't need them. This is mostly related to public transit. These are clearly marked with TODOs.
- Support for modifying a graph tile and saving it back to disk (probably easier than a from-scratch write)
- Support for writing graph tiles from scratch (maybe a builder pattern)