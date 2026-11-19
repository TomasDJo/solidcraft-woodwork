//! The cut list as a printable PDF, written by hand with the PDF viewer's own Helvetica: no
//! dependency and no embedded font. A4, one table per material and thickness, tick boxes to
//! mark what is cut and bought.

use serde_json::Value;

const W: f64 = 595.0; // A4 in points
const H: f64 = 842.0;
const M: f64 = 48.0; // margin

/// Helvetica's widths in 1/1000 em, for the characters space to `~`. Helvetica-Bold's digits
/// have the same widths, so a bold number lines up right as well.
const WIDTHS: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278,
    278, // space to /
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, // 0 to 9
    278, 278, 584, 584, 584, 556, 1015, // : to @
    667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, // A to M
    722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, // N to Z
    278, 278, 278, 469, 556, 333, // [ to `
    556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, // a to m
    556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, // n to z
    334, 260, 334, 584, // { to ~
];

/// A character in WinAnsi, the PDF's standard 8-bit encoding: Latin-1 as it is (so æ, ø, å,
/// ×, ²), plus dashes and quotes; anything else becomes `?`.
fn byte(c: char) -> u8 {
    match c {
        '—' => 0x97,
        '–' => 0x96,
        '‘' => 0x91,
        '’' => 0x92,
        '“' => 0x93,
        '”' => 0x94,
        '€' => 0x80,
        ' '..='~' | '\u{a0}'..='\u{ff}' => c as u8,
        _ => b'?',
    }
}

fn width(text: &str, size: f64) -> f64 {
    let em = |c: char| match c {
        ' '..='~' => WIDTHS[c as usize - 32] as f64,
        _ => 556.0, // ponytail: an average for letters outside ASCII
    };
    text.chars().map(em).sum::<f64>() * size / 1000.0
}

/// `text` in lines no wider than `room`, broken at spaces.
fn wrap(text: &str, size: f64, room: f64) -> Vec<String> {
    let mut lines = vec![String::new()];
    for word in text.split(' ') {
        let line = lines.last_mut().unwrap();
        let longer = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if !line.is_empty() && width(&longer, size) > room {
            lines.push(word.into());
        } else {
            *line = longer;
        }
    }
    lines
}

const INK: f64 = 0.12;
const GREY: f64 = 0.42;
const RULE: f64 = 0.82;
const SHADE: f64 = 0.955;

/// The pages' drawing operators, and how far down the current page is filled.
struct Doc {
    pages: Vec<Vec<u8>>,
    y: f64,
}

impl Doc {
    fn raw(&mut self, ops: &str) {
        self.pages.last_mut().unwrap().extend(ops.bytes());
    }

    /// Room for `h` more points on this page, else a new page.
    fn room(&mut self, h: f64) {
        if self.y - h < M {
            self.pages.push(Vec::new());
            self.y = H - M;
        }
    }

    fn text(&mut self, x: f64, y: f64, size: f64, bold: bool, grey: f64, text: &str) {
        let font = if bold { 2 } else { 1 };
        self.raw(&format!(
            "BT /F{font} {size} Tf {grey} g {x:.1} {y:.1} Td ("
        ));
        let ops = self.pages.last_mut().unwrap();
        for b in text.chars().map(byte) {
            if matches!(b, b'(' | b')' | b'\\') {
                ops.push(b'\\');
            }
            ops.push(b);
        }
        self.raw(") Tj ET\n");
    }

    /// A rule across the page at `y`.
    fn rule(&mut self, y: f64, weight: f64, grey: f64) {
        self.raw(&format!(
            "{grey} G {weight} w {M} {y:.1} m {} {y:.1} l S\n",
            W - M
        ));
    }

    /// A rectangle, filled with `fill` or else outlined.
    fn rect(&mut self, x: f64, y: f64, w: f64, h: f64, fill: Option<f64>) {
        self.raw(&match fill {
            Some(g) => format!("{g} g {x:.1} {y:.1} {w:.1} {h:.1} re f\n"),
            None => format!("{GREY} G 0.7 w {x:.1} {y:.1} {w:.1} {h:.1} re S\n"),
        });
    }

    fn heading(&mut self, text: &str, note: &str) {
        self.room(90.0); // the heading, its table's titles and a first row
        self.y -= 26.0;
        self.text(M, self.y, 12.0, true, INK, text);
        // ponytail: bold is taken as 6 % wider than regular, close enough for a gap
        let x = M + width(text, 12.0) * 1.06 + 5.0;
        self.text(x, self.y, 8.5, false, GREY, note);
        self.y -= 12.0;
    }

    /// A table: columns of (title, x, right-aligned at x, bold). A left-aligned last column
    /// wraps; with
    /// `boxes` the first one is a tick box.
    fn table(&mut self, cols: &[(&str, f64, bool, bool)], rows: &[Vec<String>], boxes: bool) {
        self.room(40.0);
        self.y -= 12.0;
        for &(title, x, right, _) in cols {
            let x = if right { x - width(title, 7.0) } else { x };
            self.text(x, self.y, 7.0, true, GREY, title);
        }
        self.y -= 5.0;
        self.rule(self.y, 0.8, INK);
        let last = cols.len() - 1;
        let room = W - M - cols[last].1;
        for (i, row) in rows.iter().enumerate() {
            let lines = match cols[last].2 {
                true => vec![row[last].clone()],
                false => wrap(&row[last], 9.0, room),
            };
            let h = 18.0 + 11.0 * (lines.len() - 1) as f64;
            self.room(h);
            if i % 2 == 1 {
                self.rect(M, self.y - h, W - 2.0 * M, h, Some(SHADE));
            }
            let base = self.y - 12.5;
            for (j, &(_, x, right, bold)) in cols.iter().enumerate() {
                if boxes && j == 0 {
                    self.rect(x, base - 1.0, 8.0, 8.0, None);
                } else if j == last && !right {
                    for (k, line) in lines.iter().enumerate() {
                        self.text(x, base - 11.0 * k as f64, 9.0, bold, INK, line);
                    }
                } else {
                    let x = if right { x - width(&row[j], 9.0) } else { x };
                    self.text(x, base, 9.0, bold, INK, &row[j]);
                }
            }
            self.y -= h;
            self.rule(self.y, 0.3, RULE);
        }
    }
}

/// A number as the cut list shows it: `702`, `18.5`, or inches as text such as `23 1/4`.
fn shown(v: &Value) -> String {
    v.as_f64()
        .map_or_else(|| v.as_str().unwrap_or("").into(), |n| n.to_string())
}

/// The PDF of a cut list, `cutlist::cutlist`'s reply with its `hardware`. `title` heads it.
pub fn render(list: &Value, title: &str) -> Vec<u8> {
    let units = list["units"].as_str().unwrap_or("mm");
    let imperial = units == "in";
    let none = vec![];
    let parts = list["parts"].as_array().unwrap_or(&none);
    let sheets = list["sheets"].as_array().unwrap_or(&none);
    let hardware = list["hardware"].as_array().unwrap_or(&none);
    let mut doc = Doc {
        pages: vec![Vec::new()],
        y: H - M,
    };

    doc.raw(&format!(
        "0.55 0.38 0.22 RG 2.5 w {M} {} m {} {} l S\n",
        H - M,
        M + 72.0,
        H - M
    ));
    doc.y -= 34.0;
    doc.text(M, doc.y, 22.0, true, INK, &format!("{title} — Cut list"));
    doc.y -= 16.0;
    let about = format!("All sizes in {units}, length × width × thickness");
    doc.text(M, doc.y, 9.0, false, GREY, &about);

    // Four tiles: pieces, part sizes, board area, sheets.
    let (area, unit) = if imperial {
        (
            format!("{:.2}", list["area_ft2"].as_f64().unwrap_or(0.0)),
            "board area (ft²)",
        )
    } else {
        (
            format!("{:.2}", list["area_m2"].as_f64().unwrap_or(0.0)),
            "board area (m²)",
        )
    };
    let count = |key: &str| sheets.iter().filter_map(|s| s[key].as_f64()).sum::<f64>();
    let tiles = [
        (shown(&list["boards"]), "pieces to cut"),
        (parts.len().to_string(), "part sizes"),
        (area, unit),
        (count("sheets_2440x1220").to_string(), "sheets 2440 × 1220"),
    ];
    let (gap, th) = (8.0, 44.0);
    let tw = (W - 2.0 * M - 3.0 * gap) / 4.0;
    doc.y -= 16.0 + th;
    for (i, (value, label)) in tiles.iter().enumerate() {
        let x = M + i as f64 * (tw + gap);
        doc.rect(x, doc.y, tw, th, Some(SHADE));
        doc.text(x + 9.0, doc.y + 22.0, 16.0, true, INK, value);
        doc.text(x + 9.0, doc.y + 9.0, 7.5, false, GREY, label);
    }

    // One table per material and thickness, in the order of the sheets to buy.
    let cols = [
        ("CUT", M + 4.0, false, false),
        ("QTY", M + 52.0, true, false),
        ("LENGTH", M + 110.0, true, true),
        ("WIDTH", M + 160.0, true, true),
        ("THICK", M + 200.0, true, false),
        ("PARTS", M + 216.0, false, false),
    ];
    for sheet in sheets {
        let same =
            |p: &&Value| p["material"] == sheet["material"] && p["thickness"] == sheet["thickness"];
        let rows: Vec<Vec<String>> = (parts.iter().filter(same))
            .map(|p| {
                let names = p["parts"].as_array().unwrap_or(&none);
                let names: Vec<&str> = names.iter().filter_map(Value::as_str).collect();
                vec![
                    String::new(),
                    shown(&p["qty"]),
                    shown(&p["length"]),
                    shown(&p["width"]),
                    shown(&p["thickness"]),
                    names.join(", "),
                ]
            })
            .collect();
        let pieces: f64 = parts
            .iter()
            .filter(same)
            .filter_map(|p| p["qty"].as_f64())
            .sum();
        let material = match sheet["material"].as_str().unwrap_or("") {
            "" => "No material",
            m => m,
        };
        let heading = format!("{material} — {} {units}", shown(&sheet["thickness"]));
        let s = if pieces == 1.0 { "" } else { "s" };
        doc.heading(&heading, &format!("({pieces} piece{s})"));
        doc.table(&cols, &rows, true);
    }

    doc.heading("Sheet stock to buy", "");
    let (area_title, per_m2) = if imperial {
        ("AREA (FT²)", 10.7639)
    } else {
        ("AREA (M²)", 1.0)
    };
    let stock: Vec<Vec<String>> = (sheets.iter())
        .map(|s| {
            let area = s["area_m2"].as_f64().unwrap_or(0.0) * per_m2;
            vec![
                s["material"].as_str().unwrap_or("").into(),
                format!("{} {units}", shown(&s["thickness"])),
                format!("{area:.2}"),
                shown(&s["sheets_2440x1220"]),
            ]
        })
        .collect();
    let stock_cols = [
        ("MATERIAL", M + 4.0, false, false),
        ("THICK", M + 300.0, true, false),
        (area_title, M + 390.0, true, false),
        ("SHEETS 2440 × 1220", W - M - 4.0, true, false),
    ];
    doc.table(&stock_cols, &stock, false);

    if !hardware.is_empty() {
        doc.heading("Hardware", "");
        let rows: Vec<Vec<String>> = (hardware.iter())
            .map(|h| vec![String::new(), shown(&h["qty"]), shown(&h["item"])])
            .collect();
        let hw_cols = [
            ("GOT", M + 4.0, false, false),
            ("QTY", M + 52.0, true, false),
            ("ITEM", M + 66.0, false, false),
        ];
        doc.table(&hw_cols, &rows, true);
    }

    let note = "Sizes are the finished sizes of the parts in the model. Sheets are estimated \
                from area plus 15 % waste, not from a cutting layout: allow for saw kerf, \
                trimming and grain direction when laying out.";
    let lines = wrap(note, 7.5, W - 2.0 * M);
    doc.room(18.0 + 10.0 * lines.len() as f64);
    doc.y -= 18.0;
    for line in lines {
        doc.text(M, doc.y, 7.5, false, GREY, &line);
        doc.y -= 10.0;
    }
    assemble(&doc.pages)
}

/// The PDF file around the pages' operators: catalog, page tree, the two fonts, then a page
/// and its content stream for each page, and the table of where each object starts.
fn assemble(pages: &[Vec<u8>]) -> Vec<u8> {
    let font = |name: &str| {
        format!("<< /Type /Font /Subtype /Type1 /BaseFont /{name} /Encoding /WinAnsiEncoding >>")
            .into_bytes()
    };
    let kids: Vec<String> = (0..pages.len())
        .map(|i| format!("{} 0 R", 5 + 2 * i))
        .collect();
    let mut objects = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        format!(
            "<< /Type /Pages /Count {} /Kids [{}] >>",
            pages.len(),
            kids.join(" ")
        )
        .into_bytes(),
        font("Helvetica"),
        font("Helvetica-Bold"),
    ];
    for (i, ops) in pages.iter().enumerate() {
        objects.push(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {W} {H}] \
                 /Resources << /Font << /F1 3 0 R /F2 4 0 R >> >> /Contents {} 0 R >>",
                6 + 2 * i
            )
            .into_bytes(),
        );
        let mut stream = format!("<< /Length {} >>\nstream\n", ops.len()).into_bytes();
        stream.extend(ops);
        stream.extend(b"\nendstream");
        objects.push(stream);
    }
    let mut out = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::new();
    for (i, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend(format!("{} 0 obj\n", i + 1).bytes());
        out.extend(object);
        out.extend(b"\nendobj\n");
    }
    let xref = out.len();
    let size = objects.len() + 1;
    out.extend(format!("xref\n0 {size}\n0000000000 65535 f \n").bytes());
    for offset in offsets {
        out.extend(format!("{offset:010} 00000 n \n").bytes());
    }
    out.extend(
        format!("trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").bytes(),
    );
    out
}

#[cfg(test)]
mod tests;
