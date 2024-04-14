use std::future;

use futures::future::BoxFuture;
use stremio_core::types::addon::{Manifest, ManifestResource, ResourceResponse, Version};
use stremio_core::types::resource::{Stream, StreamSource};
use url::Url;
use vercel_runtime::{Body, Error, Request, Response, run};

use stremio_addon_sdk::builder::{Builder, HandlerKind};
use stremio_addon_sdk::server::{serve_serverless, ServerOptions};

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(handler_serverless).await
}

pub async fn handler_serverless(
    req: Request,
) -> Result<Response<Body>, Box<dyn std::error::Error + Send + Sync + 'static>> {
    let manifest = Manifest {
        id: "org.example.addon".into(),
        version: Version::new(1, 0, 0),
        name: "Example".into(),
        contact_email: None,
        resources: vec![ManifestResource::Short("stream".into())],
        types: vec!["movie".into()],
        catalogs: vec![],
        addon_catalogs: vec![],
        background: Some(Url::parse("https://i.imgur.com/P3JQEmD.jpg").unwrap()),
        logo: Some(Url::parse("https://i.imgur.com/M6pQlDh.jpg").unwrap()),
        description: Some("Example Addon".into()),
        id_prefixes: None,
        behavior_hints: Default::default(),
    };
    let options = ServerOptions::default();
    let router = Builder::new(manifest).handler(HandlerKind::Stream, |req| -> BoxFuture<Option<ResourceResponse>>{
        println!("Stream: {}/{}/{}/{:?}", req.resource, req.r#type, req.id, req.extra);
        if req.r#type == "movie" && req.id == "tt1254207" {
            Box::pin(future::ready(Some(ResourceResponse::Streams {
                streams: vec![Stream {
                    source: StreamSource::Url {
                        url: Url::parse("http://distribution.bbb3d.renderfarming.net/video/mp4/bbb_sunflower_1080p_30fps_normal.mp4").unwrap()
                    },
                    name: None,
                    description: None,
                    thumbnail: None,
                    subtitles: vec![],
                    behavior_hints: Default::default(),
                }],
            })))
        } else {
            Box::pin(future::ready(Some(ResourceResponse::Streams {
                streams: vec![],
            })))
        }
    }).build(options);
    serve_serverless(router, req).await
}