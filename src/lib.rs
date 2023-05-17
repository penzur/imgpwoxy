use image::imageops::FilterType;
use image::io::Reader as ImageReader;
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
            // parse url query
            let url = match req.url() {
                Ok(url) => url,
                Err(_) => return Response::error("failed to parse url", 400),
            };

            let qs = url
                .query()
                .unwrap_or("")
                .split("&")
                .map(|kv| {
                    let mut kv = kv.split("=");
                    (kv.next().unwrap_or(""), kv.next().unwrap_or(""))
                })
                .collect::<std::collections::HashMap<&str, &str>>();

            // extract url, width, and height
            let url = match qs.get("url") {
                Some(url) => url.to_owned(),
                None => return Response::error("url not found", 400),
            };
            // defaults both w and h to 256
            let width = qs
                .get("width")
                .to_owned()
                .unwrap_or(&"")
                .to_owned()
                .parse::<u32>()
                .unwrap_or(256);
            let height = qs
                .get("height")
                .to_owned()
                .unwrap_or(&"")
                .to_owned()
                .parse::<u32>()
                .unwrap_or(width.clone());

            // let's fetch the image
            let req = match Request::new(url, worker::Method::Get) {
                Ok(req) => req,
                Err(_) => return Response::error("failed to create request", 400),
            };
            // fetch that shit!
            let f_u = Fetch::Request(req);
            let mut resp = match f_u.send().await {
                Ok(resp) => resp,
                Err(_) => return Response::error("failed to fetch image", 400),
            };
            let image_bytes = match resp.bytes().await {
                Ok(image_bytes) => image_bytes,
                Err(_) => return Response::error("failed to process image", 400),
            };

            // process image
            let cursor_bytes = Cursor::new(&image_bytes);
            let img_reader = match ImageReader::new(cursor_bytes).with_guessed_format() {
                Ok(img) => img,
                Err(_) => return Response::error("failed to process image", 400),
            };
            let img = match img_reader.decode() {
                Ok(img) => img,
                Err(_) => return Response::error("failed to process image", 400),
            };
            // shadow that shit
            let resized_img = img.resize_to_fill(width, height, FilterType::Lanczos3);

            // reponse buffer
            let mut buf = Cursor::new(Vec::new());
            match resized_img.write_to(&mut buf, image::ImageOutputFormat::Jpeg(80)) {
                Ok(_) => (),
                Err(_) => return Response::error("failed to process image", 400),
            };

            let resp = match Response::from_bytes(buf.into_inner()) {
                Ok(resp) => resp,
                Err(_) => return Response::error("failed to generate response", 400),
            };

            // headers
            let mut headers = worker::Headers::new();
            match headers.set("Content-Type", "image/jpeg") {
                Ok(_) => (),
                Err(_) => return Response::error("failed to generate response", 400),
            }

            // gg!
            Ok(resp.with_status(200).with_headers(headers))
        })
        .run(req, env)
        .await
}
