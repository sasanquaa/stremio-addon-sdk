use std::error::Error;

use hyper::{HeaderMap, StatusCode};

pub(crate) type HyperResponse<T> = hyper::Response<T>;
pub(crate) type ServerlessResponse<T> = vercel_runtime::Response<T>;

pub(crate) enum Response<T> {
    Hyper(HyperResponse<T>),
    Serverless(ServerlessResponse<T>),
}

impl<T> Response<T> {
    pub(crate) fn builder() -> ResponseBuilder<T> {
        ResponseBuilder {
            code: StatusCode::OK,
            headers: HeaderMap::new(),
            body: None,
        }
    }
}

pub(crate) struct ResponseBuilder<T> {
    code: StatusCode,
    headers: HeaderMap,
    body: Option<T>,
}

impl<T> ResponseBuilder<T> {
    pub(crate) fn status(self, code: StatusCode) -> ResponseBuilder<T> {
        Self { code, ..self }
    }

    pub(crate) fn headers(self, headers: HeaderMap) -> ResponseBuilder<T> {
        Self { headers, ..self }
    }

    pub(crate) fn body(self, body: T) -> ResponseBuilder<T> {
        Self {
            body: Some(body),
            ..self
        }
    }

    pub(crate) fn build(
        self,
        is_serverless: bool,
    ) -> Result<Response<T>, Box<dyn Error + Send + Sync + 'static>>
    where
        T: Default,
    {
        if is_serverless {
            self.build_serverless()
        } else {
            self.build_hyper()
        }
    }

    fn build_hyper(self) -> Result<Response<T>, Box<dyn Error + Send + Sync + 'static>>
    where
        T: Default,
    {
        let mut builder = HyperResponse::builder().status(self.code);
        for (name, value) in &self.headers {
            builder = builder.header(name.as_str(), value.as_bytes());
        }
        builder
            .body(self.body.unwrap_or_default())
            .map(Response::Hyper)
            .map_err(|err| err.into())
    }

    fn build_serverless(self) -> Result<Response<T>, Box<dyn Error + Send + Sync + 'static>>
    where
        T: Default,
    {
        let mut builder = ServerlessResponse::builder().status(self.code.as_str());
        for (name, value) in &self.headers {
            builder = builder.header(name.as_str(), value.as_bytes());
        }
        builder
            .body(self.body.unwrap_or_default())
            .map(Response::Serverless)
            .map_err(|err| err.into())
    }
}
