# imgpwoxy 🚀

img-proxy on the edge 🤣🙈

---

**TODOS:**

- [x] resize with aspect ratio
- [x] encode url
- [x] add cache

---

**Requirements**

- [wrangler](https://developers.cloudflare.com/workers/wrangler/install-and-update/)
- [Rust](https://www.rust-lang.org/tools/install)

---

**How to dev**

run `wrangler dev -l --no-bundle`

---

**How to publish**

run `wrangler publish`

> don't forget to update `wrangler.toml` before publishing

---

**How to try**

Try the following link with your browser of choice.

`https://imgpwoxy.<your-cf-username>.workers.dev?width=<N>&height=<N>&url=<EncodeURI(S)>`

> make sure you encode the url string

---

🖖🤓
