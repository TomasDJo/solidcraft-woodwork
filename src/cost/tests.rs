use super::*;

fn list() -> Value {
    json!({
        "sheets": [
            {"material": "White melamine", "thickness_mm": 18.0, "sheets": 3},
            {"material": "White melamine", "thickness_mm": 6.0, "sheets": 1},
        ],
        "banding": [{"metres": 12.5}, {"metres": 2.0}],
        "hardware": [
            {"item": "Hinge, 110 degrees", "qty": 4},
            {"item": "Drawer slide, full extension, 450 mm, pair", "qty": 3},
            {"item": "Shelf pin", "qty": 16},
        ],
    })
}

#[test]
fn prices_are_name_equals_price_entries() {
    let p = prices("White melamine 18.0 = 42.50;  hinge=3,2\n\n Banding = 0.6 ").unwrap();
    assert_eq!(
        p,
        [
            ("white melamine 18".to_string(), 42.5),
            ("hinge".to_string(), 3.2),
            ("banding".to_string(), 0.6)
        ]
    );
    assert!(prices("Hinge 3.20").unwrap_err().contains("name = price"));
    assert!(prices("Hinge = cheap").unwrap_err().contains("not a price"));
    assert!(prices("Hinge = -1").is_err());
    assert_eq!(prices(" ; ").unwrap(), []);
}

#[test]
fn each_sheet_size_banding_and_hardware_line_is_priced() {
    let text = "White melamine 18 = 42.50; Banding = 0.60; Hinge = 3.20; Drawer slide = 9; \
                Drawer slide, full extension = 12";
    let cost = estimate(&list(), text).unwrap();
    let total = |i: usize| cost["lines"][i]["total"].clone();
    assert_eq!(total(0), json!(127.5)); // 3 sheets of 18 mm
    assert!(cost["lines"][1].get("price").is_none()); // the 6 mm sheet is another sheet
    assert_eq!(total(2), json!(8.7)); // 14.5 m of banding
    assert_eq!(total(3), json!(12.8)); // 4 hinges
    assert_eq!(total(4), json!(36.0)); // the longest matching name: full extension slides
    assert_eq!(cost["total"], json!(127.5 + 8.7 + 12.8 + 36.0));
    // what is left to price, ready to copy into Prices
    assert_eq!(
        cost["missing"],
        json!(["White melamine 6 = ?", "Shelf pin = ?"])
    );
    // no prices: every line asked for
    let none = estimate(&list(), "").unwrap();
    assert_eq!(none["total"], json!(0.0));
    assert_eq!(none["missing"].as_array().unwrap().len(), 6);
}

#[test]
fn timber_is_priced_per_running_metre_of_its_section() {
    let list = json!({"timber_metres": [
        {"thickness_mm": 48.0, "width_mm": 98.0, "metres": 4.6},
        {"thickness_mm": 38.0, "width_mm": 89.0, "metres": 2.0},
    ]});
    let cost = estimate(&list, "Timber 48x98 = 3.5; Timber 38 x 89 = 2").unwrap();
    let line = |i: usize| {
        (
            cost["lines"][i]["item"].clone(),
            cost["lines"][i]["unit"].clone(),
            cost["lines"][i]["total"].clone(),
        )
    };
    assert_eq!(line(0), (json!("Timber 48x98"), json!("m"), json!(16.1)));
    assert_eq!(line(1), (json!("Timber 38x89"), json!("m"), json!(4.0)));
    // without a price it is asked for like the rest
    let none = estimate(&list, "Hinge = 3").unwrap();
    assert_eq!(
        none["missing"],
        json!(["Timber 48x98 = ?", "Timber 38x89 = ?"])
    );
}

#[test]
fn the_csv_section_lists_each_line_the_total_and_what_has_no_price() {
    let cost = estimate(&list(), "White melamine 18 = 42.50; Hinge = 3.20").unwrap();
    let csv = csv(&cost);
    assert!(csv.starts_with("\nCost,Qty,Unit,Price,Total\n"), "{csv}");
    assert!(
        csv.contains("\"White melamine 18\",3,sheets,42.5,127.5\n"),
        "{csv}"
    );
    assert!(csv.contains("\"Shelf pin\",16,pcs,,\n"), "{csv}");
    assert!(
        csv.contains("Total,,,,140.3\n") && csv.contains("\"No price: Banding = ?\"\n"),
        "{csv}"
    );
}

#[test]
fn totals_add_up_banding_keeps_its_length_and_csv_quotes() {
    // two lines of 0.125, each shown as 0.13: the total is 0.26 with the banding's
    let list = json!({"sheets": [], "banding": [{"metres": 14.504}],
        "hardware": [{"item": "Knob", "qty": 1}, {"item": "Knob B", "qty": 1}]});
    let cost = estimate(&list, "Knob = 0.125; Banding = 20").unwrap();
    assert_eq!(cost["lines"][0]["total"], json!(290.08)); // 14.504 m, not rounded first
    assert_eq!(cost["total"], json!(290.34));
    assert_eq!(cost["given"], json!(true));
    assert_eq!(estimate(&list, " ").unwrap()["given"], json!(false));
    let odd = json!({"sheets": [{"material": "Oak\",offcut", "thickness_mm": 18.0, "sheets": 1}]});
    let csv = csv(&estimate(&odd, "Knob = 1").unwrap());
    assert!(
        csv.contains("\"No price: Oak\"\",offcut 18 = ?\"\n"),
        "{csv}"
    );
}

#[test]
fn one_sheet_is_a_sheet() {
    // the PDF reads "1 sheet", "3 sheets"
    let cost = estimate(&list(), "").unwrap();
    assert_eq!(cost["lines"][0]["unit"], "sheets"); // 3 of 18 mm
    assert_eq!(cost["lines"][1]["unit"], "sheet"); // 1 of 6 mm
}
