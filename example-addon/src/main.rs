use std::{future, io};

use stremio_core::types::addon::{
    Manifest, ManifestCatalog, ManifestExtra, ManifestResource, ResourceResponse, Version,
};

use stremio_addon_sdk::builder::{Builder, HandlerKind};
use stremio_addon_sdk::server::{serve_http, ServerOptions};
use stremio_addon_sdk::utils;

#[tokio::main]
async fn main() -> io::Result<()> {
    let manifest = Manifest {
        id: "org.example.addon".to_string(),
        version: Version::new(1, 0, 0),
        name: "Example".to_string(),
        resources: vec![
            ManifestResource::Short("catalog".into()),
            ManifestResource::Short("stream".into()),
        ],
        types: vec!["movie".into()],
        catalogs: vec![ManifestCatalog {
            id: "bbbcatalog".into(),
            r#type: "others".into(),
            name: Some("Example catalog".into()),
            extra: ManifestExtra::default(),
        }],
        id_prefixes: Some(vec!["tt".into()]),
        description: Some("Example Rust Addon".into()),
        ..utils::default_manifest()
    };
    serve_http(
        Builder::new(manifest)
            .handler(HandlerKind::Stream, |req| {
                println!("Stream: {}/{}/{}", req.resource, req.r#type, req.id);
                Box::pin(future::ready(Some(ResourceResponse::Streams {
                    streams: vec![],
                })))
            })
            .handler(HandlerKind::Catalog, |req| {
                println!("Catalog: {}/{}/{}", req.resource, req.r#type, req.id);
                Box::pin(future::ready(Some(ResourceResponse::Metas {
                    metas: vec![],
                })))
            }),
        ServerOptions::default(),
    )
    .await
}
