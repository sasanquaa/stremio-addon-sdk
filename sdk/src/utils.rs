use stremio_core::types::addon::Manifest;
use vercel_runtime::{Body, Request, Response};

use crate::{SdkRequest, SdkResponse};

pub fn default_manifest() -> Manifest {
    Manifest {
        id: "".to_string(),
        version: semver::Version {
            major: 0,
            minor: 1,
            patch: 0,
            pre: Default::default(),
            build: Default::default(),
        },
        name: "".to_string(),
        contact_email: None,
        description: None,
        logo: None,
        background: None,
        types: vec![],
        resources: vec![],
        id_prefixes: None,
        catalogs: vec![],
        addon_catalogs: vec![],
        behavior_hints: Default::default(),
    }
}

pub fn serverless_request_to_sdk_request(req: Request) -> SdkRequest<Body> {
    let uri = req.uri().to_string();
    let method = req.method().to_string();
    let headers = req.headers().clone();
    let body = req.into_body();

    let mut builder = SdkRequest::builder().uri(uri).method(method.as_str());
    for (name, value) in &headers {
        builder = builder.header(name.as_str(), value.to_str().unwrap());
    }
    builder.body(body).unwrap()
}

pub fn sdk_response_to_serverless_response(res: SdkResponse<Body>) -> Response<Body> {
    let code = res.status();
    let headers = res.headers().clone();
    let body = res.into_body();

    let mut builder = Response::builder().status(code.as_str());
    for (name, value) in &headers {
        builder = builder.header(name.as_str(), value.to_str().unwrap());
    }
    builder.body(body).unwrap()
}
