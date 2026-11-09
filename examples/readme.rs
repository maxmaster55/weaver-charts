//! Renders the README animation: a small dashboard whose charts spring to
//! new data every second.
//!
//! ```sh
//! cargo run --example readme --features motion -- /tmp/frames
//! convert -delay 3 -loop 0 /tmp/frames/*.svg -layers Optimize docs/dashboard.gif
//! ```

use std::{env, fs, path::PathBuf};

use weaver_charts::{AnimatedChart, Chart, Theme};

const W: f64 = 720.0;
const H: f64 = 400.0;

const TRAFFIC: [[f64; 8]; 3] = [
    [120.0, 180.0, 150.0, 220.0, 260.0, 210.0, 300.0, 280.0],
    [200.0, 160.0, 240.0, 190.0, 230.0, 320.0, 270.0, 340.0],
    [90.0, 140.0, 210.0, 260.0, 180.0, 230.0, 200.0, 250.0],
];
const ERRORS: [[f64; 8]; 3] = [
    [20.0, 35.0, 25.0, 50.0, 40.0, 30.0, 45.0, 38.0],
    [30.0, 22.0, 48.0, 36.0, 28.0, 60.0, 42.0, 55.0],
    [15.0, 28.0, 40.0, 30.0, 52.0, 34.0, 26.0, 44.0],
];
const REGIONS: [[f64; 4]; 3] = [
    [42.0, 28.0, 18.0, 12.0],
    [30.0, 35.0, 22.0, 13.0],
    [25.0, 25.0, 30.0, 20.0],
];
const CPU: [f64; 3] = [72.0, 38.0, 91.0];
const BARS: [[f64; 5]; 3] = [
    [12.0, 18.0, 15.0, 22.0, 9.0],
    [20.0, 11.0, 24.0, 16.0, 14.0],
    [8.0, 21.0, 13.0, 27.0, 19.0],
];

fn place(svg: String, x: f64, y: f64) -> String {
    svg.replacen("<svg ", &format!("<svg x=\"{x}\" y=\"{y}\" "), 1)
}

fn main() {
    let out = PathBuf::from(env::args().nth(1).unwrap_or_else(|| "target/frames".into()));
    fs::create_dir_all(&out).expect("create output directory");
    let card = Theme::dark().with_series(["#6c8cff", "#ff8a65", "#2fbf8f", "#f2b632", "#a77bf3"]);
    let card = Theme {
        background: Some("#161d2c".into()),
        ..card
    };

    let mut area = AnimatedChart::grow_in(
        Chart::area()
            .size(440.0, 220.0)
            .labels(["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun", "Mon"])
            .series("Requests", TRAFFIC[0])
            .series("Errors", ERRORS[0])
            .y_range(0.0, 400.0)
            .smooth(true),
    );
    let mut bars = AnimatedChart::grow_in(
        Chart::bar()
            .size(440.0, 150.0)
            .labels(["api", "web", "db", "cache", "queue"])
            .series("Deploys", BARS[0])
            .y_range(0.0, 30.0)
            .ticks(3),
    );
    let mut donut = AnimatedChart::grow_in(
        Chart::donut()
            .size(220.0, 180.0)
            .labels(["EU", "US", "Asia", "Other"])
            .series("Regions", REGIONS[0]),
    );
    let mut gauge = AnimatedChart::grow_in(
        Chart::gauge(CPU[0], 0.0, 100.0)
            .size(220.0, 190.0)
            .unit("%")
            .labels(["CPU"]),
    );

    let dt = 1.0 / 30.0;
    let mut frame = 0;
    for step in 1..=3 {
        for _ in 0..42 {
            area.advance(dt);
            bars.advance(dt);
            donut.advance(dt);
            gauge.advance(dt);
            let mut svg = format!(
                r##"<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}"><rect width="100%" height="100%" fill="#0f1420"/>"##
            );
            svg += &place(area.chart().to_svg_with(&card), 16.0, 16.0);
            svg += &place(bars.chart().to_svg_with(&card), 16.0, 236.0);
            svg += &place(donut.chart().to_svg_with(&card), 472.0, 16.0);
            svg += &place(gauge.chart().to_svg_with(&card), 472.0, 200.0);
            svg += "</svg>";
            fs::write(out.join(format!("{frame:04}.svg")), svg).unwrap();
            frame += 1;
        }
        let i = step % 3;
        area.set_series(0, TRAFFIC[i]);
        area.set_series(1, ERRORS[i]);
        bars.set_series(0, BARS[i]);
        donut.set_series(0, REGIONS[i]);
        gauge.set_value(CPU[i]);
    }
    println!("wrote {frame} frames to {}", out.display());
}
