//! Vector data for category and account icons (24 × 24 grid, stroked).

pub fn path(key: &str) -> &'static str {
    match key {
        "food" => "M7 3v8M5 3v5a2 2 0 0 0 4 0V3M7 11v10M17 21V3c-2 0-3 2-3 6v4h3",
        "car" => "M5 17h14v-5l-2-5H7l-2 5zM5 17v2M19 17v2M5 12h14M8 14.5h.01M16 14.5h.01",
        "home" => "M4 11l8-7 8 7v9H4zM10 20v-6h4v6",
        "bolt" => "M13 3 5 14h6l-1 7 8-11h-6z",
        "bag" => "M6 8h12l-1 12H7zM9 8a3 3 0 0 1 6 0",
        "heart" => "M12 20s-8-5-8-10.5A4.5 4.5 0 0 1 12 7a4.5 4.5 0 0 1 8 2.5C20 15 12 20 12 20z",
        "play" => "M6 4h12a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2zM10 9l5 3-5 3z",
        "book" => "M4 5a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v15a2 2 0 0 0-2-2H4zM20 5a1 1 0 0 0-1-1h-6a1 1 0 0 0-1 1v15a2 2 0 0 1 2-2h6z",
        "briefcase" => "M4 8h16v11H4zM9 8V5h6v3M4 13h16",
        "laptop" => "M5 6h14v9H5zM3 18h18",
        "trend" => "M3 17l6-6 4 4 8-8M15 7h6v6",
        "gift" => "M4 11h16v9H4zM3 8h18v3H3zM12 8v12M12 8C10 4 6 5 8 8M12 8c2-4 6-3 4 0",
        "cash" => "M3 7h18v10H3zM12 12m-2 0a2 2 0 1 0 4 0a2 2 0 1 0-4 0M6 10v4M18 10v4",
        "bank" => "M3 10h18L12 4zM5 10v8M9.5 10v8M14.5 10v8M19 10v8M3 20h18",
        "card" => "M3 6h18v12H3zM3 10h18M7 15h4",
        "savings" => "M12 12m-8 0a8 8 0 1 0 16 0a8 8 0 1 0-16 0M12 12m-3.5 0a3.5 3.5 0 1 0 7 0a3.5 3.5 0 1 0-7 0",
        _ => "M7 12h.01M12 12h.01M17 12h.01",
    }
}
