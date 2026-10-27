"""Outgoing email for the Noor store — stdlib smtplib only.

Configured from the environment (``.env``):

  SMTP_HOST, SMTP_PORT (465 = SSL, 587 = STARTTLS), SMTP_USER, SMTP_PASSWORD
  MAIL_FROM      sender address, e.g. orders@yourdomain.dz
  ADMIN_EMAIL    where new-order and error alerts go
  SITE_URL       public address used in password-reset links
  MAIL_OUTBOX    optional: write .eml files to this folder instead of sending
                 (local testing; never set it in production)

Every send is plain text (nothing a customer typed is ever rendered as HTML)
and runs on a background thread so a slow mail server never delays a page.
All failures are logged and swallowed: email is a courtesy, orders are not.
"""
import os
import re
import smtplib
import ssl
import sys
import threading
import time
import traceback
from email.message import EmailMessage
from email.utils import formataddr, make_msgid

SMTP_HOST = os.environ.get("SMTP_HOST", "").strip()
SMTP_PORT = int(os.environ.get("SMTP_PORT", "465") or 465)
SMTP_USER = os.environ.get("SMTP_USER", "").strip()
SMTP_PASSWORD = os.environ.get("SMTP_PASSWORD", "")
MAIL_FROM = os.environ.get("MAIL_FROM", "").strip() or SMTP_USER
ADMIN_EMAIL = os.environ.get("ADMIN_EMAIL", "").strip()
SITE_URL = os.environ.get("SITE_URL", "").strip().rstrip("/")
MAIL_OUTBOX = os.environ.get("MAIL_OUTBOX", "").strip()

EMAIL_RE = re.compile(r"^[^@\s<>\"',;]{1,64}@[A-Za-z0-9.-]{1,180}\.[A-Za-z]{2,24}$")


def enabled():
    return bool(MAIL_FROM and (MAIL_OUTBOX or SMTP_HOST))


def valid_email(addr):
    return bool(EMAIL_RE.match(str(addr or "")))


def _deliver(msg):
    if MAIL_OUTBOX:
        os.makedirs(MAIL_OUTBOX, exist_ok=True)
        name = "%d-%s.eml" % (int(time.time() * 1000), re.sub(r"\W", "", msg["To"])[:40])
        with open(os.path.join(MAIL_OUTBOX, name), "wb") as fh:
            fh.write(bytes(msg))
        return
    ctx = ssl.create_default_context()
    if SMTP_PORT == 465:
        server = smtplib.SMTP_SSL(SMTP_HOST, SMTP_PORT, timeout=20, context=ctx)
    else:
        server = smtplib.SMTP(SMTP_HOST, SMTP_PORT, timeout=20)
        server.starttls(context=ctx)
    try:
        if SMTP_USER:
            server.login(SMTP_USER, SMTP_PASSWORD)
        server.send_message(msg)
    finally:
        try:
            server.quit()
        except Exception:
            pass


def send(to, subject, text, attachment=None, sync=False):
    """Queue one plain-text email. ``attachment`` is (filename, bytes, mime)."""
    if not enabled() or not valid_email(to):
        return False
    msg = EmailMessage()
    msg["From"] = formataddr((os.environ.get("MAIL_FROM_NAME", "").strip() or "Noor", MAIL_FROM))
    msg["To"] = to
    msg["Subject"] = re.sub(r"[\r\n]+", " ", str(subject))[:180]
    msg["Message-ID"] = make_msgid(domain=MAIL_FROM.split("@")[-1] or None)
    msg.set_content(text)
    if attachment:
        fname, blob, mime = attachment
        maintype, subtype = mime.split("/", 1)
        msg.add_attachment(blob, maintype=maintype, subtype=subtype, filename=fname)

    def _run():
        try:
            _deliver(msg)
        except Exception as exc:  # never let email break a request
            sys.stderr.write("MAIL: send to %s failed: %s\n" % (to, exc))

    if sync:
        try:
            _deliver(msg)
            return True
        except Exception as exc:
            sys.stderr.write("MAIL: send to %s failed: %s\n" % (to, exc))
            return False
    threading.Thread(target=_run, daemon=True).start()
    return True


# --------------------------------------------------------------------------- #
# Message templates (French + Arabic for customers, English for the owner)
# --------------------------------------------------------------------------- #

def _lines(order):
    out = []
    for line in order.get("lines") or []:
        if isinstance(line, dict):
            size = (" — %s" % line.get("size")) if line.get("size") else ""
            out.append("  • %s%s × %s" % (line.get("name", ""), size, line.get("qty", 1)))
    return "\n".join(out)


def order_confirmation(order, shop):
    to = order.get("email", "")
    num = order.get("num", "")
    text = (
        "Bonjour %(name)s,\n\n"
        "Merci pour votre commande %(num)s chez %(shop)s.\n"
        "Nous vous appellerons au %(phone)s pour confirmer la livraison.\n\n"
        "%(lines)s\n\n"
        "Total : %(total)s — paiement à la livraison.\n\n"
        "—\n\n"
        "مرحباً %(name)s،\n"
        "شكراً على طلبك %(num)s من %(shop)s.\n"
        "سنتصل بك على الرقم %(phone)s لتأكيد التوصيل.\n"
        "المجموع: %(total)s — الدفع عند الاستلام.\n"
    ) % {"name": order.get("name", ""), "num": num, "shop": shop,
         "phone": order.get("phone", ""), "lines": _lines(order),
         "total": order.get("total", "")}
    return send(to, "%s — commande %s / طلب %s" % (shop, num, num), text)


STATUS_TEXT = {
    "confirmed": ("Votre commande %s est confirmée.", "تم تأكيد طلبك %s."),
    "shipped": ("Votre commande %s a été expédiée.", "تم شحن طلبك %s."),
    "delivered": ("Votre commande %s a été livrée. Merci !", "تم تسليم طلبك %s. شكراً!"),
    "cancelled": ("Votre commande %s a été annulée.", "تم إلغاء طلبك %s."),
}


def order_status(order, status, shop):
    if status not in STATUS_TEXT:
        return False
    num = order.get("num", "")
    fr, ar = STATUS_TEXT[status]
    text = "Bonjour %s,\n\n%s\n\n—\n\n%s\n\n%s\n" % (order.get("name", ""), fr % num, ar % num, shop)
    return send(order.get("email", ""), "%s — %s" % (shop, fr % num), text)


def admin_new_order(order, shop):
    if not ADMIN_EMAIL:
        return False
    text = (
        "New order %(num)s — %(total)s\n\n"
        "Customer: %(name)s\nPhone: %(phone)s\nWilaya: %(wilaya)s, %(commune)s\n"
        "Delivery: %(provider)s / %(method)s\n\n%(lines)s\n\n"
        "Open the admin: %(url)s/#admin\n"
    ) % {"num": order.get("num", ""), "total": order.get("total", ""),
         "name": order.get("name", ""), "phone": order.get("phone", ""),
         "wilaya": order.get("wilaya", ""), "commune": order.get("commune", ""),
         "provider": order.get("provider", ""), "method": order.get("method", ""),
         "lines": _lines(order), "url": SITE_URL}
    return send(ADMIN_EMAIL, "[%s] New order %s — %s" % (shop, order.get("num", ""), order.get("total", "")), text)


def password_reset(to, link, shop):
    text = (
        "Bonjour,\n\nPour choisir un nouveau mot de passe sur %(shop)s, ouvrez ce lien "
        "(valable 1 heure) :\n\n%(link)s\n\nSi vous n'avez rien demandé, ignorez ce message.\n\n"
        "—\n\nلإعادة تعيين كلمة المرور في %(shop)s افتح هذا الرابط (صالح لمدة ساعة):\n\n%(link)s\n\n"
        "إذا لم تطلب ذلك، تجاهل هذه الرسالة.\n"
    ) % {"shop": shop, "link": link}
    return send(to, "%s — mot de passe / كلمة المرور" % shop, text)


def error_alert(where, exc_info=None):
    """Email the owner about an unexpected server error (caller throttles)."""
    if not ADMIN_EMAIL:
        return False
    tb = "".join(traceback.format_exception(*exc_info)) if exc_info else ""
    text = "An unexpected error happened on %s\n\nWhere: %s\nTime (UTC): %s\n\n%s\n" % (
        SITE_URL or "the store", where, time.strftime("%Y-%m-%d %H:%M:%S", time.gmtime()), tb[-6000:])
    return send(ADMIN_EMAIL, "[Store error] %s" % where[:80], text)
