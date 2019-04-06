extern crate trust_dns;
extern crate tokio;

use std::net::{Ipv4Addr, SocketAddr};
use std::str::FromStr;
use tokio::runtime::current_thread::Runtime;

use trust_dns::udp::UdpClientStream;
use trust_dns::client::{Client, ClientFuture, ClientHandle};
use trust_dns::rr::{DNSClass, Name, RData, Record, RecordType};
// use trust_dns::client::client_future::ClientResponse;
use trust_dns::op::ResponseCode;
use trust_dns::rr::rdata::key::KEY;

// pub fn resolve_domain(domain: &str, result: &mut Vec<String>) {
pub fn resolve_domain(domains: &Vec<&str>, result: &mut Vec<String>) {
// We'll be using the current threads Tokio Runtime
let mut runtime = Runtime::new().unwrap();

// We need a connection, TCP and UDP are supported by DNS servers
//   (tcp construction is slightly different as it needs a multiplexer)
let stream = UdpClientStream::new(([8,8,8,8], 53).into());

// Create a new client, the bg is a background future which handles
//   the multiplexing of the DNS requests to the server.
//   the client is a handle to an unbounded queue for sending requests via the
//   background. The background must be scheduled to run before the client can
//   send any dns requests
let (bg, mut client) = ClientFuture::connect(stream);

// run the background task
runtime.spawn(bg);

let mut futures = Vec::with_capacity(domains.len());

for domain in domains {
  // Create a query future
  futures.push(client.query(Name::from_str(domain).unwrap(), DNSClass::IN, RecordType::A));
}

for query_future in futures {
  // wait for its response
  let response = runtime.block_on(query_future).unwrap();
  for a in response.answers() {
    if let &RData::CNAME(name) = &a.rdata() {
      result.push(name.to_string());
    }
  }
}

// validate it's what we expected
// if let &RData::A(addr) = response.answers()[0].rdata() {
//     assert_eq!(addr, Ipv4Addr::new(93, 184, 216, 34));
// }
}

#[test]
fn test_resolver() {
    let mut result: Vec<String> = Vec::new();
    resolve_domain(&vec!["www.bax-shop.nl", "www.googleadservices.com"], &mut result);
    for s in result {
        let d = &s[.. s.len() - 1];
        println!("CNAME: {}", d);
    }
}