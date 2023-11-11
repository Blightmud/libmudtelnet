# v2.0.1 (pending)

Initial release of `libmudtelnet` - a fork of `libtelnet-rs`.

## Bug fixes

* A `SE` byte that isn't preceded by `IAC` is now properly handled as a normal
  byte during Telnet subnegotiation processing.
* Fixed a panic when Telnet option code 0xFF is negotiated, and a truncated 
  subnegotiation (e.g. `IAC SB IAC SE`) is received.
* Multiple escaped `IAC` bytes (`e.g. IAC IAC IAC IAC`) are now properly 
 unescaped (e.g. `IAC IAC`) instead of truncated (e.g. `IAC`). 

## Features

* Many API types now derive helpful traits (`Debug`, `Eq`, etc.).
* An optional `arbitrary` crate feature is now available to enable generating 
  arbitrary `event::TelnetIAC`, `event::TelnetNegotiation` and `event::TelnetSubnegotiation` 
  instances for testing.

## Misc

* CI improvements.
* Fuzz testing.
* Simple benchmarking.
* Small performance and safety improvements (avoiding direct indexing, etc.).

# v2.0.0

As of version 2.0, `libtelnet_rs` has switched over to using the `bytes`
[crate](https://crates.io/crates/bytes).

With this change, the method signatures for most methods that return `Vec<u8>`
now return `Bytes` instead.

For most situations where a `Vec<u8>` would be required, the returned value can
simply be changed to utilize `Bytes::to_vec()`.

To make usage of the new dependency a little bit easier, it is also re-exported
as `libtelnet_rs::bytes`.
