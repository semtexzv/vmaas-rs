use upcache::{
    Cache,
    calc::updates::{
        Updates,
        Request as UpRequest,
        Response as UpResponse,
    },
};

use log::info;
use actix_web::{get, post, web, App, HttpServer, Responder, FromRequest};
use std::sync::Arc;
use std::env;
use actix_web::web::JsonConfig;
use actix_web::middleware::Logger;

type CacheData = web::Data<Arc<Cache>>;

#[post("/api/v3/updates")]
async fn updates(cache: CacheData, req: web::Json<UpRequest>) -> web::Json<UpResponse> {
    web::Json(Updates::calc_updates(&cache.get_ref(), req.into_inner()).unwrap())
}


#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env::set_var("RUST_LOG", "actix_web=trace,actix_server=debug,debug");
    env_logger::init();
    info!("Starting up");
    let cache = upcache::cache::load("./vmaas.db").unwrap();
    let cache = Arc::new(cache);

    info!("Loaded cache");
    HttpServer::new(move || App::new()
        .wrap(Logger::new("%t|%s|%D ms|%a|%u"))
        .service(updates)
        .app_data(JsonConfig::default().limit(4 * 1024 * 1024))
        .data(cache.clone())
    )
        .bind("127.0.0.1:1080")?
        .run()
        .await?;
    Ok(())
}