# Valinor

Oxidized Valhalla: "dude, maybe we should rewrite it in Rust?"

In the spirit of mythology, this project is cheekily named [Valinor](https://en.wikipedia.org/wiki/Valinor).

## Goals

* Explore platform- and CPU architecture-independent approaches to the principles of Valhalla
* Offer a safer interface to Valhalla's data structures than `libvalhalla`
* Improve the ergonomics of routing on mobile (ex: via UniFFI)
* Explore ways to improve graph tile generation (ex: better parallelism and incremental updates)
* Memory safety (note that this project _will_ still use limited amounts of unsafe code though,
  as it must interpret raw memory as graph tile structures)
* Explore safer, more extensible approaches to Valhalla's dynamic costing (ex: via WASM components)
* Improve the quality and safety of Valhalla (re-implementing ideas in Rust will undoubtedly expose issues)
