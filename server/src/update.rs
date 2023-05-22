use std::io::{Cursor, Read};

use std::time::Duration;
use std::{collections::BTreeMap, path::Path};

use curiosity::{db::Db, CuriosityError, Season, SeasonId};

use crate::ServerResult;

pub async fn update_database_periodically(db: Db) -> ServerResult<()> {
    let mut interval = actix_web::rt::time::interval(Duration::from_secs(6 * 60 * 60));
    loop {
        interval.tick().await;
        println!("Updating!");

        if let Err(e) = update_database(db.clone()).await {
            println!("error during db update: {e}");
        }
    }
}

pub async fn update_database(db: Db) -> ServerResult<()> {
    let mirror_bytes = reqwest::get("https://github.com/emily-signet/transcripts-at-the-table-mirror/archive/refs/heads/data.zip").await?.bytes().await?;

    actix_web::rt::task::spawn_blocking(move || {
        let mut mirror = zip::ZipArchive::new(Cursor::new(mirror_bytes.as_ref()))?;
        let seasons: BTreeMap<SeasonId, Season> = serde_json::from_reader(
            mirror.by_name("transcripts-at-the-table-mirror-data/seasons.json")?,
        )?;

        db.add_documents(seasons.values(), |_, episode| {
            println!("reading {}", episode.title);
            let mut f = mirror
                .by_name(
                    (Path::new("transcripts-at-the-table-mirror-data/")
                        .join(episode.download.as_ref().unwrap().plain.clone()))
                    .to_str()
                    .unwrap(),
                )
                .map_err(|_| CuriosityError::NotFound)?; // does the error fit? no. will i use it regardless? yes.

            let mut out = String::with_capacity((f.compressed_size() * 2) as usize);
            f.read_to_string(&mut out)?;
            Ok(out)
        })?;

        Ok(())
    })
    .await
    .unwrap()
}
