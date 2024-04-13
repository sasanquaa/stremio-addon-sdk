use std::fmt::{Debug, Display, Formatter};

use hyper::{header, HeaderMap, Method, StatusCode};
use hyper::header::HeaderValue;
use stremio_core::constants::ADDON_MANIFEST_PATH;
use stremio_core::types::addon::{ExtraValue, Manifest, ResourcePath};

use crate::builder::Handler;
use crate::request::Request;
use crate::response::Response;

use super::server::ServerOptions;

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Http(Box<dyn std::error::Error + Send + Sync + 'static>),
    Serde(serde_json::Error),
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Http(err) => Some(err.as_ref()),
            Error::Serde(err) => Some(err),
        }
    }
}

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

    pub(crate) async fn route<T, E>(&self, request: Request<E>) -> Result<Response<T>>
        where
            T: From<String> + Default,
    {
        let is_serverless = match request {
            Request::Serverless(_) => true,
            Request::Hyper(_) => false,
        };
        if request.method() != Method::GET {
            return self.response_from(is_serverless, ResponseKind::MethodNotAllowed);
        }
        return match request.uri().path() {
            "/" => self.response_from(
                is_serverless,
                ResponseKind::Html(self.options.landing_html.clone()),
            ),
            ADDON_MANIFEST_PATH => self.response_from(is_serverless, ResponseKind::Manifest),
            p => {
                let parts = p.split('/').skip(1).collect::<Vec<&str>>();
                if parts.len() < 3 || parts.len() > 4 {
                    return self.response_from(is_serverless, ResponseKind::BadRequest);
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
                if handler.is_none() {
                    return self.response_from(is_serverless, ResponseKind::NotFound);
                }
                let resource = (handler.unwrap().func)(&path)
                    .await
                    .map(|path| serde_json::to_string(&path).map_err(Error::Serde));
                if resource.is_none() {
                    return self.response_from(is_serverless, ResponseKind::NotFound);
                }
                self.response_from(is_serverless, ResponseKind::Json(resource.unwrap()?))
            }
        };
    }

    pub(crate) fn server_options(&self) -> &ServerOptions {
        &self.options
    }

    fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn response_from<T>(&self, is_serverless: bool, kind: ResponseKind) -> Result<Response<T>>
        where
            T: From<String> + Default,
    {
        let headers = self.header_map_from(&kind);
        let code = match &kind {
            ResponseKind::Manifest | ResponseKind::Html(_) | ResponseKind::Json(_) => {
                StatusCode::OK
            }
            ResponseKind::BadRequest => StatusCode::BAD_REQUEST,
            ResponseKind::NotFound => StatusCode::NOT_FOUND,
            ResponseKind::MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED,
        };
        let manifest = if matches!(kind, ResponseKind::Manifest) {
            Some(serde_json::to_string(self.manifest()).map_err(Error::Serde)?)
        } else {
            None
        };
        let body = match kind {
            ResponseKind::Json(str) => T::from(str),
            ResponseKind::Html(str) => T::from(str),
            ResponseKind::MethodNotAllowed => T::from("Method Not Allowed".into()),
            ResponseKind::NotFound => T::from("Not Found".into()),
            ResponseKind::BadRequest => T::from("Bad Request".into()),
            ResponseKind::Manifest => T::from(manifest.unwrap()),
        };
        Response::builder()
            .status(code)
            .headers(headers)
            .body(body)
            .build(is_serverless)
            .map_err(Error::Http)
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
    use crate::request;
    use crate::response::Response;
    use crate::router::Router;
    use crate::server::ServerOptions;
    use crate::utils::default_manifest;

    #[tokio::test]
    async fn response_kind_method_not_allowed_when_not_get() {
        let router = Router::new(default_manifest(), vec![], ServerOptions::default());
        let response = router
            .route::<String, ()>(request::Request::Hyper(
                Request::builder().method("POST").body(()).unwrap(),
            ))
            .await;
        assert!(response.is_ok());
        let response = match response.unwrap() {
            Response::Hyper(res) => res,
            Response::Serverless(_) => unreachable!(),
        };
        assert!(response.headers().is_empty());
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn response_kind_html_when_initial_path() {
        let router = Router::new(default_manifest(), vec![], ServerOptions::default());
        let response = router
            .route::<String, ()>(request::Request::Hyper(
                Request::builder()
                    .uri("http://127.0.0.1:7070/")
                    .body(())
                    .unwrap(),
            ))
            .await;
        assert!(response.is_ok());
        let response = match response.unwrap() {
            Response::Hyper(res) => res,
            Response::Serverless(_) => unreachable!(),
        };
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("text/html")
        );
    }

    #[tokio::test]
    async fn response_kind_json_when_manifest_path() {
        let router = Router::new(default_manifest(), vec![], ServerOptions::default());
        let response = router
            .route::<String, ()>(request::Request::Hyper(
                Request::builder()
                    .uri("http://127.0.0.1:7070/manifest.json")
                    .body(())
                    .unwrap(),
            ))
            .await;
        assert!(response.is_ok());
        let response = match response.unwrap() {
            Response::Hyper(res) => res,
            Response::Serverless(_) => unreachable!(),
        };
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            HeaderValue::from_static("application/json")
        );
        assert_eq!(
            response.body(),
            &serde_json::to_string(&default_manifest()).unwrap()
        );
    }

    #[tokio::test]
    async fn response_kind_bad_request_when_invalid_path() {
        let router = Router::new(default_manifest(), vec![], ServerOptions::default());
        let response = router
            .route::<String, ()>(request::Request::Hyper(
                Request::builder()
                    .uri("http://127.0.0.1:7070/foo/bar")
                    .body(())
                    .unwrap(),
            ))
            .await;
        assert!(response.is_ok());
        let response = match response.unwrap() {
            Response::Hyper(res) => res,
            Response::Serverless(_) => unreachable!(),
        };
        assert!(response.headers().is_empty());
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn response_kind_not_found_when_no_handler() {
        let router = Router::new(default_manifest(), vec![], ServerOptions::default());
        let response = router
            .route::<String, ()>(request::Request::Hyper(
                Request::builder()
                    .uri("http://127.0.0.1:7070/stream/movie/id")
                    .body(())
                    .unwrap(),
            ))
            .await;
        assert!(response.is_ok());
        let response = match response.unwrap() {
            Response::Hyper(res) => res,
            Response::Serverless(_) => unreachable!(),
        };
        assert!(response.headers().is_empty());
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn response_kind_not_found_when_no_resource() {
        let handler = Handler {
            name: "stream".into(),
            func: Arc::new(|_: &ResourcePath| Box::pin(future::ready(None))),
        };
        let router = Router::new(default_manifest(), vec![handler], ServerOptions::default());
        let response = router
            .route::<String, ()>(request::Request::Hyper(
                Request::builder()
                    .uri("http://127.0.0.1:7070/stream/movie/id.json")
                    .body(())
                    .unwrap(),
            ))
            .await;
        assert!(response.is_ok());
        let response = match response.unwrap() {
            Response::Hyper(res) => res,
            Response::Serverless(_) => unreachable!(),
        };
        assert!(response.headers().is_empty());
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
