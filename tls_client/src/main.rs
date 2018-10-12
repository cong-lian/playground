#![deny(warnings)]

#[macro_use]
extern crate serde_derive;

extern crate futures;
extern crate hyper;
extern crate hyper_rustls;
extern crate rustls;
extern crate tls_client;
extern crate tokio_core;

extern crate serde_json;
use hyper::{Body, Request, Uri};
use hyper::rt::Future;
use futures::Stream;
use futures::future;
use std::env;
use std::str::FromStr;
use tls_client::start_client;

// the same as AdvancedAsynchronousMonotonicCounter
// just for putting it into HTTP body as json format
#[derive(Serialize, Deserialize, Debug)]
pub struct AdvancedAsynchronousMonotonicCounter2 {
    pub current: usize, // current value, initialized with 0, strictly increasing, cannot be rolled back
                        // need more fields
                        // ToDo
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonToServer {
    pub key: usize,
    pub previous: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonFromServer {
    pub key: usize,
    pub counter: AdvancedAsynchronousMonotonicCounter2,
}
fn main() {
    // First parameter is target URL (mandatory)
    let url = match env::args().nth(1) {
        Some(ref url) => Uri::from_str(url).expect("well-formed URI"),
        None => {
            println!("Usage: aamcs_client url certificate private_key ca_store");
            return;
        }
    };

    // Second parameter is client certificate (mandatory)
    let cert = match env::args().nth(2) {
        Some(ref path) => {
            println!("client certificate: {}", path);
            path.to_owned()
        }
        None => {
            println!("Please provide certificate");
            println!("Usage: aamcs_client url certificate private_key ca_store");
            return;
        }
    };

    // Third parameter is client private key (mandatory)
    let rsa = match env::args().nth(3) {
        Some(ref path) => {
            println!("private key: {}", path);
            path.to_owned()
        }
        None => {
            println!("Please provide private_key");
            println!("Usage: aamcs_client url certificate private_key ca_store");
            return;
        }
    };

    // Fourth parameter is custom Root-CA store (mandatory)
    let ca = match env::args().nth(4) {
        Some(ref path) => {
            println!("Root-CA store: {}", path);
            path.to_owned()
        }
        None => {
            println!("Please provide Root-CA store");
            println!("Usage: aamcs_client url certificate private_key ca_store");
            return;
        }
    };
    // type `hyper::Client<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>`
    let client = start_client(&cert, &rsa, &ca);

    // a GET request
    // type `hyper::client::ResponseFuture`
    let get = client
        .request(
            Request::get(url.clone())
                .header("Content-Type", "application/json")
                .body({
                    let json_to_server: JsonToServer = JsonToServer {
                        key: 0,
                        previous: 0,
                    };
                    let serialized = serde_json::to_string(&json_to_server).unwrap();
                    Body::from(serialized)
                }).unwrap(),
        ).and_then(|res| {
            println!("received a response :");
            println!("Status: {}", res.status());
            println!("Headers:\n{:#?}", res.headers());

            // issue: empty body, need to fix, get the complete body correctly
            // ToDo
            let entire_body = res.into_body().concat2();
            println!("Body:\n{:#?}", entire_body);
            println!("\n");
            future::ok(())
        });

    let mut core = tokio_core::reactor::Core::new().unwrap();
    if let Err(err) = core.run(get) {
        println!("FAILED: {}", err);
        std::process::exit(1)
    }
}
