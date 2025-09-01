# Valhalla Microservice Foundations

This crate provides a set of common data structures and traits
to hide all the implementation complexity of building a microservice for Valhalla.

## How Valhalla Microservices Work

If you've only ever run `valhalla_service`, you might not realize that Valhalla is actually a collection of microservices!
While the most Valhalla users just use `valhalla_service` (which creates all the microservices in a single process),
you can also run each microservice individually.
The original reason for this was to enable independent service scalability,
but it also provides an extension point which allows for alternate implementations of specific services.
Pretty clever design!

All communication happens via ZeroMQ.
The classic `valhalla_service` is a single process running all the services,
but the communication is over UNIX sockets by default, and can even use other transport supported by ZeroMQ.

On top of ZeroMQ as the base layer,
Valhalla uses [`prime_server`](https://github.com/kevinkreiser/prime_server)
to define "workers" (the service entry point + event processing loop),
and handle the work distribution.

In addition to the standard Valhalla Protobuf messages,
`prime_server` also includes some other messages like `http_request_info_t`.
Both the `prime_server` C++ helpers and this crate abstract away the details
so you can focus on the service logic.

## Crate Design Choices

- We do not wrap the underlying C++ libraries; everything is implemented in Rust
- We're using the pure Rust implementation of ZeroMQ, even though it's experimental
  (this really just means it doesn't have feature parity, not that it is unsafe to use)
- We use [tracing](https://github.com/tokio-rs/tracing), and enable configuration with [`RUST_LOG`](https://docs.rs/env_logger/latest/env_logger/#enabling-logging)