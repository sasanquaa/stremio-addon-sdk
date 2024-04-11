use std::fmt::{Debug, Display, Formatter};

use hyper::{header, HeaderMap, Method, Request, Response, StatusCode};
use hyper::header::HeaderValue;
use stremio_core::constants::ADDON_MANIFEST_PATH;
use stremio_core::types::addon::{Manifest, ResourcePath};

use crate::builder::Handler;

use super::server::ServerOptions;

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Http(hyper::http::Error),
    Serde(serde_json::Error),
    ResourceNotFound,
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Http(err) => Display::fmt(err, f),
            Error::Serde(err) => Display::fmt(err, f),
            Error::ResourceNotFound => f.write_str("ResourceNotFound"),
        }
    }
}

enum ResponseKind {
    Json(String),
    Html(String),
    NotFound,
    MethodNotAllowed,
    Manifest,
}

pub struct Router {
    manifest: Manifest,
    handlers: Vec<Handler>,
    options: ServerOptions,
}

impl Router {
    pub fn new(manifest: Manifest, handlers: Vec<Handler>, options: ServerOptions) -> Self {
        Self {
            manifest,
            handlers,
            options,
        }
    }

    pub async fn route<T>(&self, request: Request<T>) -> Result<Response<String>> {
        if request.method() != Method::GET {
            return self.response_from(ResponseKind::MethodNotAllowed);
        }
        return match request.uri().path() {
            "/" => self.response_from(ResponseKind::Html("<html></html>".to_string())),
            ADDON_MANIFEST_PATH => self.response_from(ResponseKind::Manifest),
            p => {
                let parts = p.split('/').collect::<Vec<&str>>();
                let path = if parts.len() == 4 {
                    ResourcePath::with_extra(parts[0], parts[1], parts[2], &[])
                } else {
                    ResourcePath::without_extra(parts[0], parts[1], parts[2])
                };
                let handler = self
                    .handlers
                    .iter()
                    .find(|&handler| p.starts_with(format!("/{}", handler.name).as_str()));
                if handler.is_none() {
                    return self.response_from(ResponseKind::NotFound);
                }
                let str = (handler.unwrap().func)(&path)
                    .await
                    .map_or(Err(Error::ResourceNotFound), |path| {
                        serde_json::to_string(&path).map_err(Error::Serde)
                    })?;
                self.response_from(ResponseKind::Json(str))
            }
        };
    }

    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn response_from(&self, kind: ResponseKind) -> Result<Response<String>> {
        let body = match &kind {
            ResponseKind::Json(str) => str.to_string(),
            ResponseKind::Html(str) => str.to_string(),
            ResponseKind::NotFound => "Not Found".to_string(),
            ResponseKind::MethodNotAllowed => "Method Not Allowed".to_string(),
            ResponseKind::Manifest => {
                serde_json::to_string(self.manifest()).map_err(Error::Serde)?
            }
        };
        let code = match &kind {
            ResponseKind::Manifest | ResponseKind::Html(_) | ResponseKind::Json(_) => {
                StatusCode::OK
            }
            ResponseKind::NotFound => StatusCode::NOT_FOUND,
            ResponseKind::MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED,
        };
        let mut builder = Response::builder().status(code);
        builder
            .headers_mut()
            .unwrap()
            .extend(self.header_map_from(&kind));
        builder.body(body).map_err(Error::Http)
    }

    fn header_map_from(&self, kind: &ResponseKind) -> HeaderMap {
        let mut headers_map = HeaderMap::new();
        match kind {
            ResponseKind::Manifest | ResponseKind::Json(_) => {
                headers_map.append(
                    header::ACCESS_CONTROL_ALLOW_ORIGIN,
                    HeaderValue::from_static("*"),
                );
                headers_map.append(
                    header::CACHE_CONTROL,
                    HeaderValue::from_str(
                        format!("max-age={}, public", self.options.cache_max_age).as_str(),
                    )
                    .unwrap(),
                );
                headers_map.append(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json"),
                );
            }
            ResponseKind::Html(_) => {
                headers_map.append(header::CONTENT_TYPE, HeaderValue::from_static("text/html"));
            }
            ResponseKind::MethodNotAllowed | ResponseKind::NotFound => (),
        };
        headers_map
    }
}
