use std::{future, io};

use stremio_core::types::addon::{Manifest, ManifestResource, ResourceResponse, Version};
use stremio_core::types::resource::{Stream, StreamSource};
use url::Url;

use stremio_addon_sdk::builder::{Builder, HandlerKind};
use stremio_addon_sdk::server::{serve_http, ServerOptions};

#[tokio::main]
async fn main() -> io::Result<()> {
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
    let router = Builder::new(manifest).handler(HandlerKind::Stream, |req| {
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
        }).build(ServerOptions::default());
    serve_http(router).await
}
