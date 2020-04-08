use super::*;
#[test]
fn test_parser() {
  let mut instance: Parser = Parser::new();
  instance.receive(&bytes::concat(b"Hello, rust!", &[255, 249]));
  instance.send_text("Derp Derp Derp!");
  instance.receive(&[255, 253, 201]);
  instance.receive(&[255, 250, 201]);
  instance.receive(b"Core.Hello {}");
  instance.receive(&[255, 240]);
  for ev in instance {
    match ev {
      TelnetEvent::IAC(command) => println!("IAC: {:?}", command),
      TelnetEvent::Negotiation(nev) => println!("Negotiation: {} {}", nev.command, nev.option),
      TelnetEvent::Subnegotiation(sev) => {
        println!("Subnegotiation: {} {:?}", sev.option, sev.buffer)
      }
      TelnetEvent::Data(dev) => println!("Data: {:?}", dev.buffer),
      TelnetEvent::Send(sdev) => println!("Send: {:?}", sdev.buffer),
    }
  }
}
#[test]
fn test_escape() {
  let a: Vec<u8> = vec![255, 250, 201, 255, 205, 202, 255, 240];
  let expected: Vec<u8> = vec![255, 255, 250, 201, 255, 255, 205, 202, 255, 255, 240];
  assert_eq!(expected, Parser::escape_iac(a))
}
#[test]
fn test_unescape() {
  let a: Vec<u8> = vec![255, 255, 250, 201, 255, 255, 205, 202, 255, 255, 240];
  let expected: Vec<u8> = vec![255, 250, 201, 255, 205, 202, 255, 240];
  assert_eq!(expected, Parser::unescape_iac(a))
}