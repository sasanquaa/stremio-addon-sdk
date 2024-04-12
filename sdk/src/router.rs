use std::fmt::{Debug, Display, Formatter};

use futures::future::OptionFuture;
use hyper::{header, HeaderMap, Method, Request, Response, StatusCode};
use hyper::header::HeaderValue;
use stremio_core::constants::ADDON_MANIFEST_PATH;
use stremio_core::types::addon::{ExtraValue, Manifest, ResourcePath};

use crate::builder::Handler;

use super::server::ServerOptions;

type Result<T> = std::result::Result<T, Error>;

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
    BadRequest,
    NotFound,
    MethodNotAllowed,
    Manifest,
}

#[derive(Clone)]
pub struct Router {
    manifest: Manifest,
    handlers: Vec<Handler>,
    options: ServerOptions,
}

impl Router {
    pub(crate) fn new(manifest: Manifest, handlers: Vec<Handler>, options: ServerOptions) -> Self {
        Self {
            manifest,
            handlers,
            options,
        }
    }

    pub(crate) async fn route<T>(&self, request: Request<T>) -> Result<Response<String>> {
        if request.method() != Method::GET {
            return self.response_from(ResponseKind::MethodNotAllowed);
        }
        return match request.uri().path() {
            "/" => self.response_from(ResponseKind::Html(self.options.landing_html.clone())),
            ADDON_MANIFEST_PATH => self.response_from(ResponseKind::Manifest),
            p => {
                let parts = p.split('/').skip(1).collect::<Vec<&str>>();
                if parts.len() < 3 || parts.len() > 4 {
                    return self.response_from(ResponseKind::BadRequest);
                }
                let path = if parts.len() == 4 {
                    let extras = parts[3]
                        .replace(".json", "")
                        .split('&')
                        .map_while(|part| {
                            let extra = part.split('=').collect::<Vec<&str>>();
                            if extra.len() == 2 {
                                Some(ExtraValue {
                                    name: extra[0].to_string(),
                                    value: extra[1].to_string(),
                                })
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<ExtraValue>>();
                    ResourcePath::with_extra(parts[0], parts[1], parts[2], extras.as_slice())
                } else {
                    ResourcePath::without_extra(
                        parts[0],
                        parts[1],
                        parts[2].replace(".json", "").as_str(),
                    )
                };
                let handler = self
                    .handlers
                    .iter()
                    .find(|&handler| p.starts_with(format!("/{}", handler.name).as_str()));
                let resource = OptionFuture::from(handler.map(|handler| (handler.func)(&path)))
                    .await
                    .unwrap()
                    .map(|path| serde_json::to_string(&path).map_err(Error::Serde));
                if handler.is_none() || resource.is_none() {
                    return self.response_from(ResponseKind::NotFound);
                }
                self.response_from(ResponseKind::Json(resource.unwrap()?))
            }
        };
    }

    pub(crate) fn server_options(&self) -> &ServerOptions {
        &self.options
    }

    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn response_from(&self, kind: ResponseKind) -> Result<Response<String>> {
        let body = match &kind {
            ResponseKind::Json(str) => str.to_string(),
            ResponseKind::Html(str) => str.to_string(),
            ResponseKind::MethodNotAllowed => "Method Not Allowed".to_string(),
            ResponseKind::NotFound => "Not Found".to_string(),
            ResponseKind::BadRequest => "Bad Request".to_string(),
            ResponseKind::Manifest => {
                serde_json::to_string(self.manifest()).map_err(Error::Serde)?
            }
        };
        let code = match &kind {
            ResponseKind::Manifest | ResponseKind::Html(_) | ResponseKind::Json(_) => {
                StatusCode::OK
            }
            ResponseKind::BadRequest => StatusCode::BAD_REQUEST,
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
            _ => (),
        };
        headers_map
    }
}

#[cfg(test)]
mod tests {
    use std::future;
    use std::sync::Arc;

    use hyper::{header, Request, StatusCode};
    use hyper::http::HeaderValue;
    use stremio_core::types::addon::ResourcePath;

    use crate::builder::Handler;
    use crate::router::Router;
    use crate::server::ServerOptions;
    use crate::utils::default_manifest;

    #[tokio::test]
    async fn response_kind_method_not_allowed_when_not_get() {
        let router = Router::new(default_manifest(), vec![], ServerOptions::default());
        let response = router
            .route(Request::builder().method("POST").body(()).unwrap())
            .await;
        assert!(response.is_ok());
        assert!(response.as_ref().unwrap().headers().is_empty());
        assert_eq!(
            response.as_ref().unwrap().status(),
            StatusCode::METHOD_NOT_ALLOWED
        );
    }

    #[tokio::test]
    async fn response_kind_html_when_initial_path() {
        let router = Router::new(default_manifest(), vec![], ServerOptions::default());
        let response = router
            .route(
                Request::builder()
                    .uri("http://127.0.0.1:7070/")
                    .body(())
                    .unwrap(),
            )
            .await;
        assert!(response.is_ok());
        assert_eq!(
            response
                .unwrap()
                .headers()
                .get(header::CONTENT_TYPE)
                .unwrap(),
            HeaderValue::from_static("text/html")
        );
    }

    #[tokio::test]
    async fn response_kind_json_when_manifest_path() {
        let router = Router::new(default_manifest(), vec![], ServerOptions::default());
        let response = router
            .route(
                Request::builder()
                    .uri("http://127.0.0.1:7070/manifest.json")
                    .body(())
                    .unwrap(),
            )
            .await;
        assert!(response.is_ok());
        assert_eq!(
            response
                .as_ref()
                .unwrap()
                .headers()
                .get(header::CONTENT_TYPE)
                .unwrap(),
            HeaderValue::from_static("application/json")
        );
        assert_eq!(
            response.as_ref().unwrap().body(),
            &serde_json::to_string(&default_manifest()).unwrap()
        );
    }

    #[tokio::test]
    async fn response_kind_bad_request_when_invalid_path() {
        let router = Router::new(default_manifest(), vec![], ServerOptions::default());
        let response = router
            .route(
                Request::builder()
                    .uri("http://127.0.0.1:7070/foo/bar")
                    .body(())
                    .unwrap(),
            )
            .await;
        assert!(response.is_ok());
        assert!(response.as_ref().unwrap().headers().is_empty());
        assert_eq!(response.as_ref().unwrap().status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn response_kind_not_found_when_no_handler() {
        let router = Router::new(default_manifest(), vec![], ServerOptions::default());
        let response = router
            .route(
                Request::builder()
                    .uri("http://127.0.0.1:7070/stream/movie/id")
                    .body(())
                    .unwrap(),
            )
            .await;
        assert!(response.is_ok());
        assert!(response.as_ref().unwrap().headers().is_empty());
        assert_eq!(response.as_ref().unwrap().status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn response_kind_not_found_when_no_resource() {
        let handler = Handler {
            name: "stream".into(),
            func: Arc::new(|_: &ResourcePath| Box::pin(future::ready(None))),
        };
        let router = Router::new(default_manifest(), vec![handler], ServerOptions::default());
        let response = router
            .route(
                Request::builder()
                    .uri("http://127.0.0.1:7070/stream/movie/id.json")
                    .body(())
                    .unwrap(),
            )
            .await;
        println!("{:?}", response);
        assert!(response.is_ok());
        assert!(response.as_ref().unwrap().headers().is_empty());
        assert_eq!(response.as_ref().unwrap().status(), StatusCode::NOT_FOUND);
    }
}
