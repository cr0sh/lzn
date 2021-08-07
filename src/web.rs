use crate::error::Result;
use diesel::prelude::*;
use std::io::{Cursor, Empty};
use std::str::FromStr;
use tiny_http::{Header, Response, StatusCode};

type BytesResponse = Response<Cursor<Vec<u8>>>;

fn redirect_root() -> Response<Empty> {
    Response::empty(StatusCode(301))
        .with_header(Header::from_str("Location: /list-comics").unwrap())
}

fn static_css() -> BytesResponse {
    Response::from_string(include_str!("../static_web/styles.css"))
}

fn comic_pics(
    comic_id_: String,
    episode_id: i32,
    conn: &SqliteConnection,
) -> Result<BytesResponse> {
    use crate::models::ComicRecord;
    use crate::schema::comics::dsl::*;

    let recs = comics
        .filter(comic_id.eq(comic_id_))
        .filter(episode_seq.eq(episode_id))
        .order_by(image_seq)
        .load::<ComicRecord>(&*conn)?;

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

    Ok(Response::from_string(format!(
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
    ))
    .with_header(Header::from_str("Content-Type: text/html; charset=utf-8").unwrap()))
}

fn list_comics(conn: &SqliteConnection) -> Result<BytesResponse> {
    use crate::models::TitleRecord;
    use crate::schema::titles::dsl::*;

    let tvec = titles.order_by(title).load::<TitleRecord>(&*conn)?;

    fn into_list_row(rec: TitleRecord) -> String {
        format!(
            r#"<a href="/list-episodes/{}">{} ({})</a><br>"#,
            rec.id,
            rec.title.unwrap_or_else(|| String::from("title unknown")),
            rec.id,
        )
    }

    Ok(Response::from_string(format!(
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
            .join("")
    ))
    .with_header(Header::from_str("Content-Type: text/html; charset=utf-8").unwrap()))
}

fn list_episodes(target_id: String, conn: &SqliteConnection) -> Result<BytesResponse> {
    use crate::schema::episodes::dsl::*;

    fn into_list_row((_comic, _episode, _episode_seq): (String, Option<String>, i32)) -> String {
        format!(
            r#"<a href="/comic/{}/{}">{}</a><br>"#,
            _comic,
            _episode_seq,
            _episode.unwrap_or_else(|| String::from("title unknown")),
        )
    }

    let eps = episodes
        .select((id, title, seq))
        .distinct()
        .filter(id.eq(target_id))
        .order_by(seq)
        .load(&*conn)?
        .into_iter()
        .map(into_list_row)
        .collect::<Vec<String>>()
        .join("");

    Ok(Response::from_string(format!(
        r#"<html>
<head>
	<meta charset="UTF-8"> 
</head>
<body>        
{}
</body>
</html>"#,
        eps
    ))
    .with_header(Header::from_str("Content-Type: text/html; charset=utf-8").unwrap()))
}

pub fn serve(addr: impl std::net::ToSocketAddrs, conn: SqliteConnection) {
    let server = tiny_http::Server::http(addr).unwrap();

    for request in server.incoming_requests() {
        if request.method() != &tiny_http::Method::Get {
            continue;
        }

        log::debug!("Addr: {}, URL: {}", request.remote_addr(), request.url());

        macro_rules! respond {
            ($req:expr, $resp:expr) => {
                if let Err(err) = $req.respond($resp) {
                    log::error!("Error while responding to request: {}", err);
                }
            };
        }

        match request.url().to_owned().as_ref() {
            "/" => {
                respond!(request, redirect_root());
            }
            "/list-comics" => {
                respond!(request, list_comics(&conn).unwrap());
            }
            "/static/styles.css" => {
                respond!(request, static_css());
            }
            url => {
                if let Some(episode_path) = url.strip_prefix("/list-episodes/") {
                    respond!(
                        request,
                        list_episodes(String::from(episode_path), &conn).unwrap()
                    )
                } else if let Some(path) = url.strip_prefix("/comic/") {
                    let splits = path.split('/').collect::<Vec<_>>();
                    respond!(
                        request,
                        comic_pics(
                            splits[0].to_string(),
                            splits[1].parse::<i32>().unwrap(),
                            &conn
                        )
                        .unwrap()
                    )
                } else {
                    respond!(request, Response::from_string("Unknown request"));
                }
            }
        };
    }
}
