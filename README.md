<div align="center">
  <h1>linked-list</h1>
  <p>
    <strong>A simple linked list.</strong>
  </p>
  <p>

[![crates.io][crates.io shield]][crates.io link]
[![Documentation][docs.rs badge]][docs.rs link]
![Rust CI][github ci badge]
![Minimum Supported Rustc Version][rustc 1.63+]
<br />
<br />
[![Dependency Status][deps.rs status]][deps.rs link]
[![Download Status][shields.io download count]][crates.io link]

  </p>
</div>

[crates.io shield]: https://img.shields.io/crates/v/linked-list?label=latest
[crates.io link]: https://crates.io/crates/linked-list
[docs.rs badge]: https://docs.rs/linked-list/badge.svg?version=0.0.3
[docs.rs link]: https://docs.rs/linked-list/0.0.3/linked_list/
[github ci badge]: https://github.com/contain-rs/linked-list/workflows/Rust/badge.svg?branch=master
[rustc 1.63+]: https://img.shields.io/badge/rustc-1.63%2B-blue.svg
[deps.rs status]: https://deps.rs/crate/linked-list/0.0.3/status.svg
[deps.rs link]: https://deps.rs/crate/linked-list/0.0.3
[shields.io download count]: https://img.shields.io/crates/d/linked-list.svg

## Usage

Add this to your Cargo.toml:

```toml
[dependencies]
linked-list = "0.0.3"
```

Since Rust 2018, `extern crate` is no longer mandatory. If your edition is old (Rust 2015),
add this to your crate root:

```rust
extern crate linked_list;
```

If you want [serde](https://github.com/serde-rs/serde) support, include the feature like this:

```toml
[dependencies]
linked-list = { version = "0.0.3", features = ["serde"] }
```

<!-- cargo-rdme start -->

### Description

An alternative implementation of standard `LinkedList` featuring a prototype `Cursor`.

<!-- cargo-rdme end -->

## License

Dual-licensed for compatibility with the Rust project.

Licensed under the Apache License Version 2.0: http://www.apache.org/licenses/LICENSE-2.0,
or the MIT license: http://opensource.org/licenses/MIT, at your option.
