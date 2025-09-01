# Ilúvatar - An alternative to Valhalla's Odin Service

Sorry (not sorry); couldn't resist the name :P
Be thankful that there is no tortured acronym or attempt to force diacritics everywhere.

## What is this?

This is an attempt at an alternate narrative builder for Valhalla.
Actually, that's not the whole truth...
Odin is a rather complex service that is responsible for, among other things,
response serialization (this nominally lives in Tyr, but Tyr is not a service; it's a library)
and even for the final handling of the `/status` endpoint.

This aims to be a drop-in replacement for Odin,
but written in idiomatic Rust to make it easire to improve, extend, and debug.

## Why?

The narrative builder is pretty arcane.
It works excellently most of the time, but debugging edge cases is a huge pain.

Additionally, we've occasionally extended / modified the response format,
most recently around voice and banner instructions to support turn-by-turn navigation.
These PRs are quite large, painful, and error-prone in C++.

So this is an experimental alternative in Rust.
