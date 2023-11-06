#![no_main]

use libfuzzer_sys::fuzz_target;

use compat::{test_app, TelnetApplication};

fuzz_target!(|app: TelnetApplication| {
  test_app(&app);
});
