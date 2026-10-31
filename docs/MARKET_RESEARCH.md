# Market research: what the best finance apps do well

Sources (October 2026): NerdWallet's *Best Budget Apps for 2026*, envelopebudgeting.com's Copilot
review, the Cashew open-source project and store listings, and Android expense-tracker roundups
(getfinny.app, pocketclear.app, geekchamp.com). The roundups are partly vendor-written, so the
table below lists features that come up again and again, not anyone's ranking.

| App | Known for | Idea we take | How Income Radar does it (offline, no bank link) |
|---|---|---|---|
| **YNAB** | Zero-based budgeting: every riyal gets a job | A clear answer to "how much can I still spend" | **Safe to spend per day** and **Today left**, recalculated every minute |
| **Copilot Money** | Design, automatic categories, review flow | The app learns your habits | **Smart suggestions**: one-tap repeats of your usual entries for this time of day |
| **Monarch** | Cash flow, net worth, households | Income vs. spending over time | Stats: cash-flow bars per day, net, savings rate, end-of-cycle forecast |
| **Cashew** | Free, offline, subscriptions, heatmap widget | Spending heatmap, budgets per period | Stats: **7 × 24 hour heatmap** (dot matrix), per-category budgets with warnings |
| **Wallet / Spendee** | Rich reports | Category and weekday breakdowns | Stats: categories, weekday vs. weekend, peak hour, biggest purchases |
| **Money Lover / AndroMoney** | Fast manual entry, offline | Entry in under 5 seconds | Keypad first, recent-entry chips, today/yesterday chips, time captured automatically |
| All of them | Search, edit, transfers | Basics that must exist | Search in Activity, tap to edit, transfers between accounts |

## What users say matters most

1. **Speed of entry.** If logging takes more than about five seconds, people stop logging.
   → Keypad on screen at once, suggestion chips that fill amount + category in one tap.
2. **Privacy and offline use.** Many users avoid bank-linking apps.
   → No network permission at all, the database stays on the phone.
3. **No paywall on basics.** Budgets, reports and export are often locked behind subscriptions.
   → Everything here is free.
4. **Budgets with alerts, recurring entries, export, backup.**
   → Budgets with warnings are in this version. Recurring entries, export and backup are on the
     roadmap (see `SIMULATION_REPORT.md`).

## Features added in this round

* Time of day on every entry (to the minute), so statistics can work by hour and minute.
* **Statistics screen**: money per hour and per minute (income earned, spending burn rate),
  7 × 24 heatmap, spending by hour of day, day-by-day cash flow, category breakdown,
  weekday vs. weekend, averages and medians, biggest purchase, no-spend streaks, time between
  purchases, forecast to the end of the cycle, written insights.
* Smart suggestions on the entry screen, based on your history at this hour.
* Edit and delete entries, search, transfers between accounts.
* Category budgets with progress and over-budget warnings.
