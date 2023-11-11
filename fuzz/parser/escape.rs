#![no_main]

use libfuzzer_sys::fuzz_target;

use compat::test_escape;

fuzz_target!(|data: Vec<u8>| {
  test_escape(data);
});
