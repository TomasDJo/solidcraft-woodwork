//! Part labels: a sticker per board, to stick on after cutting. A4 sheets of 3 x 8 labels of
//! 70 x 37 mm without margins (Herma or Avery 3474), in the order of the cutting diagrams.

use super::{assemble, shown, width, Doc, GREY, H, INK, RULE, W};
use serde_json::Value;

const COLS: usize = 3;
const ROWS: usize = 8;
const LW: f64 = W / COLS as f64;
const LH: f64 = H / ROWS as f64;
const PAD: f64 = 12.0; // inside each label: most printers cannot print to the paper's edge

/// `text` at the biggest size from `max` down to 7 pt that fits `room`, cut short with `...`
/// when even 7 pt does not.
pub(super) fn fit(text: &str, max: f64, room: f64) -> (f64, String) {
    let wide = |t: &str, size: f64| width(t, size) * 1.06; // bold, as in `Doc::heading`
    let mut size = max;
    while size > 7.0 && wide(text, size) > room {
        size -= 0.5;
    }
    let mut text = text.to_string();
    if wide(&text, size) > room {
        while !text.is_empty() && wide(&format!("{text}..."), size) > room {
            text.pop();
        }
        text = format!("{}...", text.trim_end());
    }
    (size, text)
}

/// One board to label: its name, its row in `parts`, and where it is cut ("#3, sheet 1 of 2").
type Label<'a> = (&'a str, &'a Value, String);

/// The labels PDF of a cut list, `cutlist::cutlist`'s reply: the boards on the cutting layouts
/// sheet by sheet, then those on none (worktops, parts too big for a sheet).
pub fn render(list: &Value) -> Vec<u8> {
    let none = vec![];
    let parts = list["parts"].as_array().unwrap_or(&none);
    let layouts = list["layouts"].as_array().unwrap_or(&none);
    let mut labels: Vec<Label> = Vec::new();
    let mut next = 0;
    for group in list["sheets"].as_array().unwrap_or(&none) {
        let n = group["sheets"].as_u64().unwrap_or(0) as usize;
        for k in 0..n {
            let Some(sheet) = layouts.get(next + k) else {
                break;
            };
            for p in sheet["parts"].as_array().unwrap_or(&none) {
                let Some(i) = p["row"].as_u64().map(|i| i as usize) else {
                    continue;
                };
                if let (Some(row), Some(name)) = (parts.get(i), p["part"].as_str()) {
                    labels.push((name, row, format!("#{}, sheet {} of {n}", i + 1, k + 1)));
                }
            }
        }
        next += n;
    }
    let worktops = list["worktops"].as_array().unwrap_or(&none);
    for row in parts {
        for name in row["parts"].as_array().unwrap_or(&none) {
            let Some(name) = name.as_str() else { continue };
            if labels.iter().any(|l| l.0 == name) {
                continue;
            }
            let bought = worktops.iter().any(|w| w["part"] == name);
            let note = if bought {
                "bought to length"
            } else {
                "not on a sheet"
            };
            labels.push((name, row, note.into()));
        }
    }

    let units = list["units"].as_str().unwrap_or("mm");
    let mut doc = Doc {
        pages: vec![],
        y: 0.0,
    };
    for (i, (name, row, cut)) in labels.iter().enumerate() {
        let at = i % (COLS * ROWS);
        if at == 0 {
            doc.pages.push(Vec::new());
            guides(&mut doc);
        }
        let x = (at % COLS) as f64 * LW + PAD;
        let top = H - (at / COLS) as f64 * LH - PAD;
        let room = LW - 2.0 * PAD;
        let (size, name) = fit(name, 11.0, room);
        doc.text(x, top - 9.0, size, true, INK, &name);
        let dims = format!(
            "{} × {} × {} {units}",
            shown(&row["length"]),
            shown(&row["width"]),
            shown(&row["thickness"])
        );
        let (size, dims) = fit(&dims, 10.0, room);
        doc.text(x, top - 24.0, size, true, INK, &dims);
        let material = match row["material"].as_str().unwrap_or("") {
            "" => "No material",
            m => m,
        };
        let (size, material) = fit(material, 8.5, room);
        doc.text(x, top - 36.0, size, false, INK, &material);
        let bottom = top - LH + 2.0 * PAD;
        doc.text(x, bottom, 8.5, false, GREY, cut);
        part(&mut doc, x, bottom, room, row);
    }
    if doc.pages.is_empty() {
        doc.pages.push(Vec::new());
    }
    assemble(&doc.pages)
}

/// Thin lines where the sheet is cut into labels.
fn guides(doc: &mut Doc) {
    let mut ops = format!("{RULE} G 0.3 w ");
    for c in 1..COLS {
        let x = c as f64 * LW;
        ops += &format!("{x:.2} 0 m {x:.2} {H} l ");
    }
    for r in 1..ROWS {
        let y = r as f64 * LH;
        ops += &format!("0 {y:.2} m {W} {y:.2} l ");
    }
    doc.raw(&(ops + "S\n"));
}

/// The part as a small rectangle in a 62 x 32 pt box at the right end of the `room` from (`x`,
/// `y`), length across: its banded edges as thick lines (the first along the length at the
/// bottom, the first along the width on the left) and named at the left (`edges L1`), and an
/// arrow along the length when it has a grain.
fn part(doc: &mut Doc, x: f64, y: f64, room: f64, row: &Value) {
    let (bw, bh) = (62.0, 32.0);
    let edges = |key: &str| row["banding"][key].as_u64().unwrap_or(0);
    let (along, across) = (edges("length"), edges("width"));
    let banded = crate::cutlist::mark(along, across);
    if !banded.is_empty() {
        doc.text(x, y + 11.0, 7.5, false, GREY, &format!("edges {banded}"));
    }
    // ponytail: a size in inches is text ("23 1/4"): drawn half as wide as long
    let aspect = match (row["length"].as_f64(), row["width"].as_f64()) {
        (Some(l), Some(w)) if l > 0.0 => w / l,
        _ => 0.5,
    };
    let (w, h) = if aspect > bh / bw {
        (bh / aspect.min(2.5), bh) // a part wider than long: no narrower than this
    } else {
        (bw, (bw * aspect).max(7.0))
    };
    let x = x + room - w;
    doc.raw(&format!(
        "{GREY} G 0.6 w {x:.2} {y:.2} {w:.2} {h:.2} re S\n"
    ));
    let mut thick = String::new();
    for (n, line) in [
        (along >= 1, [x, y, x + w, y]),
        (along >= 2, [x, y + h, x + w, y + h]),
        (across >= 1, [x, y, x, y + h]),
        (across >= 2, [x + w, y, x + w, y + h]),
    ] {
        if n {
            let [a, b, c, d] = line;
            thick += &format!("{a:.2} {b:.2} m {c:.2} {d:.2} l ");
        }
    }
    if !thick.is_empty() {
        doc.raw(&format!("{INK} G 2.4 w 2 J {thick}S 0 J\n"));
    }
    if row["grain"] == true {
        let inset = (w * 0.15).min(6.0);
        let (cy, a, b) = (y + h / 2.0, x + inset, x + w - inset);
        doc.raw(&format!(
            "{INK} G {INK} g 0.8 w {a:.2} {cy:.2} m {:.2} {cy:.2} l S \
             {b:.2} {cy:.2} m {:.2} {:.2} l {:.2} {:.2} l f\n",
            b - 4.0,
            b - 5.0,
            cy + 2.5,
            b - 5.0,
            cy - 2.5
        ));
    }
}
