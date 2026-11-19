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
        "sheets": [{"material": "Birch plywood", "thickness": 18.0, "area_m2": 2.53, "sheets_2440x1220": 2.0}],
        "hardware": [{"item": "Hinge, 35 mm cup", "qty": 4}],
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
    let pdf = render(&list(3), "Kitchen (draft)");
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
        b"(Hinge, 35 mm cup)",
        b"(4)",
    ];
    for word in words {
        assert!(has(word), "{}", String::from_utf8_lossy(word));
    }
}

#[test]
fn a_long_list_runs_onto_more_pages() {
    let pdf = render(&list(60), "Kitchen");
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
}
