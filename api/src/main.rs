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

type CacheData = web::Data<Arc<Cache>>;

#[post("/api/v3/updates")]
async fn updates(cache: CacheData, req: web::Json<UpRequest>) -> web::Json<UpResponse> {
    web::Json(Updates::calc_updates(&cache.get_ref(), req.into_inner()).unwrap())
}


#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env::set_var("RUST_LOG", "actix_web=trace,trace");
    env_logger::init();
    let cache = upcache::cache::load("./vmaas.db").unwrap();
    /*
    let repos = vec![
        "rhel-7-desktop-extras-beta-rpms",
        "rhel-7-server-satellite-tools-6.5-debug-rpms",
        "rhel-5-client-xfs-debug-rpms",
        "rhel-7-server-satellite-tools-6.5-rpms",
        "rhel-3-es-for-itanium-rpms",
        "rhel-6-desktop-satellite-tools-6.1-rpms",
        "rhel-6-desktop-satellite-tools-6.1-rpms"
    ].into_iter().map(ToOwned::to_owned).collect();

    let up = Updates::calc_updates(&cache, UpRequest {
        package_list: vec!["python-qpid-proton-0.9-4.el6.i686".to_string()],
        repository_list: Some(repos),
        .. Default::default()
    }).unwrap();

    panic!("{:?}", up);
    */
    let cache = Arc::new(cache);

    info!("Loaded cache");
    HttpServer::new(move || App::new()
        .service(updates)
        .app_data(JsonConfig::default().limit(4 * 1024 * 1024))
        .data(cache.clone())
    )
        .bind("127.0.0.1:1080")?
        .run()
        .await?;
    Ok(())
}