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
cd website
ADMIN_PIN='choose-a-strong-pin' TRUST_PROXY=0 BEHIND_TLS=0 python3 server.py
# http://127.0.0.1:8124/
```

Add `MAIL_OUTBOX=outbox MAIL_FROM=test@example.com ADMIN_EMAIL=you@example.com`
to write emails as `.eml` files in `outbox/` instead of sending them.

## Deploy on Octenium cPanel

The store runs only on Octenium shared hosting (cPanel + "Setup Python App" /
Phusion Passenger). Everything uses the Python standard library: there is
nothing to `pip install`.

1. **Upload** — cPanel → File Manager → upload the project into the app folder
   (for example `/home/<cpanel_user>/kaser-app`). Never upload `.env` from your
   computer if it was ever shared; create a fresh one on the server.
2. **Python app** — cPanel → Setup Python App → Create Application:
   Python 3.11, application root `kaser-app`, URL = your domain,
   startup file `passenger_wsgi.py`, entry point `application`.
3. **`.env`** — copy `.env.example` to `.env` in the app folder, fill it in,
   then set its permissions to `600`:
   - `ADMIN_PIN`: a new strong one-time PIN (see "First sign-in" below).
   - `SITE_URL`: `https://yourdomain.dz`.
   - Email: create a mailbox in cPanel → Email Accounts (for example
     `orders@yourdomain.dz`); its "Connect Devices" page shows the SMTP host
     and port. Put them in `SMTP_*`, `MAIL_FROM`, `ADMIN_EMAIL` (where you want
     order alerts) and `BACKUP_EMAIL` (a mailbox *outside* this server, such as
     Gmail, for the off-site backup copy).
4. **Restart** the app in Setup Python App and open `https://yourdomain.dz/api/health`
   — it should show `{"ok": true, "backend": "sqlite"}`.
5. **Check email** — cPanel → Terminal (or Setup Python App → "Execute python
   script"): `python3 manage.py test-email`.

### First sign-in and 2-step verification

1. Open `https://yourdomain.dz/#admin`, leave **Username** empty and type the
   `ADMIN_PIN` as the password.
2. The Settings page opens. In **Sign-in & security** choose a username and a
   password (10+ characters), confirm with the PIN, and save. From now on the
   PIN no longer works.
3. Click **Turn on 2-step verification**, scan the QR code with Google
   Authenticator (or similar), type the 6-digit code plus your password, and
   confirm. Sign-in now asks for the code from your phone.
4. Optional: remove `ADMIN_PIN` from `.env` and restart.

Locked out (lost phone or password)? Put a new `ADMIN_PIN` in `.env`, run
`python3 manage.py reset-admin`, restart the app, and start again at step 1.

### Backups

- A compressed copy of the database is made automatically once a day
  (triggered by normal shop traffic), into `data/backups/` (the newest 14 are
  kept). When `BACKUP_EMAIL` is set, each daily copy is also emailed there, so
  a copy exists off the server.
- The admin **Download backup (JSON)** button also starts a full backup.
- Optional extra: cPanel → Cron Jobs, once a day:
  `cd ~/kaser-app && python3 manage.py backup`
- Restore: download a `noor-….sqlite3.gz`, unzip it, stop the app, replace
  `data/noor.sqlite3` with it, start the app.

### Maintenance commands

```bash
python3 manage.py status        # what is configured (no secrets shown)
python3 manage.py backup        # back up now (+ email copy)
python3 manage.py test-email    # send a test message to ADMIN_EMAIL
python3 manage.py reset-admin   # remove the admin account (see above)
```

## Security notes

- Admin sign-in: username + password (PBKDF2-SHA256, 310k rounds) stored in
  the database, optional TOTP 2-step verification with replay protection.
  `ADMIN_PIN` only works until the account exists. Every security change asks
  for the current password and signs out other admin sessions.
- Login lockout (5 failures → 15 minutes) and rate limits are stored in
  SQLite, so they hold across all Passenger worker processes and restarts.
- Customer and admin sessions are random 256-bit tokens stored only as
  SHA-256 digests; customer passwords use PBKDF2 and can be reset by an
  emailed single-use link (valid 1 hour).
- The server enforces stock, size, quantity (1–20 per line), 24-line and
  60-item limits; prices and totals are computed server-side. Order creation
  is transactional and idempotent (a retried request never double-orders).
- Orders, accounts, sessions and reviews live only in SQLite (`data/`) and the
  backups — not in JSON files next to the code.
- Python files, `.env`, `data/`, backups and private JSON are never served
  (allowlist in `noor_app.py` + `.htaccess`).
- Strict CSP (self only, `upgrade-insecure-requests`), `frame-ancestors 'none'`,
  `nosniff`, COOP/CORP, Permissions-Policy (payments off), HSTS.
- Unexpected errors are logged and emailed to `ADMIN_EMAIL` (at most one
  email per 15 minutes).

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
- **Settings**: WhatsApp number and social pages, sign-in & 2-step
  verification, email/backup status, backup download, logout.
- Customers can share a product with `/#/p/<product-id>`; the quick view opens
  directly.

## Data files

| File | Content |
|---|---|
| `data/noor.sqlite3` | The database: orders, products, accounts, sessions, reviews, settings, admin account |
| `data/backups/` | Daily compressed database backups (newest 14) |
| `products.json` | Catalog seed + readable copy of the live catalog |
| `settings.json` | Store settings seed + readable copy (brand, WhatsApp, theme, banner, promos) |
| `shipping.json` | Carrier fees and wilaya zones |
| `images/opt/` | Responsive WebP product variants |

Older versions also wrote `orders.json`, `users.json`, `tokens.json`,
`reviews.json`, `counter.json` and `logo.json` next to the code. On first
start, after their content is in the database, those copies are deleted.

## Customer emails

When email is configured the shop sends:
- the customer an order confirmation (French + Arabic) when they gave an
  email or were signed in, and a message when the order is confirmed,
  shipped, delivered or cancelled;
- you (`ADMIN_EMAIL`) an alert for every new order and for server errors;
- password-reset links for customer accounts.

Payments: cash on delivery only. The Permissions-Policy header keeps the
browser payment APIs switched off until online payment is added.

## Shipping data

`shipping.json` holds per-carrier home/stopdesk tables by wilaya zone. Free
delivery over 8,000 DZD applies to Noor Standard. Figures are editable
estimates — confirm against each carrier's current sheet before launch.
