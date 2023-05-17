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
                Err(_) => return Response::error("failed to parse url", 404),
            };

            // check if cache exists
            let c = Cache::default();
            let key = url.to_string();
            let cached_response = match c.get(&key, true).await {
                Ok(cached_response) => cached_response,
                Err(_) => None,
            };
            if let Some(cached_response) = cached_response {
                return Ok(cached_response);
            }

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
                None => return Response::error("url could not be found", 404),
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
                .unwrap_or(256);

            // let's fetch the image
            let req = match Request::new(url, worker::Method::Get) {
                Ok(req) => req,
                Err(_) => return Response::error("failed to create request", 404),
            };
            // fetch that shit!
            let f_u = Fetch::Request(req);
            let mut resp = match f_u.send().await {
                Ok(resp) => resp,
                Err(_) => return Response::error("failed to fetch image", 404),
            };
            let image_bytes = match resp.bytes().await {
                Ok(image_bytes) => image_bytes,
                Err(_) => return Response::error("failed to process image", 404),
            };

            // create image from response
            let cursor_bytes = Cursor::new(&image_bytes);
            let img_reader = match ImageReader::new(cursor_bytes).with_guessed_format() {
                Ok(img) => img,
                Err(_) => return Response::error("failed to process image", 404),
            };
            let img = match img_reader.decode() {
                Ok(img) => img,
                Err(_) => return Response::error("failed to process image", 404),
            };
            let resized_img = img.resize_to_fill(width, height, FilterType::Lanczos3);

            // create response buffer and write the resized_img bytes to it
            let mut buf = Cursor::new(Vec::new());
            match resized_img.write_to(&mut buf, image::ImageOutputFormat::Jpeg(90)) {
                Ok(_) => (),
                Err(_) => return Response::error("failed to process image", 404),
            };

            // construct new response
            let resp = match Response::from_bytes(buf.into_inner()) {
                Ok(resp) => resp,
                Err(_) => return Response::error("failed to generate response", 404),
            };
            // with the following headers
            let mut headers = worker::Headers::new();
            match headers.set("Content-Type", "image/jpeg") {
                Ok(_) => (),
                Err(_) => return Response::error("failed to generate response", 404),
            }
            // cache control headers
            match headers.set("Cache-Control", "public, max-age=31536000") {
                Ok(_) => (),
                Err(_) => return Response::error("failed to generate response", 404),
            }
            let resp = resp.with_status(200).with_headers(headers);
            Ok(resp)
        })
        .run(req, env)
        .await
}
