use crate::error::Result;
use actix_web::{
    get, middleware, web, App, Error as ActixError, HttpResponse, HttpServer, Responder,
};
use diesel::prelude::*;
use std::sync::Mutex;

#[get("/")]
fn redirect_root() -> impl Responder {
    HttpResponse::Found()
        .header(actix_web::http::header::LOCATION, "/list-comics")
        .finish()
        .into_body()
}

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
    use crate::schema::comics::dsl::*;

    let (comic_id_, episode_id) = path.into_inner();
    let conn = data.lock().unwrap();

    let recs = comics
        .filter(comic_id.eq(comic_id_))
        .filter(episode_seq.eq(episode_id))
        .order_by(image_seq)
        .load::<ComicRecord>(&*conn)
        .map_err(actix_web::error::ErrorInternalServerError)?;

    fn into_embedded_image(rec: &ComicRecord) -> String {
        format!(
            r#"<img alt="image sequence {}" src="data:image/jpg;base64,{}">"#,
            rec.image_seq,
            base64::encode(&rec.image)
        )
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
                .map(|x| x.episode_name.clone())
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
            rec.title.unwrap_or_else(|| String::from("title unknown")),
            rec.id,
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
    use crate::schema::comics::dsl::*;

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

    let eps = comics
        .select((comic_id, episode_name, episode_seq))
        .distinct()
        .filter(comic_id.eq(target_id))
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
            .service(redirect_root)
    })
    .bind(addr)?
    .run()?;
    Ok(())
}
