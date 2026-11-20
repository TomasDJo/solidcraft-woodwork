use super::*;

fn part(length: f64, width: f64, grain: bool) -> Part {
    Part {
        length,
        width,
        grain,
    }
}

const SHEET: [f64; 2] = [2440.0, 1220.0];

fn check(sheets: &[Vec<Placed>], kerf: f64, trim: f64) {
    check_in(sheets, SHEET, kerf, trim);
}

/// Every part inside the sheet less its trim, at least `kerf` from every other part, and each
/// sheet cut in guillotine cuts.
fn check_in(sheets: &[Vec<Placed>], sheet: [f64; 2], kerf: f64, trim: f64) {
    for placed in sheets {
        for (i, a) in placed.iter().enumerate() {
            assert!(a.x >= trim - EPS && a.y >= trim - EPS, "{a:?}");
            assert!(a.x + a.w <= sheet[0] - trim + EPS, "{a:?}");
            assert!(a.y + a.h <= sheet[1] - trim + EPS, "{a:?}");
            for b in &placed[i + 1..] {
                let apart = a.x + a.w + kerf <= b.x + EPS
                    || b.x + b.w + kerf <= a.x + EPS
                    || a.y + a.h + kerf <= b.y + EPS
                    || b.y + b.h + kerf <= a.y + EPS;
                assert!(apart, "{a:?} {b:?}");
            }
        }
        let rects: Vec<[f64; 4]> = placed.iter().map(|p| [p.x, p.y, p.w, p.h]).collect();
        assert!(guillotine(&rects), "{placed:?}");
    }
}

/// Whether the parts (x, y, w, h) can be cut apart in guillotine cuts: some straight line
/// across them crosses no part, and each side can be cut apart the same way.
fn guillotine(rects: &[[f64; 4]]) -> bool {
    if rects.len() <= 1 {
        return true;
    }
    for axis in 0..2 {
        for r in rects {
            let at = r[axis] + r[axis + 2];
            let crossed = |q: &&[f64; 4]| q[axis] < at - EPS && q[axis] + q[axis + 2] > at + EPS;
            if rects.iter().any(|q| crossed(&q)) {
                continue;
            }
            let (before, after): (Vec<[f64; 4]>, Vec<[f64; 4]>) = rects
                .iter()
                .partition(|q| q[axis] + q[axis + 2] <= at + EPS);
            if !before.is_empty() && !after.is_empty() {
                return guillotine(&before) && guillotine(&after);
            }
        }
    }
    false
}

#[test]
fn the_guillotine_check_catches_a_pinwheel() {
    let strips = [[0.0, 0.0, 2.0, 1.0], [0.0, 1.0, 2.0, 1.0]];
    assert!(guillotine(&strips));
    // four boards around a square: no straight cut goes right across
    let pinwheel = [
        [0.0, 0.0, 2.0, 1.0],
        [2.0, 0.0, 1.0, 2.0],
        [1.0, 2.0, 2.0, 1.0],
        [0.0, 1.0, 1.0, 2.0],
        [1.0, 1.0, 1.0, 1.0],
    ];
    assert!(!guillotine(&pinwheel));
}

/// Forty parts of mixed sizes, every third without grain.
fn forty() -> Vec<Part> {
    (0..40)
        .map(|i| {
            let (l, w) = (
                300.0 + (i * 137 % 900) as f64,
                100.0 + (i * 71 % 500) as f64,
            );
            part(l, w, i % 3 != 0)
        })
        .collect()
}

#[test]
fn a_kitchen_worth_of_parts_without_overlap() {
    let parts = forty();
    for (kerf, trim) in [(4.0, 10.0), (0.0, 0.0), (8.0, 25.0)] {
        let (sheets, oversize) = nest(&parts, SHEET, kerf, trim);
        assert!(oversize.is_empty());
        check(&sheets, kerf, trim);
        // every variant, whichever wins
        let order: Vec<usize> = (0..parts.len()).collect();
        for short_side in [false, true] {
            check(
                &pack(&parts, &order, SHEET, kerf, trim, short_side).0,
                kerf,
                trim,
            );
        }
        let mut placed: Vec<usize> = sheets.iter().flatten().map(|p| p.part).collect();
        placed.sort();
        assert_eq!(
            placed,
            (0..40).collect::<Vec<_>>(),
            "every part placed once"
        );
        // grained parts keep their length along the sheet's length
        for p in sheets.iter().flatten() {
            assert!(p.part.is_multiple_of(3) || !p.rotated, "{p:?}");
            let length = if p.rotated { p.h } else { p.w };
            assert_eq!(length, parts[p.part].length);
        }
    }
}

#[test]
fn never_more_sheets_than_the_first_packer() {
    // the strip packer before it took 5 sheets for each of these
    for (kerf, trim) in [(4.0, 10.0), (0.0, 0.0), (8.0, 25.0)] {
        let (sheets, _) = nest(&forty(), SHEET, kerf, trim);
        assert!(sheets.len() <= 5, "{} sheets", sheets.len());
    }
}

#[test]
fn fewer_sheets_then_the_emptiest_last_sheet_wins() {
    let at = |part: usize, w: f64| Placed {
        part,
        x: 0.0,
        y: 0.0,
        w,
        h: 100.0,
        rotated: false,
    };
    let full_first = [vec![at(0, 500.0), at(1, 500.0)], vec![at(2, 100.0)]];
    let even = [vec![at(0, 500.0)], vec![at(1, 500.0), at(2, 100.0)]];
    let one = [vec![at(0, 500.0), at(1, 500.0), at(2, 100.0)]];
    assert!(rank(&full_first) < rank(&even));
    assert!(rank(&one) < rank(&full_first));
}

#[test]
fn the_kerf_decides_the_sheet_count() {
    // 2420 x 1200 inside the trim: three 800s along it (2408 with two kerfs), and two 600s
    // across only without a kerf (1204 with one)
    let six: Vec<Part> = (0..6).map(|_| part(800.0, 600.0, true)).collect();
    let (sheets, _) = nest(&six, SHEET, 4.0, 10.0);
    assert_eq!(sheets.iter().map(Vec::len).collect::<Vec<_>>(), [3, 3]);
    check(&sheets, 4.0, 10.0);
    let mut xs: Vec<f64> = sheets[0].iter().map(|p| p.x).collect();
    xs.sort_by(f64::total_cmp);
    assert_eq!(xs, [10.0, 814.0, 1618.0]); // the trim, then a kerf after each part
    let (sheets, _) = nest(&six, SHEET, 0.0, 10.0);
    assert_eq!(sheets.len(), 1);
    // a seventh part opens a second sheet
    let mut seven = six;
    seven.push(part(800.0, 600.0, true));
    assert_eq!(nest(&seven, SHEET, 0.0, 10.0).0.len(), 2);
}

#[test]
fn grain_stays_along_the_sheet() {
    // 400 long and 1300 wide: across the sheet it is too wide, so a grained part does not fit,
    // and one without grain turns
    let (sheets, oversize) = nest(&[part(400.0, 1300.0, true)], SHEET, 4.0, 10.0);
    assert!(sheets.is_empty());
    assert_eq!(oversize, [0]);
    let (sheets, oversize) = nest(&[part(400.0, 1300.0, false)], SHEET, 4.0, 10.0);
    assert!(oversize.is_empty());
    let p = &sheets[0][0];
    assert!(p.rotated && (p.w, p.h) == (1300.0, 400.0), "{p:?}");
}

#[test]
fn a_part_turns_where_that_fits_best() {
    // A 2100 x 800 part leaves 296 beside it and 396 above it, whichever way the sheet is
    // cut first. A part 300 long and 1000 wide fits neither as it is; without grain it turns
    // into the piece above, with grain it needs a second sheet.
    let a = || part(2100.0, 800.0, true);
    let small = [2400.0, 1200.0];
    let (sheets, _) = nest(&[a(), part(300.0, 1000.0, false)], small, 4.0, 0.0);
    assert_eq!(sheets.len(), 1);
    let b = sheets[0].iter().find(|p| p.part == 1).unwrap();
    assert!(b.rotated && (b.w, b.h) == (1000.0, 300.0), "{b:?}");
    check_in(&sheets, small, 4.0, 0.0);
    let (sheets, _) = nest(&[a(), part(300.0, 1000.0, true)], small, 4.0, 0.0);
    assert_eq!(sheets.len(), 2);
    assert!(sheets.iter().flatten().all(|p| !p.rotated));
}

#[test]
fn oversize_parts_are_reported_not_dropped() {
    let parts = [
        part(2600.0, 600.0, false), // longer than the sheet either way
        part(2420.0, 1200.0, true), // fills the sheet less the trim exactly
        part(2420.0, 1201.0, true),
    ];
    let (sheets, mut oversize) = nest(&parts, SHEET, 4.0, 10.0);
    oversize.sort();
    assert_eq!(oversize, [0, 2]);
    assert_eq!(sheets.len(), 1);
    assert_eq!(sheets[0][0].part, 1);
}

#[test]
fn the_larger_leftover_keeps_the_full_side() {
    // 300 x 900 in 1000 x 1000 leaves 700 beside it and 100 above: the cut across comes first,
    // so the piece beside keeps the full 1000 and a 650 x 1000 part fits there
    let square = [1000.0, 1000.0];
    let parts = [part(300.0, 900.0, true), part(650.0, 1000.0, true)];
    let (sheets, _) = pack(&parts, &[0, 1], square, 0.0, 0.0, false);
    assert_eq!(sheets.len(), 1);
    assert_eq!((sheets[0][1].x, sheets[0][1].y), (300.0, 0.0));
    // turned the other way round, the cut along comes first and the piece above keeps it
    let parts = [part(900.0, 300.0, true), part(1000.0, 650.0, true)];
    let (sheets, _) = pack(&parts, &[0, 1], square, 0.0, 0.0, false);
    assert_eq!(sheets.len(), 1);
    assert_eq!((sheets[0][1].x, sheets[0][1].y), (0.0, 300.0));
    check_in(&sheets, square, 0.0, 0.0);
}

#[test]
fn the_short_side_fit_takes_the_snug_piece() {
    // After a 1200 x 800 part in 2400 x 1300 (cut across first): 1196 x 1300 beside it and
    // 1200 x 496 above it. A 1190 x 390 part fits both; by area it goes into the smaller
    // piece above, by short side into the one beside, where it leaves only 6.
    let parts = [part(1200.0, 800.0, true), part(1190.0, 390.0, true)];
    let size = [2400.0, 1300.0];
    let area = pack(&parts, &[0, 1], size, 4.0, 0.0, false);
    let snug = pack(&parts, &[0, 1], size, 4.0, 0.0, true);
    assert_eq!((area.0[0][1].x, area.0[0][1].y), (0.0, 804.0));
    assert_eq!((snug.0[0][1].x, snug.0[0][1].y), (1204.0, 0.0));
}
