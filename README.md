<h1 align="center">
  <img width="150" src="https://i.imgur.com/QaYvRVJ.png" />
  <p>Stremio Addon SDK</p>
</h1>

<h4 align="center">Rust version of the <a href="https://github.com/Stremio/stremio-addon-sdk" target="_blak">stremio-addon-sdk</a> using <a href="https://github.com/Stremio/stremio-core" target="_blank">stremio-core</a></h4>

## Getting started
```rust
use std::future;
use stremio_addon_sdk::builder::{Builder, HandlerKind};
use stremio_addon_sdk::server::{serve_http, ServerOptions};
use futures::future::BoxFuture;

#[tokio::main]
async fn main() {
    // create manifest file using stremio-core's Manifest struct
    let manifest = Manifest {
        // ...
    };
    let options = ServerOptions {
        // ...
    };

    // build router
    let router = Builder::new(manifest)
        // function as parameter
        .handler(HandlerKind::Catalog, handle_catalog)
        .handler(HandlerKind::Stream, handle_stream)
        // closure as parameter
        .handler(HandlerKind::Meta, |path: &ResourcePath| -> BoxFuture<Option<ResourceResponse>> {
            let response = ResourceResponse::Metas { metas: vec![] };
            return Box::pin(future::ready(response));
        })
        .build(options);

    // run HTTP server with default settings
    serve_http(router);
}
```

See the [example-addon](example-addon) for more details.

## Documentation
TODO