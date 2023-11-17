[![Build & Test][github-actions-badge]][github-actions-link]
[![crates.io][crate-badge]][crate-link]
[![Docs][docs-badge]][docs-link]

# libmudtelnet

A low-level Telnet protocol implementation for MUD clients, written in Rust.

`libmudtelnet` is a fork of [libtelnet-rs], which is itself inspired by the [libtelnet].

# Changelog

See [CHANGELOG.md](CHANGELOG.md).

# Usage

Check [src/tests.rs](tests/tests.rs) for an example parser. For a larger
example, see the [Blightmud] MUD client that uses `libtelnet-rs` for its Telnet
handling.

First, construct a parser with [`Parser::new()`][new-parser]. Ideally, you would
place this parser somewhere directly behind a socket or external source of data.

When data comes in from the socket, immediately send it into the parser with
[`parser.receive(data)`][receive]. This will append it to the current internal
buffer, and process and return any [telnet events] to be looped over and handled
as your application requires.

Any text to be sent back over the socket to the remote end should be sent
through [`parser.send_text(data)`][send-text] to ensure data will be encoded
properly for the telnet protocol. Data to be sent will be provided either by
a `events::TelnetEvents::DataSend` event after processing, or as a return from
any method used for sending data.

# Compatibility

The initial release of `libmudtelnet` has been tested for compatibility with
`libtelnet-rs`. In general while much of the code has been rewritten to be more
idiomatic Rust, the API is the same and breaking changes have been avoided. An
upcoming semver incompatible release will be made with broader API changes in
the near future.

See [CHANGELOG.md](CHANGELOG.md) for more details.

# Credits

Many thanks to:

* [envis10n] for his work on [libtelnet-rs], which `libmudtelnet` is forked from.
* [Sean Middleditch] for his work on [libtelnet], which inspired `libtelent-rs`.

[github-actions-badge]: https://img.shields.io/github/actions/workflow/status/blightmud/libmudtelnet/rust.yml?label=build%20%26%20tests&logo=github&style=for-the-badge&branch=main
[github-actions-link]: https://github.com/blightmud/libmudtelnet/actions/workflows/rust.yml?query=branch%3Amain
[crate-badge]: https://img.shields.io/crates/v/libmudtelnet.svg?style=for-the-badge
[crate-link]: https://crates.io/crates/libmudtelnet
[docs-badge]: https://img.shields.io/badge/docs-rs-blue.svg?style=for-the-badge
[docs-link]: https://docs.rs/libmudtelnet
[libtelnet-rs]: https://github.com/envis10n/libtelnet-rs
[libtelnet]: https://github.com/seanmiddleditch/libtelne
[Blightmud]: https://github.com/blightmud/blightmud
[new-parser]: https://docs.rs/libtelnet-rs/latest/libtelnet_rs/struct.Parser.html#method.new
[receive]: https://docs.rs/libtelnet-rs/latest/libtelnet_rs/struct.Parser.html#method.receive
[telnet events]: https://docs.rs/libtelnet-rs/latest/libtelnet_rs/events/enum.TelnetEvents.html
[send-text]: https://docs.rs/libtelnet-rs/latest/libtelnet_rs/struct.Parser.html#method.send_text
[envis10n]: https://github.com/envis10n/
[Sean Middleditch]: https://github.com/seanmiddleditch/
