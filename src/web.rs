use crate::error::Result;
use actix_web::{
    get, middleware, web, App, Error as ActixError, HttpResponse, HttpServer, Responder,
};
use diesel::prelude::*;
use std::sync::Mutex;

#[get("/static/styles.css")]
fn static_css() -> &'static str {
    include_str!("../static_web/styles.css")
}

#[get("/comic/{comic_id}/{episode_id}")]
fn comic_pics(
    path: web::Path<(String, i32)>,
    data: web::Data<Mutex<SqliteConnection>>,
) -> impl Responder {
    use crate::models::ComicRecord;
    use crate::schema::lezhin::dsl::*;

    let (comic_id, episode_id) = path.into_inner();
    let conn = data.lock().unwrap();

    let recs = lezhin
        .filter(comic.eq(comic_id))
        .filter(episode_seq.eq(episode_id))
        .order_by(picture_seq)
        .load::<ComicRecord>(&*conn)
        .map_err(actix_web::error::ErrorInternalServerError)?;

    fn into_embedded_image(rec: &ComicRecord) -> String {
        match &rec.picture {
            Some(inner) => format!(
                r#"<img alt="image sequence {}" src="data:image/jpg;base64,{}">"#,
                rec.picture_seq,
                base64::encode(&inner)
            ),
            None => format!("image sequence {} does not have content", rec.picture_seq),
        }
    }

    let resp = recs
        .iter()
        .map(into_embedded_image)
        .collect::<Vec<String>>()
        .join("");

    Ok::<HttpResponse, ActixError>(HttpResponse::Ok().content_type("text/html").body(format!(
        r#"<html>
<head>
    <meta charset="UTF-8"> 
    <link rel="stylesheet" href="/static/styles.css">
</head>
Found {} records, response size {}MiB, title {}<br />
{}
<div align="center">
    <a class="next-link" href="{}">Next</a>
</div>
</html>"#,
        recs.len(),
        f64::from(resp.bytes().len() as u32) / (1024f64 * 1024f64),
            recs.iter()
                .map(|x| x.episode.clone())
                .flatten()
                .next()
                .unwrap_or_else(|| String::from("(unknown)")),
        resp,
        episode_id + 1,
    )))
}

pub fn serve(addr: impl std::net::ToSocketAddrs, conn: SqliteConnection) -> Result<()> {
    let data = web::Data::new(Mutex::new(conn));

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .register_data(data.clone())
            .service(static_css)
            .service(comic_pics)
    })
    .bind(addr)?
    .run()?;
    Ok(())
}
