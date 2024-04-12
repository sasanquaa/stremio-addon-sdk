<h1 align="center">
  <img width="150" src="https://i.imgur.com/QaYvRVJ.png" />
  <p>Stremio Addon SDK</p>
</h1>

<h4 align="center">Rust version of the <a href="https://github.com/Stremio/stremio-addon-sdk" target="_blak">stremio-addon-sdk</a> using <a href="https://github.com/Stremio/stremio-core" target="_blank">stremio-core</a></h4>

## Getting started
```rust
use stremio_addon_sdk::builder::Builder;
use stremio_addon_sdk::server::{serve_http, ServerOptions};

#[tokio::main]
async fn main() {
    // create manifest file using stremio-core's Manifest struct
    let manifest = Manifest {
        // ...
    };

    // build addon interface
    let interface = Builder::new(manifest)
        // function as parameter
        .define_catalog_handler(handle_catalog)
        .define_stream_handler(handle_stream)
        // closure as parameter
        .define_meta_handler(|resource: &ResourceRef| -> EnvFuture<ResourceResponse> {
            let response = ResourceResponse::Metas { metas: vec![] };
            return Box::new(future::ok(response));
        })
        .build();

    // run HTTP server with default settings
    serve_http(interface, ServerOptions::default());
}
```

See the [example-addon](example-addon) for more details.

## Documentation
TODO