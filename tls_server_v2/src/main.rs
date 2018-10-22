//! Simple HTTPS echo service based on hyper-rustls
//!
//! First parameter is the mandatory port to use.
//! Certificate and private key are hardcoded to sample files.
#![deny(warnings)]

extern crate futures;
extern crate hyper;
extern crate rustls;
extern crate tokio;
extern crate tokio_rustls;
extern crate tokio_tcp;

use futures::future;
use futures::Stream;
use hyper::rt::Future;
use hyper::service::service_fn;
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use rustls::internal::pemfile;
use std::collections::HashMap;
use std::{env, fs, io, str, sync, sync::Arc, sync::Mutex};
use tokio_rustls::ServerConfigExt;

fn main() {
    // Serve an echo service over HTTPS, with proper error handling.
    if let Err(e) = run_server() {
        eprintln!("FAILED: {}", e);
        std::process::exit(1);
    }
}

fn error(err: String) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}

fn run_server() -> io::Result<()> {
    // First parameter is port number (optional, defaults to 1337)
    let port = match env::args().nth(1) {
        Some(ref p) => p.to_owned(),
        None => "1338".to_owned(),
    };
    let addr = format!("127.0.0.1:{}", port)
        .parse()
        .map_err(|e| error(format!("{}", e)))?;

    // Build TLS configuration.
    let tls_cfg = {
        // Load public certificate.
        let certs = load_certs("cert_util/localhost.pem")?;
        // Load private key.
        let key = load_private_key("cert_util/localhost.key")?;
        // Do not use client certificate authentication.
        let mut cfg = rustls::ServerConfig::new(rustls::NoClientAuth::new());
        // Select a certificate to use.
        cfg.set_single_cert(certs, key)
            .map_err(|e| error(format!("{}", e)))?;
        sync::Arc::new(cfg)
    };

    // Create a TCP listener via tokio.
    let tcp = tokio_tcp::TcpListener::bind(&addr)?;

    // Prepare a long-running future stream to accept and serve cients.
    let tls = tcp
        .incoming()
        .and_then(move |s| tls_cfg.accept_async(s))
        .then(|r| match r {
            Ok(x) => Ok::<_, io::Error>(Some(x)),
            Err(_e) => {
                println!("[!] Voluntary server halt due to client-connection error...");
                // Errors could be handled here, instead of server aborting.
                // Ok(None)
                Err(_e)
            }
        }).filter_map(|x| x);
    // Build a hyper server, which serves our custom echo service.
    let state = SharedState {
        request_counter: HashMap::new(),
        service_name: "Counting".to_string(),
    };

    let request_counter = Arc::new(Mutex::new(state));
    let fut = Server::builder(tls).serve(move || {
        let inner = Arc::clone(&request_counter);
        service_fn(move |req| echo(req, &inner))
    });

    // Run the future, keep going until an error occurs.
    println!("Starting to serve on https://{}.", addr);
    let mut rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(fut).map_err(|e| error(format!("{}", e)))?;
    Ok(())
}

// Future result: either a hyper body or an error.
type ResponseFuture = Box<Future<Item = Response<Body>, Error = hyper::Error> + Send>;

struct SharedState {
    request_counter: HashMap<String, Record>,
    service_name: String,
}
struct Record {
    number: usize,
    content: String,
}

fn record(state: &Arc<Mutex<SharedState>>, request: &str, content: &str) -> String {
    match request {
        "GET" => {
            let state = state.lock().unwrap();
            if !state.request_counter.contains_key(&"GET".to_string()) {
                state.request_counter.insert(
                    "GET".to_string(),
                    Record {
                        number: 1,
                        content: content.to_string(),
                    },
                );
            } else if let Some(x) = state.request_counter.get_mut(&"GET".to_string()) {
                x.number += 1;
            }
            "".to_string()
        }
        "POST" => {
            let state = state.lock().unwrap();
            // when entry not exist yet
            if !state.request_counter.contains_key(&"POST".to_string()) {
                state.request_counter.insert(
                    "POST".to_string(),
                    Record {
                        number: 1,
                        content: content.to_string(),
                    },
                );
            // already exist, update the entry, append content
            } else if let Some(x) = state.request_counter.get_mut(&"POST".to_string()) {
                x.number += 1;
                x.content.push_str("\n");
                x.content.push_str(content);
            }
            state
                .request_counter
                .get(&"POST".to_string())
                .unwrap()
                .content
        }
        "OTHERS" => {
            let state = state.lock().unwrap();
            if !state.request_counter.contains_key(&"OTHERS".to_string()) {
                state.request_counter.insert(
                    "OTHERS".to_string(),
                    Record {
                        number: 1,
                        content: content.to_string(),
                    },
                );
            } else if let Some(x) = state.request_counter.get_mut(&"OTHERS".to_string()) {
                x.number += 1;
            }
            "".to_string()
        }
        _ => "".to_string(),
    }
}

// Custom echo service, handling two different routes and a
// catch-all 404 responder.
fn echo(req: Request<Body>, counter: &Arc<Mutex<SharedState>>) -> ResponseFuture {
    let (parts, body) = req.into_parts();
    println!("{:?}", parts);

    {
        let mut counter = counter.lock().unwrap();
        println!("Service Name: {}", counter.service_name);
        for (key, value) in &counter.request_counter {
            println!(
                "Request Type : {} \nN0. : {} \nContent: {}\n",
                key, value.number, value.content
            );
        }
    }

    match (parts.method, parts.uri.path()) {
        // Help route.
        (Method::GET, "/") => {
            // for GET request, only count the number, ignore content
            let _ = record(counter, "GET", "");
            Box::new(future::ok(
                Response::builder()
                    .body(Body::from("Try POST /echo\n"))
                    .unwrap(),
            ))
        }
        // Echo service route.
        (Method::POST, "/echo") => {
            // for POST request, record the number and concatenate the content
            let entire_body = body.concat2();
            let res = entire_body.and_then(|body| {
                println!("Body:\n{}", str::from_utf8(&body).unwrap());
                println!("\n");
                let accumulated = record(counter, "POST", str::from_utf8(&body).unwrap());
                // response with accumulated content
                future::ok(Response::builder().body(Body::from(accumulated)).unwrap())
            });
            Box::new(res)
        }
        // Catch-all 404.
        _ => {
            // for other types of request, only count the number, ignore content
            let _ = record(counter, "OTHERS", "");
            Box::new(future::ok(
                Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::empty())
                    .unwrap(),
            ))
        }
    }
}

// Load public certificate from file.
fn load_certs(filename: &str) -> io::Result<Vec<rustls::Certificate>> {
    // Open certificate file.
    let certfile = fs::File::open(filename)
        .map_err(|e| error(format!("failed to open {}: {}", filename, e)))?;
    let mut reader = io::BufReader::new(certfile);

    // Load and return certificate.
    pemfile::certs(&mut reader).map_err(|_| error("failed to load certificate".into()))
}

// Load private key from file.
fn load_private_key(filename: &str) -> io::Result<rustls::PrivateKey> {
    // Open keyfile.
    let keyfile = fs::File::open(filename)
        .map_err(|e| error(format!("failed to open {}: {}", filename, e)))?;
    let mut reader = io::BufReader::new(keyfile);

    // Load and return a single private key.
    let keys = pemfile::rsa_private_keys(&mut reader)
        .map_err(|_| error("failed to load private key".into()))?;
    if keys.len() != 1 {
        return Err(error("expected a single private key".into()));
    }
    Ok(keys[0].clone())
}
