use std::fmt::Display;
use std::sync::Arc;

use futures::future::BoxFuture;
use stremio_core::constants::{
    CATALOG_RESOURCE_NAME, META_RESOURCE_NAME, STREAM_RESOURCE_NAME, SUBTITLES_RESOURCE_NAME,
};
use stremio_core::types::addon::{Manifest, ManifestResource, ResourcePath, ResourceResponse};

use crate::router::Router;
use crate::server::ServerOptions;

type HandlerFn =
    dyn Fn(&ResourcePath) -> BoxFuture<Option<ResourceResponse>> + Send + Sync + 'static;

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

impl Display for HandlerKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            HandlerKind::Meta => META_RESOURCE_NAME,
            HandlerKind::Subtitles => SUBTITLES_RESOURCE_NAME,
            HandlerKind::Stream => STREAM_RESOURCE_NAME,
            HandlerKind::Catalog => CATALOG_RESOURCE_NAME,
        }
        .to_string();
        write!(f, "{}", str)
    }
}

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
        F: Fn(&ResourcePath) -> BoxFuture<Option<ResourceResponse>> + Send + Sync + 'static,
    {
        if self.handlers.iter().any(|h| h.name == kind.to_string()) {
            panic!("handler for resource '{}' is already defined!", kind);
        }
        self.handlers.push(Handler {
            name: kind.to_string(),
            func: Arc::new(handler),
        });
        self
    }

    pub fn build(self, options: ServerOptions) -> Router {
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
            handler_names.push(HandlerKind::Catalog.to_string());
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
                if handler.name == HandlerKind::Catalog.to_string() {
                    errors.push(
                        "manifest.catalogs is empty, 'catalog' handler will never be called"
                            .to_string(),
                    );
                } else {
                    errors.push(format!(
                        "manifest.resources does not contain: {}",
                        handler.name
                    ));
                }
            }
        }
        // check if handlers that are specified in the manifest are also defined
        for name in handler_names {
            if !self.handlers.iter().any(|handler| name == handler.name) {
                errors.push(format!(
                    "manifest definition requires handler for '{}', but it is not provided",
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

#[cfg(test)]
mod tests {
    use futures::future;
    use stremio_core::types::addon::{Manifest, ManifestResource, ResourceResponse};

    use crate::builder::{Builder, HandlerKind};
    use crate::server::ServerOptions;
    use crate::utils;

    #[test]
    #[should_panic]
    fn builder_panics_if_no_handlers_attached() {
        Builder::new(utils::default_manifest()).build(ServerOptions::default());
    }

    #[test]
    #[should_panic]
    fn builder_panics_if_no_resources_defined_for_handler() {
        Builder::new(utils::default_manifest())
            .handler(HandlerKind::Stream, |_| {
                Box::pin(future::ready(Some(ResourceResponse::Streams {
                    streams: vec![],
                })))
            })
            .build(ServerOptions::default());
    }

    #[test]
    #[should_panic]
    fn builder_panics_if_no_handlers_defined_for_resource() {
        let manifest = Manifest {
            resources: vec![
                ManifestResource::Short("meta".into()),
                ManifestResource::Short("stream".into()),
            ],
            ..utils::default_manifest()
        };
        Builder::new(manifest)
            .handler(HandlerKind::Stream, |_| {
                Box::pin(future::ready(Some(ResourceResponse::Streams {
                    streams: vec![],
                })))
            })
            .build(ServerOptions::default());
    }

    #[test]
    #[should_panic]
    fn builder_panics_if_handler_is_redefined() {
        Builder::new(utils::default_manifest())
            .handler(HandlerKind::Subtitles, |_| {
                Box::pin(future::ready(Some(ResourceResponse::Subtitles {
                    subtitles: vec![],
                })))
            })
            .handler(HandlerKind::Subtitles, |_| {
                Box::pin(future::ready(Some(ResourceResponse::Subtitles {
                    subtitles: vec![],
                })))
            })
            .build(ServerOptions::default());
    }
}
