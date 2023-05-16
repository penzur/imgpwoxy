use reqwest::Client;
use std::io::Cursor;
use worker::*;

mod utils;

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or_else(|| "unknown region".into())
    );
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    log_request(&req);

    utils::set_panic_hook();

    let router = Router::new();
    router
        .get_async("/", |req, _ctx| async move {
            let client = Client::new();

            if let Some(query) = req.url().unwrap().query() {
                let params = query
                    .split("&")
                    .map(|kv| {
                        let mut kv = kv.split("=");
                        (kv.next().unwrap_or(""), kv.next().unwrap_or(""))
                    })
                    .collect::<std::collections::HashMap<&str, &str>>();

                if !params.contains_key("url") {
                    return Response::error("Missing url parameter", 400);
                }

                let url = params.get("url").unwrap().to_owned();
                let width = params
                    .get("width")
                    .unwrap_or(&&"")
                    .parse::<u32>()
                    .unwrap_or_default();
                let height = params
                    .get("height")
                    .unwrap_or(&&"")
                    .parse::<u32>()
                    .unwrap_or_default();

                let img_bytes = client.get(url).send().await.unwrap().bytes().await.unwrap();
                let img = image::io::Reader::new(Cursor::new(&img_bytes))
                    .with_guessed_format()
                    .unwrap()
                    .decode()
                    .unwrap();

                // resize
                let img = image::imageops::resize(
                    &img,
                    width,
                    height,
                    image::imageops::FilterType::Lanczos3,
                );

                // create response
                let mut buf = Cursor::new(Vec::new());
                img.write_to(&mut buf, image::ImageOutputFormat::Jpeg(80))
                    .unwrap();
                if let Ok(resp) = Response::from_bytes(buf.into_inner()) {
                    let mut heads = worker::Headers::new();
                    heads.set("Content-Type", "image/jpeg").unwrap();
                    return worker::Result::Ok(resp.with_status(200).with_headers(heads));
                }
            }
            Response::ok("ImgPWOxy!")
        })
        .run(req, env)
        .await
}
