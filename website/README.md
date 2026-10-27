# Noor — Thobe House

Trilingual (EN / FR / AR) thobe storefront for Algeria, with a small
self-hosted backend. No build step and no npm runtime dependencies: Python 3
standard library + vanilla JavaScript.

- Storefront: thobes, djellabas and shirts; sizes 52–58; per-size cart lines.
- Checkout: all 58 wilayas, home delivery or stopdesk, DZD, cash on delivery.
- Express order: from any quick view, size and quantity carry into a compact
  one-product checkout (name, phone, wilaya, commune, carrier, home/stopdesk).
  Customer details are saved on the device for repeat orders; reviews are
  collapsed so the order action stays prominent.
- Admin: orders and status flow, products/stock, discounts, branding, themes,
  event banner, low-stock alerts, revenue, WhatsApp handoff, social pages, JSON backup.
- Contact & sponsorship popup at `#contact`, with WhatsApp, Instagram, Facebook,
  TikTok, media-kit copy and a floating WhatsApp button. Edit the links in
  Admin → Settings → Contact & social pages. The three URLs shipped in
  `settings.json` are examples — replace them before launch.
- Images: responsive WebP variants (`images/opt/*-320/640/896.webp`) with JPEG
  fallback; photos added from the admin are resized in the browser before upload.
  Fonts are self-hosted WOFF2 subsets.

## Run locally

```bash
cd noor
ADMIN_PIN='choose-a-strong-pin' TRUST_PROXY=0 python3 server.py
# http://127.0.0.1:8124/
```

Handy environment variables:

| Variable | Default | Meaning |
|---|---|---|
| `PORT` | `8124` | HTTP port |
| `HOST` | `127.0.0.1` | Bind address; use `0.0.0.0` only behind a proxy/firewall |
| `ADMIN_PIN` | — | Admin login. Must be 8+ chars and not a known weak PIN; admin stays disabled otherwise |
| `TRUST_PROXY` | `0` | Set `1` only behind a trusted reverse proxy, so `X-Forwarded-For` is used for rate limits |
| `BEHIND_TLS` | `1` | Sends HSTS and marks cookies `Secure` (TLS terminated upstream) |
| `NOOR_DB` | `data/noor.sqlite3` | SQLite path for local state |

`.env` is loaded automatically by both `store.py` and `noor_app.py` and is also
read by `./restart.sh` for validation. The restart script refuses to start
without a strong `ADMIN_PIN`, defaults to `HOST=127.0.0.1` / `TRUST_PROXY=1` /
`BEHIND_TLS=1`, and only stops the PID it started, so it will not kill a test
server.

## Deploy to Octenium (`https://octenium.com/`)

### Option A — Octenium Shared Hosting (cPanel · Particulier / Professionnel / Entreprise)

Octenium's shared web hosting plans include **cPanel** with **CloudLinux Python
Selector (Phusion Passenger)**, **Nginx® acceleration**, and free SSL.

1. **Upload the project files**:
   - Log in to **Octenium Client Area → Connexion au cPanel → File Manager**.
   - Upload and extract the project archive into a dedicated directory (for
     example `/home/<cpanel_user>/noor-app`) or directly into your domain's
     application root, ensuring `.htaccess` and `passenger_wsgi.py` are present.
2. **Create the Python Application in cPanel**:
   - Open **Software → Setup Python App** in cPanel and click **Create Application**.
   - **Python version**: Select **Python 3.9+** (3.10 / 3.11 / 3.12 recommended).
   - **Application root**: `noor-app` (or your chosen folder).
   - **Application URL**: Select your domain (root `/`).
   - **Application startup file**: `passenger_wsgi.py`
   - **Application Entry point**: `application`
3. **Configure Environment Variables**:
   - Either add environment variables in the **Setup Python App** screen or copy
     `.env.example` to `.env` (`chmod 600 .env`) in the application root:
     - `ADMIN_PIN`: Your strong admin PIN (at least 8 characters, not a common word).
     - `TRUST_PROXY`: `1` (Octenium routes traffic through its local Nginx/Apache proxy).
     - `BEHIND_TLS`: `1` (Octenium enables free SSL/HTTPS automatically).
4. **Start & Verify**:
   - Click **Create / Save** and then **Restart** in Setup Python App.
   - Visit `https://yourdomain.dz/api/health` — it should return
     `{"ok": true, "backend": "sqlite"}`.
   - Visit `https://yourdomain.dz/#admin` and sign in with your `ADMIN_PIN` to
     verify the store name, WhatsApp number, and social links.

> **Security on Apache/cPanel**: The included `.htaccess` automatically blocks
> direct HTTP access to `.env`, `*.py`, `data/noor.sqlite3`, `*.bak`, and
> private runtime JSON mirrors (`orders.json`, `users.json`, `tokens.json`,
> `settings.json`, `reviews.json`, `logo.json`, `counter.json`). Keep
> `.htaccess` in the application/document root and set `.env` permissions to `0600`.

### Option B — Octenium VPS (`Serveur dédié VPS`)

1. Copy the project to `/opt/noor` (or `/var/www/noor`), create `.env` from
   `.env.example` (`chmod 600 .env`) with a strong `ADMIN_PIN`, `TRUST_PROXY=1`,
   and `BEHIND_TLS=1`.
2. Start the server with `./restart.sh` (listens on `127.0.0.1:8124`) or run it
   under `systemd` / Gunicorn (`gunicorn -w 2 -k gthread --threads 8 -b 127.0.0.1:8124 wsgiapp:application`).
3. Proxy traffic from Nginx/Apache over HTTPS to `http://127.0.0.1:8124` with
   `X-Forwarded-For` and `X-Real-IP` headers set.

## Deploy to Vercel + Upstash Redis

Vercel's filesystem is ephemeral, so production state uses Upstash Redis
through `store.py`. The same code auto-selects Redis when these variables exist;
local development keeps using SQLite.

1. Create a free Upstash Redis database and copy its REST URL + token.
2. Add the variables and deploy:

```bash
npm i -g vercel
vercel login
vercel env add UPSTASH_REDIS_REST_URL production
vercel env add UPSTASH_REDIS_REST_TOKEN production
vercel env add ADMIN_PIN production        # 8+ chars, not noor-admin
vercel env add TRUST_PROXY production      # value: 1
vercel env add BEHIND_TLS production       # value: 1
vercel --prod
```

`api/*.py` are thin routes over `wsgiapp.py`, which loads `noor_app.py`; static
files are served by the CDN. `vercel.json` carries the security headers, CSP and
blocks the backend source files from static URLs. `.vercelignore` keeps the
local-only `server.py` wrapper, secrets, the database and runtime JSON out of the
upload while leaving the modules needed by the functions in the bundle.

## Security notes

- Admin and customer passwords/tokens are hashed (PBKDF2-SHA256, 310k rounds)
  with transparent upgrades of older hashes; tokens are stored hashed.
- Admin login has per-IP throttling and lockout after repeated failures.
- Server enforces stock, size, quantity (1–20 per line), 24-line and 60-item
  limits; prices and totals are computed server-side, never trusted from the
  browser.
- Order creation is transactional; retries with the same `Idempotency-Key` do
  not double-decrement stock. Cancelling an order restocks each line exactly
  once, including concurrent cancellation requests.
- Sensitive files (`noor_app.py`, `store.py`, `wsgiapp.py`, `passenger_wsgi.py`,
  `.env`, `orders.json`, `*.bak`, `data/`, SQLite files, `logo.json`) are never
  served statically (`noor_app.py` allowlist + `.htaccess` on Apache/cPanel +
  `vercel.json` on Vercel CDN).
- CSP is self-only (no Google Fonts/CDNs), with `frame-ancestors 'none'`,
  `nosniff`, COOP/CORP and a Permissions-Policy; HSTS is on by default.
- Uploaded logos are size/type/MIME checked, stored as `logo.json` in backend
  storage and served from `/api/logo` — this survives read-only/serverless
  deployments.

Set `chmod 600 .env`, use a unique admin PIN, and serve over HTTPS. Rotate the
PIN if it has ever been shared.

## Owner guide

- **Shop** (`#shop`): search, category filters, size chips, quick view, reviews.
- **Express order**: in a quick view, choose a size then press **Order now**.
  Size and quantity are already set; confirm the customer details and carrier,
  then place the order. It creates the same server-side order as bag checkout,
  without emptying the bag. The delivery data is remembered locally for the
  next express order. **Add to bag** remains available for multi-item orders.
- **Checkout**: name, Algerian `05/06/07` phone, 58 wilayas, commune, carrier
  (Noor Standard / Maystro / Noest / EMS), home or stopdesk, promo code, COD.
- **Admin** (`#admin`, footer link): six task-oriented tabs —
  Overview, Orders, Products, Storefront, Discounts, Settings.
- **Orders**: confirm → shipped → delivered; cancel returns stock. Search by
  number, name or phone; call/WhatsApp the customer from the card.
- **Products**: create/edit/delete, stock, price, sizes, category, and up to 8
  photos chosen from the device (first photo is the cover; tap another to make
  it the cover). Select several products to delete them together. Stock is
  decremented atomically at checkout.
- **Storefront**: brand name, logo upload/removal, theme presets/custom colors,
  announcement banner.
- **Discounts**: add/enable/disable/delete percentage codes.
- **Settings**: WhatsApp number, backup download, logout.
- Customers can share a product with `/#/p/<product-id>`; the quick view opens
  directly.

## Data files

| File | Content |
|---|---|
| `products.json` | Catalog seed + human-readable mirror; SQLite/Redis is live truth |
| `orders.json` | Orders mirror (`.bak` rotation on save) |
| `users.json` | PBKDF2-hashed passwords, never plaintext |
| `tokens.json` | Hashed, expiring login/admin tokens |
| `reviews.json` | Product reviews (capped) |
| `analytics.jsonl` | View/add/order events for admin stats |
| `settings.json` | Brand, WhatsApp, theme, banner, promos, logo reference |
| `shipping.json` | Carrier fees and wilaya zones |
| `logo.json` | Uploaded logo bytes (base64) + MIME type |
| `data/noor.sqlite3` | Local transactional store (not for static serving) |
| `images/opt/` | Responsive WebP product variants |

Back up `orders.json`, `products.json`, `users.json` (Admin → Backup JSON) and
the SQLite database/Redis instance regularly.

## Tests and performance

The repository was verified with:

- API/security suite: 56 checks (headers, static exposure, auth, server-side
  pricing, promos, stock, idempotency, cancellation restock, reviews,
  tracking whitelist, product CRUD, oversized bodies).
- Concurrency suite: last-item race, concurrent same-key orders, concurrent
  cancellation, order caps.
- Browser flows: EN/FR/AR RTL, shop/filters, quick view, size guide, cart,
  checkout, admin login/tabs/order cancellation, logo upload, stale-cart
  sanitisation.
- Lighthouse mobile: **85 performance**, 100 accessibility, 100 best practices,
  100 SEO (desktop: 98 / 100 / 100 / 100).

## Shipping data

`shipping.json` holds per-carrier home/stopdesk tables by wilaya zone. Free
delivery over 8,000 DZD applies to Noor Standard. Figures are editable
estimates — confirm against each carrier's current sheet before launch.
