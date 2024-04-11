pub mod builder;
pub mod router;
pub mod server;
pub mod utils;

#[cfg(test)]
mod tests {
    use futures::future;
    use stremio_core::types::addon::{Manifest, ManifestResource, ResourceResponse};

    use crate::builder::HandlerKind;
    use crate::server::ServerOptions;

    use super::*;

    #[test]
    #[should_panic]
    fn builder_panics_if_no_handlers_attached() {
        builder::Builder::new(utils::default_manifest()).build(ServerOptions::default());
    }

    #[test]
    #[should_panic]
    fn builder_panics_if_no_resources_defined_for_handler() {
        builder::Builder::new(utils::default_manifest())
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
        builder::Builder::new(manifest)
            .handler(HandlerKind::Stream, |_| {
                Box::pin(future::ready(Some(ResourceResponse::Streams {
                    streams: vec![],
                })))
            })
            .build(ServerOptions::default());
    }
}
