//! L-shaped corner cabinet: one leg along the back wall, the other (the return) running
//! forward along a side wall.
//!
//! Written for a corner at the back left: the footprint's bounding box is `width` (X) by
//! `return` (Y, the front at 0), and the return is on the left. A right corner is the same
//! mirrored in X. Like the box cabinet, every size is an expression over design parameters.
//!
//! The diagonal shape has one front straight across the open corner, at 45 degrees, in place
//! of the two fronts of the L.
//!
//! This file is the carcass; `input.rs` reads the dialog, `doors.rs` is the L's bi-fold
//! pair, `diagonal.rs` the diagonal shape's door and plinth.

mod diagonal;
mod doors;
mod input;

use crate::fittings::{self, PinRows, CLIP, LEG_INSET, PIN_SETBACK, PITCH, STRIP, STRIP_SETBACK};
use crate::joint::{Joinery, Joint, BOTTOM, CARCASS};
use crate::kit::{mm, Kit};
use crate::Commands;
pub use input::hand;
use input::Input;
use serde_json::{json, Value};

/// A face carrying shelves (`Corner::shelf_faces`): board, first row's place, way in, way to
/// the second row, run.
type ShelfFace = (String, [String; 2], [i32; 3], [i32; 3], String);

// Split boards: two dowels across each joint, in from its ends far enough for the shortest
// leg (`input` keeps it 100 mm or more).
const DOWEL_D: f64 = 8.0;
const DOWEL_INTO: f64 = 30.0; // into each board
const DOWEL_INSET: f64 = 40.0; // from each end of the joint

/// What the steps of a build share: the checked input in mm, and the sizes as expressions.
struct Corner<'a> {
    kit: &'a Kit,
    joinery: Joinery<'a>,
    right: bool,
    diagonal: bool,
    split: bool, // the L-shaped boards as two dowelled boards
    handles: &'a str,
    handle_at: &'a str,
    /// Grooved fronts and their spacing and depth parameters, on a cabinet with doors.
    grooves: Option<(fittings::Grooves<'a>, [String; 2])>,
    /// Shaker doors' frame width and panel thickness parameters.
    eframe: Option<[String; 2]>,
    glass: bool,          // glass doors: a pane in the frame
    egap: Option<String>, // a grip edge's gap, the parameter
    gap: f64,             // and in mm, 0 without one
    a: f64,
    b: f64,
    d: f64,
    h: f64,
    t: f64,
    plinth: f64,
    plinth_type: &'a str, // board, notch, legs or casters: see `fittings::plinth_type`
    dr: f64,              // depth at the return's end: `d` on the L
    g: f64,               // the reveal around the doors, and its parameter
    inset: bool,          // the doors inside the opening, the shelves a panel back behind them
    pilasters: bool,      // movable shelves on pilaster strips and clips, not pins (#264)
    er: String,
    ea: String,
    eb: String,
    ed: String,
    edr: String,
    eh: String,
    et: String,
    epl: String,
    ekick: String, // the plinth board's or the notch's set-back
    wall_mount: &'a str,
    // The mount board's thickness and height: the backs stand that much off the walls.
    emount: Option<[String; 2]>,
}

/// The build's checks of the fields, without making anything: the dialog hook's.
pub fn checked(p: &Value) -> Result<(), String> {
    input::read(&Kit::new(p)?, p).map(|_| ())
}

pub fn build(p: &Value) -> Result<(Commands, Value), String> {
    let kit = Kit::new(p)?;
    let Input {
        a,
        b,
        d,
        h,
        t,
        plinth,
        plinth_type,
        plinth_setback,
        dr,
        shelves,
        pins,
        pilasters,
        on_return,
        handles,
        handle_at,
        grooves,
        frame,
        grip_gap,
        glass,
        right,
        diagonal,
        split,
        reveal,
        inset,
        wall_mount,
        mount_t,
        mount_h,
    } = input::read(&kit, p)?;

    let ea = kit.parameter("width", a, "along the back wall");
    let eb = kit.parameter("return", b, "along the side wall");
    let ed = kit.parameter("depth", d, "depth of both legs");
    let eh = kit.parameter("height", h, "outside height, plinth included");
    let et = kit.parameter("thickness", t, "panel thickness");
    // Over a plinth board or notch the ends and backs reach below the bottom (`BOTTOM`).
    let closed = plinth > 0.0 && !matches!(plinth_type, "legs" | "casters");
    let joinery = Joinery::new(&kit, p, (t, &et), false, closed)?;
    // ponytail: no dado on a corner cabinet: its L-shaped boards (notch, chamfer picked by
    // point) and the backs would all have to grow into the grooves.
    crate::kit::check(
        joinery.method(CARCASS) != "dado",
        "`joints`: a dado is for box cabinets; choose another joint for a corner cabinet",
    )?;
    // the shortest joint: the backs and the ends, the return's end, a split board's return
    let shortest = [d - t, dr - t, h - plinth - 2.0 * t, b - d - t];
    let shortest = match split {
        true => shortest.iter().fold(f64::MAX, |a, b| a.min(*b)),
        false => shortest[..3].iter().fold(f64::MAX, |a, b| a.min(*b)),
    };
    joinery.fits(CARCASS, shortest, p, "the shortest joint")?;
    // As on the box cabinet: a plinth taken away sets its parameter to 0.
    let epl = match plinth > 0.0 {
        true => kit.parameter("plinth", plinth, "plinth height"),
        false if kit.has("plinth") => {
            kit.parameter("plinth", 0.0, "plinth height");
            mm(0.0)
        }
        false => mm(0.0),
    };
    // As on the box cabinet.
    let on_feet = matches!(plinth_type, "legs" | "casters");
    let ekick = match plinth > 0.0 && !on_feet || kit.has("plinth_setback") {
        true => kit.parameter("plinth_setback", plinth_setback, "plinth set-back"),
        false => mm(plinth_setback),
    };
    // Only with doors, or when an edit took them off (as on the box cabinet).
    let er = match on_return.is_some() || kit.has("reveal") {
        true => kit.parameter_as("reveal", reveal, "gap around the doors", mm(reveal)),
        false => mm(reveal),
    };
    let grooves = grooves.map(|g| (g, fittings::groove_sizes(&kit, &g)));
    let eframe = frame.map(|f| fittings::frame_sizes(&kit, f));
    let egap = grip_gap.map(|g| kit.parameter("grip_gap", g, "grip edge gap"));
    let emount = fittings::mount_sizes(&kit, wall_mount, mount_t, mount_h, &et);
    // A rail on each wall.
    let steel = wall_mount == "steel";
    fittings::steel_rail(&kit, "wall_rail", steel, a, &ea);
    fittings::steel_rail(&kit, "wall_rail_return", steel, b, &eb);
    kit.centre(&ea, &eb);
    let edr = match diagonal {
        true => format!("({ea} - {eb} + {ed})"),
        false => ed.clone(),
    };
    let corner = Corner {
        kit: &kit,
        joinery,
        right,
        diagonal,
        split,
        handles,
        handle_at,
        grooves,
        eframe,
        glass,
        egap,
        gap: grip_gap.unwrap_or(0.0),
        a,
        b,
        d,
        h,
        t,
        plinth,
        plinth_type: if plinth > 0.0 { plinth_type } else { "none" },
        dr,
        g: reveal,
        inset,
        pilasters,
        er,
        ea,
        eb,
        ed,
        edr,
        eh,
        et,
        epl,
        ekick,
        wall_mount,
        emount,
    };
    let pins = corner.carcass(shelves, pins); // holes, run after the panels exist
    corner.joints();
    // the joints before the pin rows: a cut into a board full of pin holes is slow
    let mut drill = corner.joinery.finish();
    drill.extend(pins);
    match on_return {
        Some(on_return) if diagonal => diagonal::door(&corner, on_return),
        Some(on_return) => doors::build(&corner, on_return),
        None => {}
    }
    // The bounding box: `return` is its depth.
    Ok(kit.finish(drill, [a, h, b]))
}

impl Corner<'_> {
    /// Mirrored for a right corner: where a box of this length starts.
    fn from(&self, x: &str, length: &str) -> String {
        match self.right {
            true => format!("{} - ({x}) - ({length})", self.ea),
            false => x.to_string(),
        }
    }

    /// Mirrored for a right corner: where a point is.
    fn point(&self, x: &str) -> String {
        self.from(x, "0 mm")
    }

    fn flip(&self) -> i32 {
        if self.right {
            -1
        } else {
            1
        }
    }

    fn panel(&self, part: &str, corner: [&str; 3], size: [&str; 3], cut: Option<&[String]>) {
        let x = self.from(corner[0], size[0]);
        self.kit.panel(part, [&x, corner[1], corner[2]], size, cut)
    }

    /// The end of the back leg and the end of the return are the cabinet's two sides: together
    /// they span its whole footprint, which is what placing against it measures.
    fn ends(&self) -> (&'static str, &'static str) {
        match self.right {
            true => ("Side L", "Side R"),
            false => ("Side R", "Side L"),
        }
    }

    /// The sides, backs, L-shaped boards and plinth. Returns the shelf pin holes.
    fn carcass(&self, s: u32, pins: bool) -> Commands {
        let Corner {
            ea,
            eb,
            ed,
            edr,
            eh,
            et,
            epl,
            ..
        } = self;
        let name = self.kit.name.as_str();
        let panel = |part: &str, corner: [&str; 3], size: [&str; 3], cut: Option<&[String]>| {
            self.panel(part, corner, size, cut)
        };
        let (back_end, return_end) = self.ends();
        let zero = mm(0.0);
        // Where a neighbour stands: at the end of the back leg its back is on the same wall;
        // at the end of the return the wall is beside it.
        let side = |end: &'static str| &end[end.len() - 1..];
        (self.kit).seat(side(back_end), false, &self.point(ea), eb);
        (self.kit).seat(side(return_end), true, &self.point(&zero), &zero);
        // On legs or casters the whole carcass stands on them; else the ends and backs reach
        // the floor.
        let (z, high) = match self.plinth_type {
            "legs" | "casters" => (epl.clone(), format!("{eh} - {epl}")),
            _ => (zero.clone(), eh.to_string()),
        };
        let back_end_x = format!("{ea} - {et}");
        let front_y = format!("{eb} - {ed}");
        panel(back_end, [&back_end_x, &front_y, &z], [et, ed, &high], None);
        panel(return_end, [&zero, &zero, &z], [edr, et, &high], None);
        // Full panels against both walls, standing on the floor like the sides. With a mount
        // board they stand that far off the walls, between the bottom and the top, which
        // reach the walls over the boards.
        let ech = format!("({eh} - {epl} - 2*{et})"); // clear height
        let (tm, back_z, back_h) = match &self.emount {
            Some([tm, _]) => (tm.clone(), format!("{epl} + {et}"), ech.clone()),
            None => (zero.clone(), z.clone(), high.clone()),
        };
        let off = |e: String| match self.emount {
            Some(_) => format!("{e} - {tm}"),
            None => e,
        };
        let back_x = match self.emount {
            Some(_) => format!("{et} + {tm}"),
            None => et.clone(),
        };
        // Finger joints: the back runs through the back return and the back leg's end to
        // their outsides, the back return through the return's end (`fingered`).
        let grow = |yes: bool| if yes { et.to_string() } else { zero.clone() };
        let (left, right) = (
            grow(self.fingered("Back return", "Back")),
            grow(self.fingered(back_end, "Back")),
        );
        let front = grow(self.fingered(return_end, "Back return"));
        panel(
            "Back",
            [
                &format!("{back_x} - {left}"),
                &off(format!("{eb} - {et}")),
                &back_z,
            ],
            [
                &off(format!("{ea} - 2*{et} + {left} + {right}")),
                et,
                &back_h,
            ],
            None,
        );
        panel(
            "Back return",
            [&tm, &format!("{et} - {front}"), &back_z],
            [et, &off(format!("{eb} - {et} + {front}")), &back_h],
            None,
        );
        // The mount boards under the top: along the back wall from the side wall to the back
        // leg's end, and along the side wall from behind the return's end. Box edges (see
        // `fittings::mount_board`): the back wall's as on the box cabinet (front bottom 9,
        // back top 3); along the side wall the inner face is +x (bottom 11, wall top 0), or
        // -x for a right corner (7, 2).
        if let Some([tm, mh]) = &self.emount {
            let z = format!("{eh} - {et} - {mh}");
            let mount = |at: [&str; 3], size: [&str; 3], suffix: &str, edges: [u32; 2]| {
                let board =
                    |part: &str, at: [&str; 3], size: [&str; 3]| panel(part, at, size, None);
                let wall_mount = self.wall_mount;
                fittings::mount_board(self.kit, wall_mount, tm, board, [at, size], suffix, edges)
            };
            let y = format!("{eb} - {tm}");
            mount(
                [&zero, &y, &z],
                [&format!("{ea} - {et}"), tm, mh],
                "",
                [9, 3],
            );
            let length = format!("{eb} - {et} - {tm}");
            let edges = if self.right { [7, 2] } else { [11, 0] };
            mount([&zero, et, &z], [tm, &length, mh], " return", edges);
        }
        // Bottom, top and shelves are L-shaped: a board over the whole inside, with the open
        // corner cut out of it, square or (diagonal) at 45 degrees. The cut list gives the
        // board before the cut. Each board's height is also kept in mm, for picking its edge.
        let egap = format!("(({ech} - {s}*{et}) / {})", s + 1); // between shelves
        let (t, clear_h) = (self.t, self.h - self.plinth - 2.0 * self.t);
        let gap = (clear_h - s as f64 * t) / (s + 1) as f64;
        let mut boards = vec![
            ("Bottom".to_string(), epl.clone(), self.plinth),
            ("Top".to_string(), format!("{eh} - {et}"), self.h - t),
        ];
        let elast = fittings::top_pin_row(&ech);
        for i in 0..s {
            // Height over the bottom panel: evenly spaced, or on the nearest row of pins.
            let mut over = format!("{egap}*{} + {et}*{i}", i + 1);
            let mut over_mm = gap * (i + 1) as f64 + t * i as f64;
            if pins {
                over = fittings::on_pin_row(&over, &elast);
                over_mm = fittings::on_pin_row_mm(over_mm, clear_h);
            }
            boards.push((
                format!("Shelf {}", i + 1),
                format!("{epl} + {et} + {over}"),
                self.plinth + t + over_mm,
            ));
        }
        for (part, z, z_mm) in &boards {
            // Behind inset doors a shelf's front edges are a panel further back.
            let back = self.inset && part.starts_with("Shelf");
            // Where the board starts from either wall: inside the backs; with a mount board
            // the bottom and top reach the walls and the shelves stop at the backs.
            let w0 = match (&self.emount, part.starts_with("Shelf")) {
                (None, _) => et.clone(),
                (Some(_), false) => zero.clone(),
                (Some([tm, _]), true) => format!("({et} + {tm})"),
            };
            // From there to inside the other end: the whole inside without a mount board.
            let inner = |e: &str| match self.emount {
                None => format!("{e} - 2*{et}"),
                Some(_) => format!("{e} - {et} - {w0}"),
            };
            // The open corner: x from, its width, and its depth from y = t.
            let notch = match back {
                true => [
                    format!("{ed} - {et}"),
                    format!("{ea} - {ed}"),
                    format!("{eb} - {ed}"),
                ],
                false => [
                    ed.to_string(),
                    format!("{ea} - {et} - {ed}"),
                    format!("{eb} - {ed} - {et}"),
                ],
            };
            // Finger joints: the bottom and top run through the uprights round them to their
            // outsides, and leave the corners where two uprights meet (`fingered`).
            let piece = format!("{part} return");
            let front_board = if self.split {
                piece.as_str()
            } else {
                part.as_str()
            };
            let ext = |face: &str, end: &str| match self.fingered(face, end) {
                true => et.to_string(),
                false => zero.clone(),
            };
            let (el, er, eb_, ef) = (
                ext("Back return", part),
                ext(back_end, part),
                ext("Back", part),
                ext(return_end, front_board),
            );
            let fingers = [&el, &er, &eb_, &ef].iter().any(|e| **e != zero);
            if self.split {
                // Two rectangles, joined along the back leg's front edge: the back leg's
                // board over the whole width, and the return's in front of it.
                let leg = format!("{} - {w0}", notch[0]); // inside depth of both legs
                let joint = format!("{et} + {}", notch[2]);
                let wide = inner(ea);
                match fingers {
                    true => {
                        let x0 = format!("{w0} - {el}");
                        let size = [format!("{wide} + {el} + {er}"), format!("{leg} + {eb_}")];
                        panel(part, [&x0, &joint, z], [&size[0], &size[1], et], None);
                        let y0 = format!("{et} - {ef}");
                        let size = [format!("{leg} + {el}"), format!("{} + {ef}", notch[2])];
                        panel(&piece, [&x0, &y0, z], [&size[0], &size[1], et], None);
                        let far = format!("{joint} + {leg} + {eb_} - {et}");
                        let right = format!("{w0} + {wide} + {er} - {et}");
                        self.corner(part, z, (&el, &eb_), [&x0, &far], "back left");
                        self.corner(part, z, (&er, &eb_), [&right, &far], "back right");
                        self.corner(&piece, z, (&el, &ef), [&x0, &y0], "front left");
                    }
                    false => {
                        panel(part, [&w0, &joint, z], [&wide, &leg, et], None);
                        panel(&piece, [&w0, et, z], [&leg, &notch[2], et], None);
                    }
                }
                let targets = [format!("{name} {part}"), format!("{name} {piece}")];
                let ends = [
                    format!("{w0} + {}", mm(DOWEL_INSET)),
                    format!("{} - {}", notch[0], mm(DOWEL_INSET)),
                ];
                for (n, x) in ends.iter().enumerate() {
                    // One cut through both boards, across the joint.
                    let y = format!("{joint} - {}", mm(DOWEL_INTO));
                    let base = self.kit.at(&self.point(x), &y, &format!("{z} + {et}/2"));
                    self.kit.push((
                        "solid.cylinder",
                        json!({
                            "base": base, "axis": [0, 1, 0],
                            "diameter": mm(DOWEL_D), "height": mm(2.0 * DOWEL_INTO),
                            "operation": "cut", "targets": targets,
                            "name": format!("{name} {piece} dowel {}", n + 1),
                        }),
                    ));
                }
                continue;
            }
            if fingers {
                // grown on its fingered edges; the open corner's notch from the front edge
                let x0 = format!("{w0} - {el}");
                let y0 = format!("{et} - {ef}");
                let wide = format!("{} + {el} + {er}", inner(ea));
                let deep = format!("{} + {ef} + {eb_}", inner(eb));
                panel(part, [&x0, &y0, z], [&wide, &deep, et], None);
                let far = format!("{y0} + {deep} - {et}");
                let right = format!("{x0} + {wide} - {et}");
                self.corner(part, z, (&el, &eb_), [&x0, &far], "back left");
                self.corner(part, z, (&er, &eb_), [&right, &far], "back right");
                self.corner(part, z, (&el, &ef), [&x0, &y0], "front left");
                if !self.diagonal {
                    let x1 = format!("{x0} + {wide}");
                    let size = [
                        format!("{x1} - ({})", notch[0]),
                        format!("{eb} - {ed} - ({y0})"),
                    ];
                    panel(
                        &format!("{part} notch"),
                        [&notch[0], &y0, z],
                        [&size[0], &size[1], et],
                        Some(&[format!("{name} {part}")]),
                    );
                    continue;
                }
            } else {
                panel(part, [&w0, et, z], [&inner(ea), &inner(eb), et], None);
            }
            if self.diagonal {
                // The board's upright edge at the open corner, cut back to the front.
                let edge = [
                    (self.a / 2.0 - t) * self.flip() as f64,
                    t - self.b / 2.0,
                    z_mm + t / 2.0,
                ];
                self.kit.push((
                    "solid.chamfer",
                    json!({
                        "edges": [edge], "distance": match back {
                            true => format!("{eb} - {ed} - 2*{et} + sqrt(2)*{et}"),
                            false => format!("{eb} - {ed} - 2*{et}"),
                        },
                        "body": format!("{name} {part}"), "name": format!("{name} {part} corner"),
                    }),
                ));
                continue;
            }
            panel(
                &format!("{part} notch"),
                [&notch[0], et, z],
                [&notch[1], &notch[2], et],
                Some(&[format!("{name} {part}")]),
            );
        }
        let kick = self.ekick.clone();
        if self.plinth_type == "notch" {
            // The toe kick cut out of each end's front bottom corner, as on the box cabinet.
            let ends = [
                (
                    back_end,
                    [back_end_x.as_str(), &front_y],
                    [et.as_str(), &kick],
                ),
                (return_end, [&format!("{edr} - {kick}"), &zero], [&kick, et]),
            ];
            for (end, [x, y], [sx, sy]) in ends {
                let target = [format!("{name} {end}")];
                panel(
                    &format!("{end} notch"),
                    [x, y, &zero],
                    [sx, sy, epl],
                    Some(&target),
                );
            }
        } else if matches!(self.plinth_type, "legs" | "casters") {
            // Under each corner of the footprint, and the L's inner corner. A caster's plate
            // stays under the bottom panel, clear of the ends and backs (as on the box cabinet).
            let inset = mm(LEG_INSET);
            let near = match self.plinth_type {
                "legs" => inset.clone(),
                _ => format!("({et} + {inset})"),
            };
            let (wall_x, wall_y) = (format!("{ea} - {near}"), format!("{eb} - {near}"));
            let front = format!("{front_y} + {inset}");
            let mut at = vec![
                (wall_x.clone(), front.clone()),
                (wall_x, wall_y.clone()),
                (near.clone(), wall_y),
                (near.clone(), near.clone()),
                (format!("{edr} - {inset}"), near),
            ];
            if !self.diagonal {
                at.push((format!("{ed} - {inset}"), front));
            }
            for (n, (x, y)) in at.iter().enumerate() {
                // Split, the two at the return's end are under the return's board.
                let bottom = match self.split && (n == 3 || n == 4) {
                    true => "Bottom return",
                    false => "Bottom",
                };
                let x = self.point(x);
                fittings::foot(
                    self.kit,
                    self.plinth_type,
                    n + 1,
                    [&x, y],
                    epl,
                    [et, bottom],
                );
            }
        } else if self.plinth_type == "board" && self.diagonal {
            diagonal::plinth(self);
        } else if self.plinth_type == "board" {
            // Two boards set back from the fronts, meeting in the inner corner.
            let x = format!("{ed} - {kick} - {et}");
            let y = format!("{eb} - {ed} + {kick}");
            panel(
                "Plinth",
                [&x, &y, &zero],
                [&format!("{ea} - {et} - ({x})"), et, epl],
                None,
            );
            panel(
                "Plinth return",
                [&x, et, &zero],
                [et, &format!("{y} - {et}"), epl],
                None,
            );
        }
        match (pins, self.pilasters) {
            (true, true) => {
                let shelves: Vec<(String, String)> = (boards.iter())
                    .filter(|b| b.0.starts_with("Shelf ") && !b.0.ends_with(" return"))
                    .map(|b| (b.0.clone(), b.1.clone()))
                    .collect();
                self.strips(&ech, clear_h.round(), &shelves);
                Vec::new()
            }
            (true, false) => self.pins(&elast),
            _ => Vec::new(),
        }
    }

    /// Whether the joint of `end` into `face` is finger jointed: the method says finger (the
    /// bottom's, `BOTTOM`, over a plinth), and the joint is a corner, open at both boards'
    /// edges. Else it is butted (`none`): with a wall mount the backs stand on the bottom, under
    /// the top, and against the middle of the back leg's and the return's ends (through
    /// mortises), and the diagonal's bottom and top keep their front edges for the chamfer.
    fn fingered(&self, face: &str, end: &str) -> bool {
        let scope = match end.starts_with("Bottom") {
            true => BOTTOM,
            false => CARCASS,
        };
        if self.joinery.method(scope) != "finger" {
            return false;
        }
        let mount = self.emount.is_some();
        let (back_end, return_end) = self.ends();
        let ends = face == back_end || face == return_end;
        match end {
            "Back" if face == "Back return" => true,
            "Back" | "Back return" => !mount && ends,
            e if e.starts_with("Bottom") || e.starts_with("Top") => match ends {
                true => !self.diagonal,
                false => !mount && (face == "Back" || face == "Back return"),
            },
            _ => false,
        }
    }

    /// Cuts a board's corner square, a board thick each way, at `[x, y]` (left corner
    /// coordinates) where both its edges there are grown into finger joints (`grown`): two
    /// uprights meet there, finger jointed to each other.
    fn corner(
        &self,
        part: &str,
        z: &str,
        grown: (&String, &String),
        [x, y]: [&String; 2],
        what: &str,
    ) {
        let zero = mm(0.0);
        if *grown.0 == zero || *grown.1 == zero {
            return;
        }
        let et = self.et.as_str();
        let cut = [format!("{} {part}", self.kit.name)];
        self.panel(
            &format!("{part} corner {what}"),
            [x, y, z],
            [et, et, et],
            Some(&cut),
        );
    }

    /// The carcass's joints (`joints`): the backs' ends into the ends and the back return,
    /// and the bottom and top into the four upright panels round them; with a wall mount the
    /// bottom and top reach the walls and the backs stand on the bottom, under the top. A split
    /// board's two pieces keep their own dowels (an edge joint).
    fn joints(&self) {
        let Corner {
            ea,
            eb,
            ed,
            edr,
            eh,
            et,
            epl,
            ..
        } = self;
        let f = self.flip();
        let (back_end, return_end) = self.ends();
        let zero = mm(0.0);
        let (tm, mount) = match &self.emount {
            Some([tm, _]) => (tm.clone(), true),
            None => (zero.clone(), false),
        };
        let (z, high) = match (mount, self.plinth_type) {
            (true, _) => (format!("{epl} + {et}"), format!("({eh} - {epl} - 2*{et})")),
            (_, "legs" | "casters") => (epl.clone(), format!("{eh} - {epl}")),
            _ => (zero.clone(), eh.clone()),
        };
        // x as for a left corner; mirrored here with the directions along X
        let joint = |face: &str,
                     end: &str,
                     at: [String; 3],
                     into: [i32; 3],
                     along: [i32; 3],
                     run: String| {
            let joint = Joint {
                face: face.into(),
                end: end.into(),
                at: [self.point(&at[0]), at[1].clone(), at[2].clone()],
                into: [into[0] * f, into[1], into[2]],
                // the end board's hidden face: under the bottom, over the top, a back's wall side
                out: match end {
                    "Back" => [0, 1, 0],
                    "Back return" => [-f, 0, 0],
                    e if e.starts_with("Bottom") => [0, 0, -1],
                    _ => [0, 0, 1],
                },
                along: [along[0] * f, along[1], along[2]],
                run,
                single: false,
                between: false,
            };
            // finger joints where they can be made, else butted (`fingered`)
            let scope = match end.starts_with("Bottom") {
                true => BOTTOM,
                false => CARCASS,
            };
            let method = match self.joinery.method(scope) {
                "finger" if !self.fingered(face, end) => "none",
                m => m,
            };
            self.joinery.joint_by(method, &joint);
        };
        let (x, y, up) = ([1, 0, 0], [0, 1, 0], [0, 0, 1]);
        let back_y = format!("{eb} - {tm} - {et}/2"); // the Back's middle
        let back_x = format!("{tm} + {et}/2"); // the Back return's
                                               // The backs' upright ends.
        let inner_x = format!("{et} + {tm}");
        let wall_x = format!("{ea} - {et}");
        joint(
            back_end,
            "Back",
            [wall_x.clone(), back_y.clone(), z.clone()],
            [-1, 0, 0],
            up,
            high.clone(),
        );
        joint(
            "Back return",
            "Back",
            [inner_x.clone(), back_y.clone(), z.clone()],
            x,
            up,
            high.clone(),
        );
        joint(
            return_end,
            "Back return",
            [back_x.clone(), et.clone(), z.clone()],
            y,
            up,
            high.clone(),
        );
        let front_y = format!("{eb} - {ed}");
        for (part, plane, mid, down) in [
            (
                "Bottom",
                format!("{epl} + {et}"),
                format!("{epl} + {et}/2"),
                1,
            ),
            ("Top", format!("{eh} - {et}"), format!("{eh} - {et}/2"), -1),
        ] {
            let piece = format!("{part} return");
            // what is in front of the back leg: the return's piece when split
            let front = if self.split { piece.as_str() } else { part };
            let (w0, reach) = match mount {
                true => (zero.clone(), zero.clone()),
                false => (et.clone(), et.clone()),
            };
            joint(
                back_end,
                part,
                [wall_x.clone(), front_y.clone(), mid.clone()],
                [-1, 0, 0],
                y,
                format!("{ed} - {reach}"),
            );
            joint(
                return_end,
                front,
                [w0.clone(), et.clone(), mid.clone()],
                y,
                x,
                format!("{edr} - {w0}"),
            );
            match mount {
                // the backs stand on the bottom and under the top
                true => {
                    let into = [0, 0, down];
                    joint(
                        part,
                        "Back",
                        [inner_x.clone(), back_y.clone(), plane.clone()],
                        into,
                        x,
                        format!("{ea} - 2*{et} - {tm}"),
                    );
                    let run = format!("{eb} - {et} - {tm}");
                    if self.split {
                        let leg = format!("{eb} - {ed} - {et}");
                        joint(
                            &piece,
                            "Back return",
                            [back_x.clone(), et.clone(), plane.clone()],
                            into,
                            y,
                            leg,
                        );
                        joint(
                            part,
                            "Back return",
                            [back_x.clone(), front_y.clone(), plane.clone()],
                            into,
                            y,
                            format!("{ed} - {tm}"),
                        );
                    } else {
                        joint(
                            part,
                            "Back return",
                            [back_x.clone(), et.clone(), plane.clone()],
                            into,
                            y,
                            run,
                        );
                    }
                }
                false => {
                    let wall_y = format!("{eb} - {et}");
                    joint(
                        "Back",
                        part,
                        [et.clone(), wall_y, mid.clone()],
                        [0, -1, 0],
                        x,
                        format!("{ea} - 2*{et}"),
                    );
                    if self.split {
                        let leg = format!("{eb} - {ed} - {et}");
                        joint(
                            "Back return",
                            &piece,
                            [et.clone(), et.clone(), mid.clone()],
                            x,
                            y,
                            leg,
                        );
                        joint(
                            "Back return",
                            part,
                            [et.clone(), front_y.clone(), mid.clone()],
                            x,
                            y,
                            format!("{ed} - {et}"),
                        );
                    } else {
                        joint(
                            "Back return",
                            part,
                            [et.clone(), et.clone(), mid.clone()],
                            x,
                            y,
                            format!("{eb} - 2*{et}"),
                        );
                    }
                }
            }
        }
    }

    /// The faces that carry the shelves: each end and each back, with the first pin row's place
    /// on its inner face (37 mm from the corners of a leg, x before `point`), the way into the
    /// board, the way to the second row, and the distance the two rows run over.
    fn shelf_faces(&self) -> Vec<ShelfFace> {
        let Corner { ea, eb, ed, et, .. } = self;
        let flip = self.flip();
        let (back_end, return_end) = self.ends();
        // Behind inset doors the shelves, so the rows, end a panel further back.
        let (front, o) = match self.inset {
            true => (format!("{eb} - {ed} + {et}"), format!(" - {et}")),
            false => (format!("{eb} - {ed}"), String::new()),
        };
        // With a mount board the backs, so their rows, stand off the walls by its thickness
        // and the rows are that much shorter.
        let (o, back_x, back_y) = match &self.emount {
            Some([tm, _]) => (
                format!("{o} - {tm}"),
                format!("{et} + {tm}"),
                format!("{eb} - {et} - {tm}"),
            ),
            None => (o, et.clone(), format!("{eb} - {et}")),
        };
        let (near, far) = (
            format!("{back_x} + {}", mm(PIN_SETBACK)),
            format!("{front} + {}", mm(PIN_SETBACK)),
        );
        let wall_x = format!("{ea} - {et}");
        let (run, return_run) = (format!("{ed} - {et}{o}"), format!("{} - {et}{o}", self.edr));
        vec![
            (
                back_end.to_string(),
                [wall_x, far.clone()],
                [flip, 0, 0],
                [0, 1, 0],
                run.clone(),
            ),
            (
                return_end.to_string(),
                [near.clone(), et.clone()],
                [0, -1, 0],
                [flip, 0, 0],
                return_run,
            ),
            (
                "Back".to_string(),
                [near, back_y],
                [0, 1, 0],
                [flip, 0, 0],
                run.clone(),
            ),
            (
                "Back return".to_string(),
                [back_x, far],
                [-flip, 0, 0],
                [0, 1, 0],
                run,
            ),
        ]
    }

    /// Like the box cabinet's: one blind hole per panel, repeated up it and across to a
    /// second row. Two rows in each end and in each back.
    fn pins(&self, top_row: &str) -> Commands {
        let Corner { kit, et, epl, .. } = self;
        let blind = fittings::blind_pin(et);
        let z = format!("{epl} + {et} + {}", mm(2.0 * PITCH));
        let mut drill = Vec::new();
        for (part, [x, y], into, across, run) in self.shelf_faces() {
            let rows = PinRows {
                part: &part,
                position: kit.at(&self.point(&x), &y, &z),
                into,
                depth: &blind,
                across,
                run: &run,
            };
            drill.extend(fittings::pin_rows(kit, top_row, rows));
        }
        fittings::shared_pin_rows(kit, drill)
    }

    /// Pilaster strips (#264), as the box cabinet's: on each face that carries shelves a dado
    /// `STRIP` wide and deep on each pin row's line (the one by the opening `STRIP_SETBACK` in)
    /// over the shelves' height, a strip model in
    /// it, and under each shelf (`shelves`, their lower faces) a clip at the first strip and a
    /// pattern of it to the second. `length` is the strips' in mm, for their names.
    fn strips(&self, zone: &str, length: f64, shelves: &[(String, String)]) {
        let Corner { kit, et, epl, .. } = self;
        let [w, deep, out, high] = [STRIP[0], STRIP[1], CLIP[0], CLIP[1]].map(mm);
        let base = format!("{epl} + {et}");
        for (part, [x, y], into, across, run) in self.shelf_faces() {
            let first = [self.point(&x), y];
            let a = usize::from(into[0] == 0); // the axis into the board; `1 - a` is along the face
                                               // The row by the opening stands `STRIP_SETBACK` in, clear of the hinge plates: a
                                               // face along y starts at the front (first row), one along x at the wall (second).
            let back = mm(STRIP_SETBACK - PIN_SETBACK);
            let shift = if across[1] != 0 {
                back.clone()
            } else {
                mm(0.0)
            };
            let spacing = format!("{run} - {} - {back}", mm(2.0 * PIN_SETBACK));
            let target = [format!("{} {part}", kit.name)];
            // one axis's low end and size: into the board, out of it (a clip), or across a row
            let span = |at: &str, axis: usize, row: usize, way: i32, size: &str| match axis == a {
                true if way > 0 => (at.to_string(), size.to_string()),
                true => (format!("{at} - {size}"), size.to_string()),
                false => {
                    let sign = if across[axis] < 0 { "-" } else { "+" };
                    let centre = format!("{at} {sign} ({shift} + {row}*({spacing}))");
                    (format!("{centre} - {w}/2"), w.clone())
                }
            };
            let rows = [("front", 0), ("back", 1)];
            for (name, row) in rows {
                let [(x0, sx), (y0, sy)] =
                    [0, 1].map(|axis| span(&first[axis], axis, row, into[a], &deep));
                let (corner, size) = ([x0.as_str(), &y0, &base], [sx.as_str(), &sy, zone]);
                kit.panel(
                    &format!("{part} pilaster dado {name}"),
                    corner,
                    size,
                    Some(&target),
                );
                kit.model(
                    &format!("{part} pilaster {name} ({length} mm)"),
                    corner,
                    size,
                );
            }
            for (shelf, z) in shelves {
                let clip = format!("{shelf} {part} clip");
                let [(x0, sx), (y0, sy)] =
                    [0, 1].map(|axis| span(&first[axis], axis, 0, -into[a], &out));
                let z0 = format!("{z} - {high}");
                kit.model(&clip, [&x0, &y0, &z0], [&sx, &sy, &high]);
                kit.push((
                    "solid.pattern.rectangular",
                    json!({
                        "bodies": [format!("{} {clip}", kit.name)], "name": format!("{} {clip}s", kit.name),
                        "dir1": across, "count1": 2, "spacing1": spacing,
                    }),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests;
