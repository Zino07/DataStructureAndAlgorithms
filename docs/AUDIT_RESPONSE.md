# Audit response: your data, your rules

A reverse-engineering audit of the first APK (v0.1.0, `libincome_radar.so`) listed 25 weaknesses.
This is the status of each one against the code in this repository, checked by tests
(`crates/core/tests/manage.rs`, `decimals.rs`, `app/tests/manage_ui.rs`, `currency_ui.rs`).

Legend: **Fixed** · **Already fine** (the claim was about the old binary, or did not apply) ·
**Partly** · **Not done** (said plainly, with the reason).

## 1. Database and data integrity

| # | Finding | Status | What changed |
|---|---|---|---|
| 1 | Archived accounts counted on some screens, hidden on others | **Fixed** | "Archive" is gone. An account is now **deleted** (its entries move to another account, balance included, or are deleted with it). Old archived accounts come back on upgrade (schema v3). Nothing can be half-hidden any more. |
| 2 | Statistics silently cut at 600 entries | **Already fine** | The `LIMIT 600` was only on the "usual at this time" suggestions (a 60-day window). Statistics read the whole range. The suggestion limit is now 5000. |
| 3 | Whole table pulled into memory | **Already fine** | The activity list is capped in SQL (300 rows); statistics read one date range; only the CSV export reads everything, on purpose. |
| 4 | No indexes on `transfer.from_account / to_account` | **Fixed** | Indexes added in schema v3. |
| 5 | Transfers missing from search / "All" | **Fixed** | "All" lists transfers merged by date, and the search finds them (note, account names, the word "transfer"). |

## 2. Editing and flexibility

| # | Finding | Status | What changed |
|---|---|---|---|
| 6 | Account name / opening balance could not be edited | **Fixed** | Tap an account: rename, change type, correct the opening balance, or delete. |
| 7 | Transfers could not be edited | **Fixed** | Tap a transfer → edit (amount, date, accounts, note) or delete; undo works for both. |
| 8 | Categories fixed, none could be added or changed | **Fixed** | Settings → Categories: add, rename, change icon (26 icons), reorder, hide/show, delete (entries move to another category or go with it). Budgets follow. |
| 9 | No subcategories or tags | **Not done** | A bigger data-model change; categories are flat. |

## 3. Money and accounting

| # | Finding | Status | What changed |
|---|---|---|---|
| 10 | KWD / BHD / OMR / JOD / TND / IQD rejected a third decimal | **Fixed** | Decimals follow the currency: 3 for those six, 0 for JPY and similar, 2 otherwise. Switching between them rescales stored amounts so values are kept (a sub-unit amount never rounds to zero). |
| 11 | Currency is only a label; no per-account currency | **Not done** | Still one currency for the whole app, no conversion. |
| 12 | No overdraft protection | **Partly (by design)** | Negative balances stay allowed (credit cards, overdrafts). Saving the entry that takes an account below zero says so in the confirmation. |
| 13 | Payday limited to days 1–28 | **Fixed** | Days 1–31; 29–31 use the last day of shorter months (tested across leap years). |
| 14 | Shallow "money clock" | **Partly** | Hours per working day are a setting (1–16), and only income from categories marked **paid work** counts (Salary and Freelance by default; switch per category). Working days still follow the weekend setting. |

## 4. Entry and dates

| # | Finding | Status | What changed |
|---|---|---|---|
| 15 | No calendar | **Fixed** | "Pick date" opens a month calendar (back 5 years, ahead 1 year). |
| 16 | Future / planned entries blocked | **Fixed** | Allowed. They show in Activity as **Planned**, and join balances and totals when their day comes. |
| 17 | Keypad without sums or paste | **Partly** | The keypad does `+ − × ÷` with correct precedence and shows the result live. **Paste** is not on the custom keypad (Android's clipboard needs Java APIs); the Note field supports the system paste. |
| 18 | No repeating entries | **Fixed** | Repeat daily / weekly / monthly / yearly when adding. A list (Settings → Repeating entries) to pause or delete. Missed days are created on the next open. Entries only, not transfers. |

## 5. Data loss and privacy

| # | Finding | Status | What changed |
|---|---|---|---|
| 19 | No backup, export or restore | **Partly** | Back up (a complete database file), Restore latest backup, Export CSV (spreadsheet-safe). Files go to the app's folder `Android/data/app.incomeradar/files/`, which a computer or file manager can reach. **Not done:** a share sheet / "save to Downloads" picker, JSON and PDF. |
| 20 | No PIN or biometric lock | **Partly** | PIN lock (4–8 digits, locks again after 20 s away, wait after 5 wrong tries, "Forgot PIN" erases and starts over). **Not done:** fingerprint / face. The database file itself is **not encrypted**. |
| 21 | Sample data pollutes real data | **Fixed** | Sample entries are flagged: **Remove sample data** takes out only them; **Erase everything** starts a brand-new app. |

## 6. Engineering

| # | Finding | Status | What changed |
|---|---|---|---|
| 22 | Debug leftovers (`/data/local/tmp/income-radar`, `INCOME_RADAR_TODAY`) | **Fixed** | The Android build has neither: no fallback path, no test clock. |
| 23 | `unwrap()` panics abort the app | **Mostly fixed** | Startup errors are reported instead of `expect`; no `unwrap` on user-input paths. The release profile still uses `panic = "abort"` (size). |
| 24 | Back gesture, TalkBack, one orientation, one CPU | **Partly** | Back now closes one level at a time (entry page, sub-pages, screens). The APK now carries arm64-v8a and x86_64. **Not done:** 32-bit ARM (armeabi-v7a): the graphics library has no prebuilt binary for it and building one from source was not attempted. **Not verified:** TalkBack on a real phone; landscape is unchanged (portrait only). |
| 25 | No Arabic / RTL | **Not done** | Needs an Arabic font, translated text and mirrored layouts. A separate piece of work. |

## Not verified

Nothing here has run on a real phone: the Android-only behaviour (back gesture, relock after leaving the app,
the backup folder) is by construction and desktop tests, not observed on a device.
