# Simulation report: 100 people × 5 sessions

Raw numbers: [`SIMULATION_RESULTS.md`](SIMULATION_RESULTS.md). Harness: `app/examples/simulate.rs`.

## How it works

Five personas (salaried commuter, freelancer, student, family manager, power user) × 20 people each.
Every person gets 30 days of simulated real life (purchases at realistic times, salary, rent, typos,
forgetting), then opens the app five times over the following days and logs what they remember.
The harness uses the **real UI** (accessibility tree: tap, type, scroll, drag) through Slint's testing
backend, keeps a shadow ledger, and after every session compares each account balance and the home
total with the ledger, and checks statistics sums. In debug mode it checks after every single entry.

Limits, stated plainly: this is a desktop run of the real UI code with scripted behaviour. It is
not 100 humans, it does not run on an Android device, and its timings are desktop CPU timings.

## What it found, and what was fixed

| # | Finding | Type | Fix |
|---|---|---|---|
| 1 | **Save button was at the very bottom of a long entry page**; every save failed in the first runs because the button was off screen | Real UX bug | Save button pinned above the keyboard / navigation bar on a fade; the error message shows in the bar too (the error used to sit at the top of the page, out of sight) |
| 2 | **Memory grew without bound**: a timer held a strong reference to the context that owned it (reference cycle) | Real bug | Weak reference in the timer closures |
| 3 | **Undo toast sat on top of the keypad.** Fixing a typo right after saving, a tap near the keypad's bottom row hit "Undo" and deleted the entry; balances then drifted (112.62 off in one case) | Real UX bug | Toast hidden while the entry screen is open |
| 4 | **Category row was a one-line swipe list of 11 chips.** Far categories needed up to 3 swipes (815 swipe sequences in one run), and in some states the row would not scroll at all | Real design problem | Category picker is a 3-column grid; every category is one tap. Swipes fell from 815 to 106 (accounts and time chips still scroll) |
| 5 | **Entry time defaults to "now"**, so a purchase logged hours later is off by hours (median 365 min off in the first run) | Feature gap | Quick time chips: 1 h ago, 3 h ago, this morning, last night. Median error fell to 230 min. It is still large because the simulated people only adjust the time sometimes, as real people do |
| 6 | "Try sample data" button overflowed on the empty home screen | Layout bug | Stacked layout |
| 7 | Status bar overlapped the title, navigation bar overlapped tiles (reported from a real phone) | Layout bug | Safe-area insets, plus a floor (28 / 16 px) on Android in case the platform reports none, glass top bar when scrolled, bottom fade. **Not yet verified on a phone** |
| 8 | Keypad "0" and "." could be hidden behind the pinned Save bar when the page is not scrolled | Found by harness; the layout is correct at full scroll, so the harness now scrolls like a person | none needed in the app |

Two harness mistakes were fixed along the way (tapping a row by title when several rows share one;
clearing too few digits when editing). They looked like app bugs and were not.

## Final run

* 500 sessions, 3,917 entries, 49,598 taps/inputs
* **0 balance mismatches** between the app and the shadow ledger
* 0 statistics inconsistencies, 0 NaN/inf values, 0 pages wider than the screen
* Tap to updated screen: p50 5 ms, p95 13 ms, max 48 ms (desktop). Statistics for a range: 0.16 ms.
  Opening the app (database + first data): p50 7 ms.
* Database after 5 sessions of heavy use: about 0.9 MB (max 2.1 MB)
* Taps per entry: p50 10, p95 18. 31% of entries start from a suggestion chip, but 95% of those
  amounts are retyped because real amounts vary (see ideas below).

## Still open

1. **15 of about 4,100 save attempts (0.4%, 6 of 100 people) still fail in the harness** with
   "Choose a category" or "Enter an amount". Balances stay correct, so no data is wrong. The
   cause is not found. Chip rows sometimes ignore the harness's drag, and I suspect a Slint
   Flickable edge case; I cannot say whether a finger on a phone would hit it.
2. **Process memory still rises across the run (17 MB → 354 MB over 500 app launches in one process).**
   The harness creates a new window for every session, which a phone never does. One cycle was fixed (#2).
   What remains may be window state kept by the testing backend rather than the app, but I did not
   prove that. Measure on a device before trusting the "low RAM" claim.
3. "Left to spend today" and projections were checked for consistency, not for how helpful they feel.
4. Nothing here tests Arabic text, large font sizes, TalkBack, rotation, or interruptions.

## Features still missing (ranked by what people ask for)

1. **Recurring entries and subscriptions** (rent, salary, Netflix): the biggest gap; 714 simulated purchases
   were "forgotten", and recurring items are the ones people forget least when automated.
2. **Export and backup** (CSV, encrypted file). Today a lost phone means lost data.
3. **Suggestions that learn amounts as ranges**: amount chips are retyped 95% of the time. Offer "usual
   range" or round-number nudges instead.
4. **Reminder to log** in the evening (needs the notification permission; deliberately not added yet).
5. Arabic UI with right-to-left layout; multiple currencies; savings goals; widget.
