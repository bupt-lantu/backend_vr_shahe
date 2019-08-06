#![feature(async_await)]

// #![deny(warnings)]  // FIXME: https://github.com/rust-lang/rust/issues/62411
#[macro_use]
extern crate lazy_static;

use futures_util::TryStreamExt;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::sync::RwLock;

static FILENAME: &'static str = "./map.json";

lazy_static! {
    static ref MAP: RwLock<HashMap<String, String>> = RwLock::new(HashMap::new());
}

static KEYVALUETYPE: &[u8] = b"key or value should be str";
static BODYTYPE: &[u8] = b"body should be object";
static GETERROR: &[u8] = b"object should has key or value";
// Using service_fn, we can turn this function into a `Service`.
async fn param_example(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/") => {
            let body = serde_json::to_string(&MAP.read().unwrap().clone())
                .ok()
                .unwrap();
            Ok(Response::new(body.into()))
        }
        (&Method::POST, "/add") => {
            let b = req.into_body().try_concat().await?;
            let object = match serde_json::from_slice::<Value>(&b.into_bytes()[..]) {
                Ok(ref v) if v.is_object() => v.as_object().unwrap().clone(),
                _ => {
                    return Ok(Response::builder()
                        .status(StatusCode::UNPROCESSABLE_ENTITY)
                        .body(BODYTYPE.into())
                        .unwrap())
                }
            };
            if let (Some(key), Some(value)) = (object.get("key"), object.get("value")) {
                match (key.as_str(), value.as_str()) {
                    (Some(key), Some(value)) => MAP
                        .write()
                        .unwrap()
                        .insert(key.to_string(), value.to_string()),
                    _ => {
                        return Ok(Response::builder()
                            .status(StatusCode::UNPROCESSABLE_ENTITY)
                            .body(KEYVALUETYPE.into())
                            .unwrap())
                    }
                };
            } else {
                return Ok(Response::builder()
                    .status(StatusCode::UNPROCESSABLE_ENTITY)
                    .body(GETERROR.into())
                    .unwrap());
            }
            let body = serde_json::to_string(&MAP.read().unwrap().clone())
                .ok()
                .unwrap();
            write(&body);
            Ok(Response::new("ok".into()))
        }
        (&Method::POST, "/del") => {
            let b = req.into_body().try_concat().await?;
            let result = MAP
                .write()
                .unwrap()
                .remove(&String::from_utf8(b.into_bytes().to_vec()).ok().unwrap());

            match result {
                Some(_) => {
                    let body = serde_json::to_string(&MAP.read().unwrap().clone())
                        .ok()
                        .unwrap();
                    write(&body);
                }
                _ => {}
            };
            Ok(Response::new("ok".into()))
        }
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::empty())
            .unwrap()),
    }
}

fn write(json: &str) {
    let file = OpenOptions::new().create(true).write(true).truncate(true).open(FILENAME);
    match file {
        Ok(mut stream) => {
            let _ = stream.write_all(json.as_bytes());
        }
        Err(err) => println!("{}",err),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    {
        let file = OpenOptions::new().read(true).open(FILENAME);
        match file {
            Ok(mut stream) => {
                let mut json_string = String::new();
                let _ = stream.read_to_string(&mut json_string);
                let hashmap: HashMap<String, String> = match serde_json::from_str(&json_string) {
                    Ok(v) => v,
                    _ => HashMap::new(),
                };
                *MAP.write().unwrap() = hashmap;
            }
            _ => {}
        }
    }

    let addr = ([0,0,0,0], 1337).into();
    let server = Server::bind(&addr).serve(make_service_fn(|_| {
        async { Ok::<_, hyper::Error>(service_fn(param_example)) }
    }));
    println!("Listening on http://{}", addr);
    server.await?;
    Ok(())
}
