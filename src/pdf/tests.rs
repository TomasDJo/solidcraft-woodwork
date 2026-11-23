use super::*;
use serde_json::json;

fn list(rows: usize) -> Value {
    let part = |i: usize| {
        json!({"qty": 2, "length": 720.0, "width": 560.0 - i as f64, "thickness": 18.0,
               "material": "Birch plywood", "parts": [format!("Cabinet Side ({i})"), "Bjørk"]})
    };
    json!({
        "units": "mm", "boards": rows * 2, "area_m2": 2.94,
        "parts": (0..rows).map(part).collect::<Vec<_>>(),
        "sheets": [{"material": "Birch plywood", "thickness": 18.0, "area_m2": 2.53, "sheets": 2, "yield_pct": 41.5}],
        "sheet_mm": [2440.0, 1220.0], "kerf_mm": 4.0, "trim_mm": 10.0, "layouts": [], "oversize": [],
        "hardware": [{"item": "Hinge, cup", "qty": 4}],
    })
}

/// The byte offsets in the xref table point at the objects they name, and startxref at xref.
fn well_formed(pdf: &[u8]) {
    let text = String::from_utf8_lossy(pdf);
    assert!(text.starts_with("%PDF-1.4") && text.ends_with("%%EOF\n"));
    let xref: usize = text
        .rsplit("startxref\n")
        .next()
        .unwrap()
        .lines()
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert!(pdf[xref..].starts_with(b"xref\n"));
    let table = String::from_utf8_lossy(&pdf[xref..]);
    let entries: Vec<&str> = (table.lines().skip(3))
        .take_while(|l| l.ends_with(" n "))
        .collect();
    assert!(entries.len() >= 6, "{} objects", entries.len());
    for (i, entry) in entries.iter().enumerate() {
        let at: usize = entry[..10].parse().unwrap();
        assert!(
            pdf[at..].starts_with(format!("{} 0 obj", i + 1).as_bytes()),
            "object {}",
            i + 1
        );
    }
}

#[test]
fn a_cut_list_as_pdf() {
    let pdf = render(&list(3), "Kitchen (draft)", true);
    well_formed(&pdf);
    let has = |bytes: &[u8]| pdf.windows(bytes.len()).any(|w| w == bytes);
    assert!(has(b"/Count 1 "), "one page");
    // brackets escaped; the dash and ø in WinAnsi
    assert!(has(b"(Kitchen \\(draft\\) \x97 Cut list)"));
    let words: [&[u8]; 6] = [
        b"(Birch plywood \x97 18 mm)",
        b"(720)",
        b"(558)",
        b"Bj\xf8rk",
        b"(Hinge, cup)",
        b"(4)",
    ];
    for word in words {
        assert!(has(word), "{}", String::from_utf8_lossy(word));
    }
}

#[test]
fn a_long_list_runs_onto_more_pages() {
    let pdf = render(&list(60), "Kitchen", true);
    well_formed(&pdf);
    let text = String::from_utf8_lossy(&pdf);
    let pages = text.matches("/Type /Page ").count();
    assert!(pages >= 2, "{pages} pages");
    assert!(text.contains(&format!("/Count {pages} ")));
}

#[test]
fn long_names_wrap_at_spaces() {
    assert_eq!(wrap("Side L, Side R", 9.0, 1000.0), ["Side L, Side R"]);
    assert_eq!(wrap("Side L, Side R", 9.0, 40.0), ["Side L,", "Side R"]);
    // a word longer than the room stays whole on its own line
    assert_eq!(wrap("Countertop x", 9.0, 10.0), ["Countertop", "x"]);
    assert!((width("0123456789", 10.0) - 55.6).abs() < 1e-9);
    assert!((width("— ×", 10.0) - 18.62).abs() < 1e-9); // a dash an em wide
}

#[test]
fn banding_and_grain_marks() {
    let mut banded = list(1);
    banded["parts"][0]["grain"] = json!(true);
    banded["parts"][0]["banding"] = json!({"length": 2, "width": 1});
    banded["size"] = json!("cut");
    banded["banding_mm"] = json!(1.0);
    banded["banding"] = json!([{"material": "Birch plywood", "thickness": 18.0, "metres": 3.63}]);
    let pdf = render(&banded, "Kitchen", true);
    well_formed(&pdf);
    let has = |bytes: &[u8]| pdf.windows(bytes.len()).any(|w| w == bytes);
    let words: [&[u8]; 6] = [
        b"(L2 W1)",
        b"(yes)",
        b"cut sizes: finished less the 1 mm banding",
        b"(Edge banding to buy)",
        b"(\\(1 mm, plus 10 % waste\\))",
        b"(3.7)", // 3.63 rounded up
    ];
    for word in words {
        assert!(has(word), "{}", String::from_utf8_lossy(word));
    }
    // without banding: no section, finished sizes
    let pdf = render(&list(1), "Kitchen", true);
    let has = |bytes: &[u8]| pdf.windows(bytes.len()).any(|w| w == bytes);
    assert!(!has(b"Edge banding") && has(b"finished sizes"));
}

#[test]
fn cutting_diagrams_follow_the_field() {
    let mut listed = list(2);
    let part = |name: &str, row: usize, x: f64, w: f64, h: f64| json!({"part": name, "row": row, "x": x, "y": 10.0, "w": w, "h": h, "rotated": false});
    let sheet = |parts: Vec<Value>| {
        json!({"material": "Birch plywood", "thickness": 18.0, "size": [2440.0, 1220.0],
               "yield_pct": 30.2, "parts": parts})
    };
    listed["layouts"] = json!([
        sheet(vec![
            // both rows have a Bjørk: the label goes by the row, not the name
            part("Bjørk", 0, 10.0, 720.0, 560.0),
            part("Cabinet Side (1)", 1, 734.0, 720.0, 50.0)
        ]),
        sheet(vec![
            part("Bjørk", 1, 10.0, 720.0, 559.0),
            part("Cabinet Side (0)", 0, 734.0, 20.0, 20.0)
        ]),
    ]);
    listed["oversize"] = json!([{"part": "Long", "length": 3000.0, "width": 600.0,
                                 "reason": "bigger than the sheet less its trim"}]);
    listed["worktops"] = json!([{"part": "Kitchen Worktop", "material": "Oak", "thickness": 38.0,
                                 "length": 3300.0, "width": 600.0}]);
    // the worktop's row: a table of its own though no sheet is bought for it
    listed["parts"]
        .as_array_mut()
        .unwrap()
        .push(json!({"qty": 1, "length": 3300.0,
        "width": 600.0, "thickness": 38.0, "material": "Oak", "parts": ["Kitchen Worktop"]}));
    let without = render(&listed, "Kitchen", false);
    let with = render(&listed, "Kitchen", true);
    well_formed(&with);
    let has = |pdf: &[u8], bytes: &[u8]| pdf.windows(bytes.len()).any(|w| w == bytes);
    let pages = |pdf: &[u8]| String::from_utf8_lossy(pdf).matches("/Type /Page ").count();
    assert_eq!((pages(&without), pages(&with)), (1, 2));
    assert!(!has(&without, b"Cutting diagrams") && has(&with, b"(Cutting diagrams)"));
    let words: [&[u8]; 8] = [
        b"sheet 1 of 2",
        b"sheet 2 of 2",
        b"(2440 \xd7 1220 mm, yield 30.2 %)",
        b"(1)",            // the first row's number, in the legend and on its part
        b"(720 \xd7 560)", // its size on the part
        b"/F2 7.5 Tf",     // a narrow part gets number and size on one line
        b"0.78 g",         // offcuts
        b"[2 2] 0 d",      // the trim
    ];
    for word in words {
        assert!(has(&with, word), "{}", String::from_utf8_lossy(word));
    }
    // the sheets note on both, with the part too big for a sheet and why; the worktops
    for pdf in [&with, &without] {
        assert!(has(pdf, b"4 mm saw kerf"));
        assert!(has(
            pdf,
            b"Long \\(3000 \xd7 600 mm\\): bigger than the sheet less its trim"
        ));
        assert!(has(pdf, b"(Worktops, bought to length)") && has(pdf, b"(Kitchen Worktop)"));
        assert!(has(pdf, b"(Oak \x97 38 mm)"));
        assert!(has(pdf, b"(2)") && has(pdf, b"(SHEETS 2440 \xd7 1220)"));
    }
}

/// A cut list of `boards` boards in one row, all laid out on one sheet but the last.
fn boards(boards: usize) -> Value {
    let names: Vec<String> = (0..boards).map(|i| format!("Base Shelf {i}")).collect();
    let placed: Vec<Value> = (names[..boards - 1].iter())
        .map(|n| json!({"part": n, "row": 0, "x": 10.0, "y": 10.0, "w": 500.0, "h": 300.0}))
        .collect();
    json!({
        "units": "mm",
        "parts": [{"qty": boards, "length": 500.0, "width": 300.0, "thickness": 18.0,
                   "material": "Oak", "grain": true, "banding": {"length": 1, "width": 0},
                   "parts": names}],
        "sheets": [{"material": "Oak", "thickness": 18.0, "sheets": 1}],
        "layouts": [{"parts": placed}],
    })
}

#[test]
fn a_label_per_board_24_a_page() {
    let pdf = labels::render(&boards(30));
    well_formed(&pdf);
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.contains("/Count 2 "), "two pages");
    assert_eq!(text.matches("(Base Shelf ").count(), 30);
    assert_eq!(text.matches("(#1, sheet 1 of 1)").count(), 29);
    // the one on no sheet comes last
    let last = text.find("(Base Shelf 29)").unwrap();
    assert!(text.rfind("(not on a sheet)").unwrap() > last);
    assert!(text.contains("(500 \u{fffd} 300 \u{fffd} 18 mm)")); // × in WinAnsi
    assert!(
        text.contains("2.4 w") && text.contains("(edges L1)"),
        "the banded edge"
    );
    assert!(text.contains(" l f\n"), "the grain arrow's head");
    assert_eq!(labels::render(&json!({})), assemble(&[Vec::new()]));
}

#[test]
fn labels_escape_and_shorten_names() {
    let mut list = boards(2);
    list["parts"][0]["parts"] = json!(["Side (L)", "A very long cabinet name Drawer 3 Front"]);
    list["parts"][0]["banding"] = json!({"length": 0, "width": 0});
    list["layouts"] = json!([]);
    list["worktops"] = json!([{"part": "Side (L)"}]);
    let pdf = labels::render(&list);
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.contains(r"(Side \(L\))") && text.contains("(bought to length)"));
    assert!(!text.contains("2.4 w"), "no banded edge");
    let (size, cut) = labels::fit(&"Drawer ".repeat(10), 11.0, 174.0);
    assert!(size == 7.0 && cut.ends_with("...") && width(&cut, 7.0) * 1.06 <= 174.0);
    assert_eq!(labels::fit("Side L", 11.0, 174.0), (11.0, "Side L".into()));
}

#[test]
fn a_notched_side_says_so() {
    let mut cut = list(1);
    cut["parts"][0]["note"] = json!("notched");
    let pdf = render(&cut, "Kitchen", false);
    assert!(String::from_utf8_lossy(&pdf).contains("notched"));
    assert!(!String::from_utf8_lossy(&render(&list(1), "Kitchen", false)).contains("notched"));
    let mut boards = boards(2);
    boards["parts"][0]["note"] = json!("notched");
    let text = String::from_utf8_lossy(&labels::render(&boards)).to_string();
    assert!(text.contains("(#1, sheet 1 of 1, notched)"), "{text}");
}

#[test]
fn an_l_board_is_drawn_as_an_l_in_the_diagram() {
    let mut listed = list(1);
    listed["layouts"] = json!([{"material": "Birch plywood", "thickness": 18.0, "size": [2440.0, 1220.0],
        "yield_pct": 30.0, "parts": [{"part": "C Bottom", "row": 0, "x": 10.0, "y": 10.0,
        "w": 864.0, "h": 864.0, "rotated": false, "corner": [322.0, 322.0]}]}]);
    let pdf = render(&listed, "Corner", true);
    well_formed(&pdf);
    let text = String::from_utf8_lossy(&pdf);
    // a six-point path, not a rectangle
    let path = text
        .lines()
        .find(|l| l.contains(" m ") && l.ends_with(" h B"))
        .expect("an L path");
    assert_eq!(path.matches(" l ").count(), 5, "{path}");
}
