#![deny(warnings)]

extern crate hyper;
extern crate hyper_rustls;
extern crate rustls;
extern crate tokio_core;

use hyper::client;
use rustls::internal::pemfile;
use std::{fs, io};

fn load_certs(filename: &str) -> Vec<rustls::Certificate> {
    let certfile = fs::File::open(filename).expect("cannot open certificate file");
    let mut reader = io::BufReader::new(certfile);
    pemfile::certs(&mut reader).unwrap()
}

fn load_private_key(filename: &str) -> rustls::PrivateKey {
    let keyfile = fs::File::open(filename).expect("cannot open private key file");
    let mut reader = io::BufReader::new(keyfile);
    let keys = pemfile::rsa_private_keys(&mut reader).unwrap();
    assert!(keys.len() == 1);
    keys[0].clone()
}

pub fn start_client(cert: &str, rsa: &str, root_cert: &str) -> client::Client<hyper_rustls::HttpsConnector<client::HttpConnector>, hyper::Body>{
    let https = {
        // takes number of DNS worker threads
        let mut http = client::HttpConnector::new(4);
        http.enforce_http(false);
        let mut tls = rustls::ClientConfig::new();
        let certs = load_certs(&cert);
        let privkey = load_private_key(&rsa);

        tls.set_single_client_cert(certs, privkey);
        let f = fs::File::open(root_cert).unwrap();
        let mut rd = io::BufReader::new(f);
        tls.root_store.add_pem_file(&mut rd).unwrap();
        hyper_rustls::HttpsConnector::from((http, tls))
    };
    client::Client::builder().build::<_, hyper::Body>(https)
}
