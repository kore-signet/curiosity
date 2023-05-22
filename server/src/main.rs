#![allow(unused_must_use)]

use std::sync::Arc;

use std::error::Error;

use actix_cors::Cors;
use actix_web::{web, App, HttpServer};

use curiosity::db::Db;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>> {
    std::fs::create_dir_all("./satt");
    let mut db: Db = Db::new("./satt")?;
    server::update::update_database(db.clone()).await.unwrap();

    Arc::get_mut(&mut db.store.db).unwrap().compact();

    let db_for_update = db.clone();
    actix_web::rt::spawn(server::update::update_database_periodically(db_for_update));

    HttpServer::new(move || {
        App::new()
            .wrap(Cors::permissive())
            .service(web::scope("/api").service(server::api::search))
            .service(actix_files::Files::new("/", "./static").index_file("index.html"))
            .app_data(web::Data::new(db.clone()))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await?;

    Ok(())
}
// fn main() {}
