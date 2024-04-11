use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use stremio_core::constants::{
    CATALOG_RESOURCE_NAME, META_RESOURCE_NAME, STREAM_RESOURCE_NAME, SUBTITLES_RESOURCE_NAME,
};
use stremio_core::types::addon::{Manifest, ManifestResource, ResourcePath, ResourceResponse};

use crate::router::Router;
use crate::server::ServerOptions;

type HandlerFuture = dyn Future<Output = Option<ResourceResponse>> + Send;
type HandlerFn = dyn Fn(&ResourcePath) -> Pin<Box<HandlerFuture>> + Send + Sync + 'static;

#[derive(Clone)]
pub struct Handler {
    pub(crate) name: String,
    pub(crate) func: Arc<HandlerFn>,
}

pub enum HandlerKind {
    Meta,
    Subtitles,
    Stream,
    Catalog,
}

impl HandlerKind {
    fn as_str(&self) -> &str {
        match self {
            HandlerKind::Meta => META_RESOURCE_NAME,
            HandlerKind::Subtitles => SUBTITLES_RESOURCE_NAME,
            HandlerKind::Stream => STREAM_RESOURCE_NAME,
            HandlerKind::Catalog => CATALOG_RESOURCE_NAME,
        }
    }
}

#[derive(Clone)]
pub struct Builder {
    manifest: Manifest,
    handlers: Vec<Handler>,
}

impl Builder {
    pub fn new(manifest: Manifest) -> Self {
        Self {
            manifest,
            handlers: vec![],
        }
    }

    pub fn handler<F>(mut self, kind: HandlerKind, handler: F) -> Self
    where
        F: Fn(&ResourcePath) -> Pin<Box<HandlerFuture>> + Send + Sync + 'static,
    {
        if self.handlers.iter().any(|h| h.name == kind.as_str()) {
            panic!("handler for resource {} is already defined!", kind.as_str());
        }
        self.handlers.push(Handler {
            name: kind.as_str().to_string(),
            func: Arc::new(handler),
        });
        self
    }

    pub(crate) fn build(self, options: ServerOptions) -> Router {
        self.validate();
        Router::new(self.manifest, self.handlers, options)
    }

    fn validate(&self) {
        let mut errors = Vec::new();
        let mut handler_names = Vec::new();
        let manifest = &self.manifest;

        if self.handlers.is_empty() {
            errors.push("at least one handler must be defined".into());
        }
        // get all handlers that are declared in the manifest
        if !manifest.catalogs.is_empty() {
            handler_names.push(HandlerKind::Catalog.as_str().into());
        }
        for resource in &manifest.resources {
            // NOTE: resource.name() should probably be public in stremio-core, making this code unnecessary
            match resource {
                ManifestResource::Short(name) => handler_names.push(name.to_string()),
                ManifestResource::Full { name, .. } => handler_names.push(name.to_string()),
            }
        }
        // check if defined handlers are also specified in the manifest
        for handler in &self.handlers {
            if !handler_names.iter().any(|name| *name == handler.name) {
                errors.push(format!(
                    "manifest.resources does not contain: {}",
                    handler.name
                ));
            }
        }
        // check if handlers that are specified in the manifest are also defined
        for name in handler_names {
            if !self.handlers.iter().any(|handler| name == handler.name) {
                errors.push(format!(
                    "manifest definition requires handler for {}, but it is not provided",
                    name
                ));
            }
        }
        if !errors.is_empty() {
            let error = errors.join("\n");
            let error_formatted = format!("\n--failed to build addon interface-- \n{}", error);
            panic!("{}", error_formatted);
        }
    }
}
