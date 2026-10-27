#!/usr/bin/env python3
"""Owner maintenance commands for the Noor store.

Run from the application folder (cPanel -> Terminal, or a cPanel cron job):

  python3 manage.py backup         Back up the database now (and email a copy
                                   to BACKUP_EMAIL when it is set).
  python3 manage.py test-email     Send a test message to ADMIN_EMAIL.
  python3 manage.py reset-admin    Locked out? Remove the admin account and
                                   2-step verification. Sign-in then uses the
                                   ADMIN_PIN from .env again, so set a new PIN
                                   first and create a new account right after.
  python3 manage.py status         Show what is configured (no secrets).
"""
import os
import sys

ROOT = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, ROOT)

import noor_app as app  # noqa: E402  (loads .env, store and mailer)


def main(argv):
    cmd = argv[1] if len(argv) > 1 else ""
    if cmd == "backup":
        path = app._run_backup(send_copy=True)
        if not path:
            print("Backup FAILED - see the message above.")
            return 1
        print("Backup written: %s" % path)
        if app.BACKUP_EMAIL:
            print("A copy is being emailed to the BACKUP_EMAIL address.")
            import time
            time.sleep(8)  # let the background email thread finish
        return 0
    if cmd == "test-email":
        if not app.mailer.enabled() or not app.mailer.ADMIN_EMAIL:
            print("Email is not configured: set SMTP_HOST, SMTP_USER, SMTP_PASSWORD, "
                  "MAIL_FROM and ADMIN_EMAIL in .env")
            return 1
        ok = app.mailer.send(app.mailer.ADMIN_EMAIL, "Test email from your store",
                             "Email works. Order alerts and customer emails will be sent.\n",
                             sync=True)
        print("Sent." if ok else "Sending FAILED - check the SMTP settings.")
        return 0 if ok else 1
    if cmd == "reset-admin":
        answer = input("Remove the admin account and 2-step verification? Type YES: ")
        if answer.strip() != "YES":
            print("Nothing changed.")
            return 1
        app.save_json(app.ADMIN_FILE, {})
        app.revoke_sessions(admin=True)
        print("Admin account removed. Sign in with the ADMIN_PIN from .env, then create "
              "a new account in Admin -> Settings -> Sign-in & security.")
        if not app.PIN_USABLE:
            print("WARNING: ADMIN_PIN in .env is missing or too weak, so sign-in is disabled "
                  "until you set one (8+ characters) and restart the app.")
        return 0
    if cmd == "status":
        acct = app.admin_account()
        print("Admin account:      %s" % ("yes (%s)" % acct.get("user") if app.admin_has_credentials(acct) else "not created yet"))
        print("2-step verification: %s" % ("on" if acct.get("totp") else "off"))
        print("Bootstrap PIN:      %s" % ("usable" if app.PIN_USABLE else "not set"))
        print("Email:              %s" % ("configured" if app.mailer.enabled() else "not configured"))
        print("Admin alerts to:    %s" % (app.mailer.ADMIN_EMAIL or "-"))
        print("Backups emailed to: %s" % (app.BACKUP_EMAIL or "-"))
        print("Database:           %s" % app._store.DB_PATH)
        print("Health:             %s" % ("ok" if app._store.health().healthy else "PROBLEM"))
        return 0
    print(__doc__)
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
