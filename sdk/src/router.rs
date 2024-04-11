use std::fmt::{Debug, Display, Formatter};
use std::future;
use std::future::Future;

use futures::FutureExt;
use hyper::{header, HeaderMap, Method, Request, Response, StatusCode};
use hyper::header::HeaderValue;
use stremio_core::constants::ADDON_MANIFEST_PATH;
use stremio_core::types::addon::{Manifest, ResourcePath};

use crate::builder::Handler;
use crate::landing::landing_template;

use super::server::ServerOptions;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Http(hyper::http::Error),
    Serde(serde_json::Error),
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Http(err) => Display::fmt(err, f),
            Error::Serde(err) => Display::fmt(err, f),
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

    pub fn route<T>(
        &self,
        request: Request<T>,
    ) -> Box<dyn Future<Output = Result<Response<String>>> + Send + Unpin + '_> {
        if request.method() != Method::GET {
            return Box::new(future::ready(
                self.response_from(ResponseKind::MethodNotAllowed),
            ));
        }
        return match request.uri().path() {
            "/" => Box::new(future::ready(
                self.response_from(ResponseKind::Html(landing_template(self.manifest()))),
            )),
            ADDON_MANIFEST_PATH => {
                Box::new(future::ready(self.response_from(ResponseKind::Manifest)))
            }
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
                    .find(|&handler| p.starts_with(format!("/{}", handler.name).as_str()))
                    .ok_or_else(|| self.response_from(ResponseKind::NotFound).unwrap_err());
                if let Ok(handler) = handler {
                    self.resource_from_handler(&path, handler)
                } else {
                    Box::new(future::ready(Err(handler.err().unwrap())))
                }
            }
        };
    }

    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn resource_from_handler(
        &self,
        path: &ResourcePath,
        handler: &Handler,
    ) -> Box<dyn Future<Output = Result<Response<String>>> + Send + Unpin + '_> {
        Box::new((handler.func)(path).map(|option| {
            if let Some(resource) = option {
                serde_json::to_string(&resource)
                    .map(|str| self.response_from(ResponseKind::Json(str)).unwrap())
                    .map_err(Error::Serde)
            } else {
                self.response_from(ResponseKind::NotFound)
            }
        }))
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
