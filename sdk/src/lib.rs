pub mod builder;
pub mod helpers;
pub mod landing;
pub mod router;
pub mod server;

// #[cfg(test)]
// mod tests {
//     use stremio_core::types::addon::Manifest;
//     use stremio_core::types::addons::*;
//
//     use futures::future;
//     use crate::server::ServerOptions;
//
//     use super::*;
//
//     #[test]
//     #[should_panic]
//     fn builder_panics_if_no_handlers_attached() {
//         builder::Builder::new(Manifest::default()).build(ServerOptions::default());
//     }
//
//     #[test]
//     #[should_panic]
//     fn builder_panics_if_no_resources_defined_for_handler() {
//         builder::Builder::new(scaffold::Scaffold::default_manifest())
//             .define_stream_handler(|_| {
//                 Box::new(future::ok(ResourceResponse::Streams { streams: vec![] }))
//             })
//             .build();
//     }
//
//     #[test]
//     #[should_panic]
//     fn builder_panics_if_no_handlers_defined_for_resource() {
//         let manifest = Manifest {
//             resources: vec![
//                 ManifestResource::Short("meta".into()),
//                 ManifestResource::Short("stream".into()),
//             ],
//             ..scaffold::Scaffold::default_manifest()
//         };
//         builder::Builder::new(manifest)
//             .define_stream_handler(|_| {
//                 Box::new(future::ok(ResourceResponse::Streams { streams: vec![] }))
//             })
//             .build();
//     }
// }
