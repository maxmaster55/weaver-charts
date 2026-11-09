use weaver_charts::*;

#[test]
fn nice_bounds_pick_round_numbers() {
    assert_eq!(nice_bounds(0.0, 97.0, 5), (0.0, 100.0, 20.0));
    assert!(nice_bounds(3.0, 3.0, 4).2 > 0.0);
    let mut s = Scale::new(0.0, 0.93, 100.0, 0.0);
    assert_eq!(s.nice(4), vec![0.0, 0.2, 0.4, 0.6, 0.8, 1.0]);
}

#[test]
fn tick_labels_are_compact() {
    assert_eq!(format_tick(1200.0), "1.2k");
    assert_eq!(format_tick(2_500_000.0), "2.5M");
    assert_eq!(format_tick(0.5), "0.5");
    assert_eq!(format_tick(40.0), "40");
    assert_eq!(format_tick(-0.0), "0");
}

#[test]
fn line_charts_draw_one_stroke_per_series_and_a_legend() {
    let chart = Chart::line()
        .series("a", [1.0, 2.0, 3.0])
        .series("b", [3.0, 2.0, 1.0])
        .points(true);
    let shapes = chart.shapes();
    let strokes = shapes
        .iter()
        .filter(|s| {
            matches!(
                s,
                Shape::Path {
                    role: Role::SeriesStroke(_),
                    ..
                }
            )
        })
        .count();
    assert_eq!(strokes, 2);
    let points = shapes
        .iter()
        .filter(|s| {
            matches!(
                s,
                Shape::Circle {
                    role: Role::Point(_),
                    ..
                }
            )
        })
        .count();
    assert_eq!(points, 6 + 2); // data points plus legend dots
}

#[test]
fn smooth_lines_pass_through_every_point() {
    let chart = Chart::line()
        .size(300.0, 200.0)
        .legend(false)
        .series("a", [0.0, 10.0, 0.0, 10.0])
        .smooth(true);
    let shapes = chart.shapes();
    let Some(Shape::Path { cmds, .. }) = shapes.iter().find(|s| {
        matches!(
            s,
            Shape::Path {
                role: Role::SeriesStroke(0),
                ..
            }
        )
    }) else {
        panic!("no line");
    };
    let ends: Vec<(f64, f64)> = cmds
        .iter()
        .map(|c| match *c {
            PathCmd::MoveTo(x, y) | PathCmd::LineTo(x, y) | PathCmd::CubicTo(_, _, _, _, x, y) => {
                (x, y)
            }
            _ => panic!("unexpected command"),
        })
        .collect();
    assert_eq!(ends.len(), 4);
    assert!(ends[1].1 < ends[0].1, "higher values are drawn higher up");
}

#[test]
fn bars_are_grouped_or_stacked() {
    let grouped = Chart::bar()
        .series("a", [1.0, 2.0])
        .series("b", [3.0, 4.0])
        .shapes();
    let rects: Vec<_> = grouped
        .iter()
        .filter(|s| matches!(s, Shape::Rect { .. }))
        .collect();
    assert_eq!(rects.len(), 4);

    let stacked = Chart::bar()
        .series("a", [1.0])
        .series("b", [3.0])
        .stacked(true)
        .legend(false)
        .shapes();
    let ys: Vec<(f64, f64)> = stacked
        .iter()
        .filter_map(|s| match s {
            Shape::Rect { y, height, .. } => Some((*y, *height)),
            _ => None,
        })
        .collect();
    assert_eq!(ys.len(), 2);
    assert!(
        (ys[1].0 + ys[1].1 - ys[0].0).abs() < 1e-6,
        "second bar sits on the first"
    );
}

#[test]
fn donut_and_gauge_headlines() {
    let donut = Chart::donut().slice("EU", 30.0).slice("US", 10.0);
    assert!(donut.to_svg().contains(">75%<"));
    assert!(donut.to_svg().contains(">EU<"));
    let gauge = Chart::gauge(150.0, 0.0, 100.0).unit("%");
    let svg = gauge.to_svg();
    assert!(svg.contains(">150%<"));
}

#[test]
fn non_finite_values_are_skipped() {
    let chart = Chart::line()
        .series("a", [1.0, f64::NAN, 3.0])
        .points(true)
        .legend(false);
    let points = chart
        .shapes()
        .iter()
        .filter(|s| matches!(s, Shape::Circle { .. }))
        .count();
    assert_eq!(points, 2);
    let empty = Chart::sparkline(Vec::new());
    assert!(empty.shapes().is_empty());
}

#[test]
fn svg_escapes_text() {
    let svg = Chart::bar().labels(["<b>&"]).series("x", [1.0]).to_svg();
    assert!(svg.contains("&lt;b&gt;&amp;"));
}

#[cfg(feature = "motion")]
#[test]
fn animated_charts_settle_exactly_on_new_data() {
    let mut chart = AnimatedChart::new(Chart::line().series("a", [1.0, 2.0]));
    chart.set_series(0, [5.0, 6.0, 7.0]);
    assert_eq!(chart.chart().all_series()[0].values.len(), 2);
    chart.advance(0.0);
    assert_eq!(chart.chart().all_series()[0].values.len(), 3);
    let mut frames = 0;
    while chart.advance(1.0 / 60.0) {
        frames += 1;
        assert!(frames < 600);
    }
    assert_eq!(chart.chart().all_series()[0].values, vec![5.0, 6.0, 7.0]);

    let mut gauge = AnimatedChart::grow_in(Chart::gauge(80.0, 0.0, 100.0));
    assert_eq!(gauge.chart().value(), 0.0);
    gauge.settle();
    assert_eq!(gauge.chart().value(), 80.0);
}
