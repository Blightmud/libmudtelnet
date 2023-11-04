# v2.0

As of version 2.0, `libtelnet_rs` has switched over to using the `bytes`
[crate](https://crates.io/crates/bytes).

With this change, the method signatures for most methods that return `Vec<u8>`
now return `Bytes` instead.

For most situations where a `Vec<u8>` would be required, the returned value can
simply be changed to utilize `Bytes::to_vec()`.

To make usage of the new dependency a little bit easier, it is also re-exported
as `libtelnet_rs::bytes`.
