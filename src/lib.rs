use reqwest::Client;
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
            let wcli = Client::new();

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

                // fetch that shit! and resize like a real mo'fucka!
                // get url from params
                let img_url = params.get("url").unwrap().to_owned();
                // get img_url using reqwest Client
                console_log!("img url: {}", img_url);
                let pl = wcli
                    .get(img_url)
                    .send()
                    .await
                    .unwrap()
                    .bytes()
                    .await
                    .unwrap();
                let img = image::load_from_memory(&pl).unwrap();
                img.resize_exact(200, 200, image::imageops::FilterType::Lanczos3);
                return Response::from_bytes(img.into_bytes());
            }
            Response::ok("ImgPWOxy!")
        })
        .run(req, env)
        .await
}
