use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use vercel_runtime::Body;

use crate::request::{HyperRequest, Request, ServerlessRequest};
use crate::response::{HyperResponse, ServerlessResponse};
use crate::response::Response;
use crate::router::Router;

#[derive(Debug, Clone)]
pub struct ServerOptions {
    pub ip: IpAddr,
    pub port: u16,
    pub cache_max_age: i32,
    pub landing_html: String,
}

impl Default for ServerOptions {
    fn default() -> Self {
        Self {
            ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port: 43001,
            cache_max_age: 24 * 3600 * 3, // cache 3 days,
            landing_html: "<html>Hello World</html>".into(),
        }
    }
}

pub async fn serve_http(
    router: Router,
) -> Result<HyperResponse<String>, Box<dyn Error + Send + Sync + 'static>> {
    let options = router.server_options();
    let addr = SocketAddr::new(options.ip, options.port);
    let listener = TcpListener::bind(addr).await?;
    println!("Running on: {}", addr);
    loop {
        let stream = listener.accept().await?.0;
        let io = TokioIo::new(stream);
        let router_arc = Arc::new(router.clone());
        let service = service_fn(move |req: HyperRequest<hyper::body::Incoming>| {
            println!("Incoming request: {}", req.uri());
            let router_arc_clone = router_arc.clone();
            async move {
                router_arc_clone
                    .route(Request::Hyper(req))
                    .await
                    .map(|res: Response<String>| match res {
                        Response::Hyper(res) => res,
                        _ => unreachable!(),
                    })
            }
        });
        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                eprintln!("connection error: {:?}", err);
            }
        });
    }
}

pub async fn serve_serverless(
    router: Router,
    request: ServerlessRequest,
) -> Result<ServerlessResponse<Body>, Box<dyn Error + Send + Sync + 'static>> {
    router
        .route::<Body, Body>(Request::Serverless(request))
        .await
        .map(|res: Response<Body>| match res {
            Response::Serverless(res) => res,
            _ => unreachable!(),
        })
        .map_err(|e| e.into())
}
