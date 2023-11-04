[![Rust](https://github.com/envis10n/libtelnet-rs/workflows/Rust/badge.svg?branch=master)](https://github.com/envis10n/libtelnet-rs)
[![Crates.io](https://img.shields.io/crates/v/libtelnet-rs)](https://crates.io/crates/libtelnet-rs)
[![Docs.rs](https://docs.rs/libtelnet-rs/badge.svg)](https://docs.rs/libtelnet-rs)

# libtelnet-rs

A [libtelnet]-inspired telnet parser for Rust.

[libtelnet]: https://github.com/seanmiddleditch/libtelnet

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

[Blightmud]: https://github.com/blightmud/blightmud
[new-parser]: https://docs.rs/libtelnet-rs/latest/libtelnet_rs/struct.Parser.html#method.new
[receive]: https://docs.rs/libtelnet-rs/latest/libtelnet_rs/struct.Parser.html#method.receive
[telnet events]: https://docs.rs/libtelnet-rs/latest/libtelnet_rs/events/enum.TelnetEvents.html
[send-text]: https://docs.rs/libtelnet-rs/latest/libtelnet_rs/struct.Parser.html#method.send_text
