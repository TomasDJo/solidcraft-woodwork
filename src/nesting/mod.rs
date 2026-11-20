//! Parts laid out on sheets, in cuts a panel saw can make: every cut runs right across the
//! piece it is made in (guillotine cuts). Sizes in mm; x runs along the sheet's length, y
//! along its width, from the sheet's corner.

/// A part to cut: its length runs along the grain when it has one.
#[derive(Clone, Copy)]
pub struct Part {
    pub length: f64,
    pub width: f64,
    pub grain: bool,
}

/// A part on a sheet: which part (its index in the list given), its corner, its size along x
/// and y, and whether its length runs along y (turned a quarter).
#[derive(Debug, Clone, PartialEq)]
pub struct Placed {
    pub part: usize,
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub rotated: bool,
}

const EPS: f64 = 1e-6;

/// The sheets, each its placed parts, and the indices of the parts too big for a sheet less
/// its trim. `sheet` is length and width; `kerf` is left between parts; `trim` is cut off every
/// edge. A part with grain keeps its length along the sheet's length; one without may turn.
// ponytail: 8 greedy layouts (4 orders x 2 fit rules) into the free pieces of all open
// sheets; the fewest sheets wins, then the emptiest last sheet. Not optimal: free pieces are
// never joined again, and nothing is searched beyond these orders.
pub fn nest(
    parts: &[Part],
    sheet: [f64; 2],
    kerf: f64,
    trim: f64,
) -> (Vec<Vec<Placed>>, Vec<usize>) {
    // largest first by: area; longest side; length; width
    let key = |p: &Part, by: usize| match by {
        0 => [p.length * p.width, 0.0],
        1 => [p.length.max(p.width), p.length * p.width],
        2 => [p.length, p.width],
        _ => [p.width, p.length],
    };
    let mut best: Option<(Vec<Vec<Placed>>, Vec<usize>)> = None;
    for by in 0..4 {
        let mut order: Vec<usize> = (0..parts.len()).collect();
        order.sort_by(|&a, &b| key(&parts[b], by).partial_cmp(&key(&parts[a], by)).unwrap());
        for short_side in [false, true] {
            let laid = pack(parts, &order, sheet, kerf, trim, short_side);
            let better = best.as_ref().is_none_or(|b| rank(&laid.0) < rank(&b.0));
            if better {
                best = Some(laid);
            }
        }
    }
    best.unwrap()
}

/// How good a layout is, less is better: its sheets, then the area used on the last one (the
/// emptier, the bigger the offcut left in one piece).
fn rank(sheets: &[Vec<Placed>]) -> (usize, f64) {
    let used = |s: &Vec<Placed>| s.iter().map(|p| p.w * p.h).sum::<f64>();
    (sheets.len(), sheets.last().map_or(0.0, used))
}

/// Parts in this order, each into the free piece of any open sheet that fits it best: the
/// smallest piece, or with `short_side` the one that leaves the least on its shorter side.
/// Each part splits its piece in two, in guillotine cuts.
fn pack(
    parts: &[Part],
    order: &[usize],
    sheet: [f64; 2],
    kerf: f64,
    trim: f64,
    short_side: bool,
) -> (Vec<Vec<Placed>>, Vec<usize>) {
    let blank = [trim, trim, sheet[0] - 2.0 * trim, sheet[1] - 2.0 * trim];
    // per sheet: its free pieces (x, y, w, h) and its parts
    let mut sheets: Vec<(Vec<[f64; 4]>, Vec<Placed>)> = Vec::new();
    let mut oversize = Vec::new();
    for &i in order {
        let part = &parts[i];
        let mut sizes = vec![(part.length, part.width, false)];
        if !part.grain {
            sizes.push((part.width, part.length, true));
        }
        let fits = |r: &[f64; 4], &(w, h, _): &(f64, f64, bool)| w <= r[2] + EPS && h <= r[3] + EPS;
        let Some(&first) = sizes.iter().find(|s| fits(&blank, s)) else {
            oversize.push(i);
            continue;
        };
        let (mut best, mut least) = (None, [f64::INFINITY; 2]);
        for (s, (free, _)) in sheets.iter().enumerate() {
            for (f, r) in free.iter().enumerate() {
                for size in sizes.iter().filter(|z| fits(r, z)) {
                    let (dw, dh) = (r[2] - size.0, r[3] - size.1);
                    let key = match short_side {
                        false => [r[2] * r[3], dw.min(dh)],
                        true => [dw.min(dh), dw.max(dh)],
                    };
                    if key < least {
                        (best, least) = (Some((s, f, *size)), key);
                    }
                }
            }
        }
        let (s, f, (w, h, rotated)) = match best {
            Some(found) => found,
            None => {
                sheets.push((vec![blank], Vec::new()));
                (sheets.len() - 1, 0, first)
            }
        };
        let (free, placed) = &mut sheets[s];
        let r = free.remove(f);
        placed.push(Placed {
            part: i,
            x: r[0],
            y: r[1],
            w,
            h,
            rotated,
        });
        // Split along the shorter leftover: the larger leftover keeps the piece's full side.
        let (beside, above) = match r[2] - w < r[3] - h {
            // a cut along the sheet at the part's top edge, then one across at its end
            true => (
                [r[0] + w + kerf, r[1], r[2] - w - kerf, h],
                [r[0], r[1] + h + kerf, r[2], r[3] - h - kerf],
            ),
            // a cut across at the part's end, then one along at its top edge
            false => (
                [r[0] + w + kerf, r[1], r[2] - w - kerf, r[3]],
                [r[0], r[1] + h + kerf, w, r[3] - h - kerf],
            ),
        };
        free.extend(
            [beside, above]
                .into_iter()
                .filter(|p| p[2] > EPS && p[3] > EPS),
        );
    }
    (
        sheets.into_iter().map(|(_, placed)| placed).collect(),
        oversize,
    )
}

#[cfg(test)]
mod tests;
