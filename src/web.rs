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

#[get("/list-comics")]
fn list_comics(data: web::Data<Mutex<SqliteConnection>>) -> impl Responder {
    use crate::models::TitleRecord;
    use crate::schema::titles::dsl::*;

    let conn = data.lock().unwrap();

    let tvec = titles
        .order_by(title)
        .load::<TitleRecord>(&*conn)
        .map_err(actix_web::error::ErrorInternalServerError)?;

    fn into_list_row(rec: TitleRecord) -> String {
        format!(
            r#"<a href="/list-episodes/{}">{} ({})</a><br>"#,
            rec.id,
            rec.id,
            rec.title.unwrap_or_else(|| String::from("title unknown"))
        )
    }

    Ok::<_, ActixError>(HttpResponse::Ok().content_type("text/html").body(format!(
        r#"<html>
<head>
    <meta charset="UTF-8"> 
</head>
<body>        
{}
</body>
</html>"#,
        tvec.into_iter()
            .map(into_list_row)
            .collect::<Vec<String>>()
            .join(""))))
}

#[get("/list-episodes/{comic_id}")]
fn list_episodes(
    path: web::Path<String>,
    data: web::Data<Mutex<SqliteConnection>>,
) -> impl Responder {
    use crate::schema::lezhin::dsl::*;

    let target_id = path.into_inner();
    let conn = data.lock().unwrap();

    fn into_list_row((_comic, _episode, _episode_seq): (String, Option<String>, i32)) -> String {
        format!(
            r#"<a href="/comic/{}/{}">{} ({})</a><br>"#,
            _comic,
            _episode_seq,
            _episode.unwrap_or_else(|| String::from("title unknown")),
            _episode_seq,
        )
    }

    let eps = lezhin
        .select((comic, episode, episode_seq))
        .distinct()
        .filter(comic.eq(target_id))
        .order_by(episode_seq)
        .load(&*conn)
        .map_err(actix_web::error::ErrorInternalServerError)?
        .into_iter()
        .map(into_list_row)
        .collect::<Vec<String>>()
        .join("");

    Ok::<_, ActixError>(HttpResponse::Ok().content_type("text/html").body(format!(
        r#"<html>
<head>
	<meta charset="UTF-8"> 
</head>
<body>        
{}
</body>
</html>"#,
        eps
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
            .service(list_comics)
            .service(list_episodes)
    })
    .bind(addr)?
    .run()?;
    Ok(())
}
