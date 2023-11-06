use arbitrary::{Arbitrary, Unstructured};
use bencher::{benchmark_group, benchmark_main, Bencher};
use libmudtelnet::bytes::Bytes;
use libmudtelnet::compatibility::CompatibilityEntry;
use libmudtelnet::events::{TelnetIAC, TelnetNegotiation, TelnetSubnegotiation};
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};
use std::hint::black_box;

#[derive(Debug, Arbitrary)]
enum SimulationEvents {
  IAC(TelnetIAC),
  Negotiation(TelnetNegotiation),
  Subnegotiation(TelnetSubnegotiation),
  Data(Vec<u8>),
}

impl SimulationEvents {
  fn to_bytes(&self) -> Bytes {
    // TODO(XXX): to_bytes() shouldn't require moving, we could drop clones here. Or maybe there
    //   should be an as_ref() for each event?
    match self {
      SimulationEvents::IAC(iac) => iac.clone().to_bytes(),
      SimulationEvents::Negotiation(neg) => neg.clone().to_bytes(),
      SimulationEvents::Subnegotiation(sub) => sub.clone().to_bytes(),
      SimulationEvents::Data(data) => Bytes::copy_from_slice(data),
    }
  }
}

const SEED: [u8; 32] = [
  0xb8, 0x5a, 0x8d, 0xcf, 0xb9, 0x9f, 0x1a, 0x2a, 0x6, 0xf8, 0x46, 0x14, 0xee, 0x5f, 0x69, 0xae,
  0x44, 0x23, 0x36, 0xf9, 0xc0, 0x40, 0x57, 0xd8, 0x62, 0x7f, 0x98, 0x71, 0xe2, 0xba, 0x76, 0xf9,
];

fn bench_receive(b: &mut Bencher) {
  let mut data_buf = [0; 1024 * 1024];
  StdRng::from_seed(SEED).fill_bytes(&mut data_buf);

  b.iter(|| {
    let mut parser = libmudtelnet::Parser::default();
    for i in 0..255 {
      parser.options.set_option(
        i,
        CompatibilityEntry {
          local: true,
          remote: true,
          local_state: false,
          remote_state: false,
        },
      );
    }

    const INPUT_EVENTS: usize = 10_000;
    for _ in 0..INPUT_EVENTS {
      let input_event = SimulationEvents::arbitrary(&mut Unstructured::new(&data_buf)).unwrap();
      black_box(parser.receive(&input_event.to_bytes()));
    }
  });
}

fn bench_og_receive(b: &mut Bencher) {
  let mut data_buf = [0; 1024 * 1024];
  StdRng::from_seed(SEED).fill_bytes(&mut data_buf);

  b.iter(|| {
    let mut parser = libtelnet_rs::Parser::default();
    for i in 0..255 {
      parser.options.set_option(
        i,
        libtelnet_rs::compatibility::CompatibilityEntry {
          local: true,
          remote: true,
          local_state: false,
          remote_state: false,
        },
      );
    }

    const INPUT_EVENTS: usize = 10_000;
    for _ in 0..INPUT_EVENTS {
      let input_event = SimulationEvents::arbitrary(&mut Unstructured::new(&data_buf)).unwrap();
      black_box(parser.receive(&input_event.to_bytes()));
    }
  });
}

benchmark_group!(parser_benches, bench_receive, bench_og_receive);

benchmark_main!(parser_benches);
