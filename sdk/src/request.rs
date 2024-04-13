use std::str::FromStr;

use hyper::{Method, Uri};

pub(crate) type HyperRequest<T> = hyper::Request<T>;
pub(crate) type ServerlessRequest = vercel_runtime::Request;

pub(crate) enum Request<T> {
    Hyper(HyperRequest<T>),
    Serverless(ServerlessRequest),
}

impl<T> Request<T> {
    pub(crate) fn method(&self) -> Method {
        match self {
            Request::Hyper(req) => req.method().clone(),
            Request::Serverless(req) => Method::from_str(req.method().as_str()).unwrap(),
        }
    }

    pub(crate) fn uri(&self) -> Uri {
        match self {
            Request::Hyper(req) => req.uri().clone(),
            Request::Serverless(req) => req.uri().to_string().parse::<Uri>().unwrap(),
        }
    }
}
