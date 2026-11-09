//! The chart builder.

use std::f64::consts::PI;

use crate::scale::{clean, format_tick, nice_bounds, Scale};
use crate::shape::{to_svg, Anchor, PathCmd, Role, Shape, Theme};

/// The kinds of chart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChartKind {
    /// Lines through the values, with axes.
    Line,
    /// Filled areas under the lines, with axes.
    Area,
    /// Vertical bars, grouped (or stacked) per label, with axes.
    Bar,
    /// A small line with no axes, for inline trends.
    Sparkline,
    /// A ring split into slices.
    Donut,
    /// A 270° arc filled up to a value.
    Gauge,
}

/// A named list of values.
#[derive(Clone, Debug, PartialEq)]
pub struct Series {
    /// Shown in the legend.
    pub name: String,
    /// The data. Non-finite values are skipped.
    pub values: Vec<f64>,
}

/// A chart, built with chainable methods and turned into [`Shape`]s or SVG.
///
/// ```
/// use weaver_charts::Chart;
///
/// let svg = Chart::line()
///     .size(480.0, 240.0)
///     .labels(["Mon", "Tue", "Wed", "Thu", "Fri"])
///     .series("Requests", [120.0, 180.0, 150.0, 220.0, 260.0])
///     .series("Errors", [4.0, 9.0, 3.0, 12.0, 6.0])
///     .smooth(true)
///     .to_svg();
/// assert!(svg.starts_with("<svg"));
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Chart {
    kind: ChartKind,
    width: f64,
    height: f64,
    series: Vec<Series>,
    labels: Vec<String>,
    smooth: bool,
    points: bool,
    stacked: bool,
    legend: bool,
    ticks: usize,
    y_range: Option<(f64, f64)>,
    value: f64,
    min: f64,
    max: f64,
    unit: String,
    decimals: usize,
}

impl Chart {
    fn new(kind: ChartKind) -> Self {
        Chart {
            kind,
            width: 400.0,
            height: 220.0,
            series: Vec::new(),
            labels: Vec::new(),
            smooth: false,
            points: false,
            stacked: false,
            legend: true,
            ticks: 4,
            y_range: None,
            value: 0.0,
            min: 0.0,
            max: 100.0,
            unit: String::new(),
            decimals: 0,
        }
    }

    /// A line chart.
    pub fn line() -> Self {
        Chart::new(ChartKind::Line)
    }

    /// An area chart.
    pub fn area() -> Self {
        Chart::new(ChartKind::Area)
    }

    /// A bar chart.
    pub fn bar() -> Self {
        Chart::new(ChartKind::Bar)
    }

    /// A sparkline (no axes, no legend).
    pub fn sparkline(values: impl IntoIterator<Item = f64>) -> Self {
        Chart::new(ChartKind::Sparkline)
            .size(120.0, 32.0)
            .series("", values)
            .legend(false)
    }

    /// A donut. Add slices with [`slice`](Self::slice).
    pub fn donut() -> Self {
        Chart::new(ChartKind::Donut).size(220.0, 220.0)
    }

    /// A gauge showing `value` between `min` and `max`.
    pub fn gauge(value: f64, min: f64, max: f64) -> Self {
        let mut chart = Chart::new(ChartKind::Gauge).size(200.0, 200.0);
        chart.value = value;
        chart.min = min;
        chart.max = max;
        chart
    }

    /// The kind of chart.
    pub fn kind(&self) -> ChartKind {
        self.kind
    }

    /// Output size.
    pub fn size(mut self, width: f64, height: f64) -> Self {
        self.width = width.max(1.0);
        self.height = height.max(1.0);
        self
    }

    /// Output width and height.
    pub fn dimensions(&self) -> (f64, f64) {
        (self.width, self.height)
    }

    /// Add a series.
    pub fn series(
        mut self,
        name: impl Into<String>,
        values: impl IntoIterator<Item = f64>,
    ) -> Self {
        self.series.push(Series {
            name: name.into(),
            values: values.into_iter().collect(),
        });
        self
    }

    /// Add one donut slice.
    pub fn slice(mut self, label: impl Into<String>, value: f64) -> Self {
        if self.series.is_empty() {
            self.series.push(Series {
                name: String::new(),
                values: Vec::new(),
            });
        }
        self.series[0].values.push(value);
        self.labels.push(label.into());
        self
    }

    /// Category labels along the x axis (or slice names for a donut).
    pub fn labels<S: Into<String>>(mut self, labels: impl IntoIterator<Item = S>) -> Self {
        self.labels = labels.into_iter().map(Into::into).collect();
        self
    }

    /// Curve lines smoothly through the points (monotone, so the curve
    /// never overshoots the data).
    pub fn smooth(mut self, smooth: bool) -> Self {
        self.smooth = smooth;
        self
    }

    /// Mark every data point with a dot.
    pub fn points(mut self, points: bool) -> Self {
        self.points = points;
        self
    }

    /// Stack bars (and areas) instead of placing them side by side.
    pub fn stacked(mut self, stacked: bool) -> Self {
        self.stacked = stacked;
        self
    }

    /// Show a legend when there is more than one series.
    pub fn legend(mut self, legend: bool) -> Self {
        self.legend = legend;
        self
    }

    /// About how many y-axis ticks to draw.
    pub fn ticks(mut self, ticks: usize) -> Self {
        self.ticks = ticks.max(1);
        self
    }

    /// Fix the y range instead of fitting it to the data.
    pub fn y_range(mut self, min: f64, max: f64) -> Self {
        self.y_range = Some((min, max));
        self
    }

    /// A unit appended to the gauge value, such as `"%"` or `" ms"`.
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = unit.into();
        self
    }

    /// Decimal places for the gauge value (default 0).
    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals.min(6);
        self
    }

    /// The series, in order.
    pub fn all_series(&self) -> &[Series] {
        &self.series
    }

    /// Replace the values of series `index` (used by animation).
    pub fn set_values(&mut self, index: usize, values: Vec<f64>) {
        if let Some(series) = self.series.get_mut(index) {
            series.values = values;
        }
    }

    /// The gauge value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Change the gauge value.
    pub fn set_value(&mut self, value: f64) {
        self.value = value;
    }

    /// SVG with the light theme.
    pub fn to_svg(&self) -> String {
        self.to_svg_with(&Theme::light())
    }

    /// SVG with your theme.
    pub fn to_svg_with(&self, theme: &Theme) -> String {
        to_svg(&self.shapes(), self.width, self.height, theme)
    }

    /// The chart as plain shapes, for drawing with any renderer.
    pub fn shapes(&self) -> Vec<Shape> {
        match self.kind {
            ChartKind::Line | ChartKind::Area | ChartKind::Bar => self.cartesian(),
            ChartKind::Sparkline => self.sparkline_shapes(),
            ChartKind::Donut => self.donut_shapes(),
            ChartKind::Gauge => self.gauge_shapes(),
        }
    }

    fn max_len(&self) -> usize {
        self.series
            .iter()
            .map(|s| s.values.len())
            .max()
            .unwrap_or(0)
            .max(self.labels.len())
    }

    fn data_bounds(&self) -> (f64, f64) {
        if let Some(range) = self.y_range {
            return range;
        }
        let stack = self.stacked && matches!(self.kind, ChartKind::Bar | ChartKind::Area);
        let mut lo: f64 = 0.0;
        let mut hi: f64 = f64::MIN;
        if stack {
            for i in 0..self.max_len() {
                let total: f64 = self
                    .series
                    .iter()
                    .filter_map(|s| s.values.get(i).copied().filter(|v| v.is_finite()))
                    .sum();
                hi = hi.max(total);
            }
        } else {
            for v in self.series.iter().flat_map(|s| &s.values) {
                if v.is_finite() {
                    lo = lo.min(*v);
                    hi = hi.max(*v);
                }
            }
        }
        if hi == f64::MIN {
            hi = 1.0;
        }
        (lo, hi)
    }

    fn cartesian(&self) -> Vec<Shape> {
        let mut shapes = Vec::new();
        let show_legend = self.legend && self.series.len() > 1;
        let top = if show_legend { 34.0 } else { 14.0 };
        let bottom = if self.labels.is_empty() { 14.0 } else { 30.0 };
        let (lo, hi) = self.data_bounds();
        let (lo, hi, step) = if self.y_range.is_some() {
            let (_, _, step) = nice_bounds(lo, hi, self.ticks);
            (lo, hi, step)
        } else {
            nice_bounds(lo, hi, self.ticks)
        };
        let tick_values: Vec<f64> = {
            let mut v = Vec::new();
            let mut t = (lo / step).ceil() * step;
            while t <= hi + step * 1e-9 {
                v.push(clean(t, step));
                t += step;
            }
            v
        };
        let label_width = tick_values
            .iter()
            .map(|t| format_tick(*t).len())
            .max()
            .unwrap_or(1) as f64
            * 7.0;
        let left = 12.0 + label_width;
        let right = if self.labels.is_empty() { 14.0 } else { 24.0 };
        let plot_x = left;
        let plot_w = (self.width - left - right).max(1.0);
        let plot_y = top;
        let plot_h = (self.height - top - bottom).max(1.0);
        let y = Scale::new(lo, hi, plot_y + plot_h, plot_y);

        for t in &tick_values {
            let py = y.map(*t);
            shapes.push(Shape::Line {
                x1: plot_x,
                y1: py,
                x2: plot_x + plot_w,
                y2: py,
                role: Role::Grid,
            });
            shapes.push(Shape::Text {
                x: plot_x - 8.0,
                y: py + 4.0,
                text: format_tick(*t),
                size: 11.0,
                anchor: Anchor::End,
                role: Role::Label,
            });
        }
        shapes.push(Shape::Line {
            x1: plot_x,
            y1: y.map(lo.max(0.0).min(hi)),
            x2: plot_x + plot_w,
            y2: y.map(lo.max(0.0).min(hi)),
            role: Role::Axis,
        });

        let n = self.max_len().max(1);
        let is_bar = self.kind == ChartKind::Bar;
        let slot = plot_w / n as f64;
        let x_at = |i: usize| -> f64 {
            if is_bar || n == 1 {
                plot_x + slot * (i as f64 + 0.5)
            } else {
                plot_x + plot_w * i as f64 / (n - 1) as f64
            }
        };
        for (i, label) in self.labels.iter().enumerate() {
            shapes.push(Shape::Text {
                x: x_at(i),
                y: plot_y + plot_h + 20.0,
                text: label.clone(),
                size: 11.0,
                anchor: Anchor::Middle,
                role: Role::Label,
            });
        }

        match self.kind {
            ChartKind::Bar => self.bars(&mut shapes, &y, slot, &x_at),
            _ => self.lines(&mut shapes, &y, &x_at),
        }

        if show_legend {
            let mut x = plot_x;
            for (i, s) in self.series.iter().enumerate() {
                shapes.push(Shape::Circle {
                    cx: x + 5.0,
                    cy: 14.0,
                    r: 5.0,
                    role: Role::Point(i),
                });
                shapes.push(Shape::Text {
                    x: x + 15.0,
                    y: 18.0,
                    text: s.name.clone(),
                    size: 12.0,
                    anchor: Anchor::Start,
                    role: Role::Label,
                });
                x += 30.0 + s.name.chars().count() as f64 * 7.0;
            }
        }
        shapes
    }

    fn lines(&self, shapes: &mut Vec<Shape>, y: &Scale, x_at: &dyn Fn(usize) -> f64) {
        let stacked = self.stacked && self.kind == ChartKind::Area;
        let mut base = vec![0.0; self.max_len()];
        for (si, s) in self.series.iter().enumerate() {
            let pts: Vec<(f64, f64)> = s
                .values
                .iter()
                .enumerate()
                .filter(|(_, v)| v.is_finite())
                .map(|(i, v)| {
                    let v = if stacked { base[i] + v } else { *v };
                    (x_at(i), y.map(v))
                })
                .collect();
            if pts.is_empty() {
                continue;
            }
            let line = if self.smooth {
                monotone_path(&pts)
            } else {
                polyline(&pts)
            };
            if self.kind == ChartKind::Area {
                let mut area = line.clone();
                let floor: Vec<(f64, f64)> = if stacked {
                    s.values
                        .iter()
                        .enumerate()
                        .filter(|(_, v)| v.is_finite())
                        .map(|(i, _)| (x_at(i), y.map(base[i])))
                        .rev()
                        .collect()
                } else {
                    let zero = y.map(y.domain_min.max(0.0));
                    vec![(pts[pts.len() - 1].0, zero), (pts[0].0, zero)]
                };
                for (x, fy) in floor {
                    area.push(PathCmd::LineTo(x, fy));
                }
                area.push(PathCmd::Close);
                shapes.push(Shape::Path {
                    cmds: area,
                    role: Role::SeriesFill(si),
                    stroke_width: 0.0,
                    opacity: 0.22,
                });
            }
            shapes.push(Shape::Path {
                cmds: line,
                role: Role::SeriesStroke(si),
                stroke_width: 2.5,
                opacity: 1.0,
            });
            if self.points {
                for &(px, py) in &pts {
                    shapes.push(Shape::Circle {
                        cx: px,
                        cy: py,
                        r: 3.5,
                        role: Role::Point(si),
                    });
                }
            }
            if stacked {
                for (i, v) in s.values.iter().enumerate() {
                    if v.is_finite() {
                        base[i] += v;
                    }
                }
            }
        }
    }

    fn bars(&self, shapes: &mut Vec<Shape>, y: &Scale, slot: f64, x_at: &dyn Fn(usize) -> f64) {
        let groups = self.series.len().max(1);
        let group_w = slot * 0.7;
        let bar_w = if self.stacked {
            group_w
        } else {
            group_w / groups as f64
        };
        let zero = y.domain_min.max(0.0).min(y.domain_max);
        let mut base = vec![zero; self.max_len()];
        for (si, s) in self.series.iter().enumerate() {
            for (i, v) in s.values.iter().enumerate() {
                if !v.is_finite() {
                    continue;
                }
                let (from, to) = if self.stacked {
                    (base[i], base[i] + v)
                } else {
                    (zero, *v)
                };
                let x = if self.stacked {
                    x_at(i) - group_w / 2.0
                } else {
                    x_at(i) - group_w / 2.0 + bar_w * si as f64
                };
                let (y1, y2) = (y.map(from), y.map(to));
                shapes.push(Shape::Rect {
                    x: x + 1.0,
                    y: y1.min(y2),
                    width: (bar_w - 2.0).max(1.0),
                    height: (y1 - y2).abs(),
                    radius: 3.0_f64.min(bar_w / 4.0),
                    role: Role::SeriesFill(si),
                });
                if self.stacked {
                    base[i] += v;
                }
            }
        }
    }

    fn sparkline_shapes(&self) -> Vec<Shape> {
        let Some(s) = self.series.first() else {
            return Vec::new();
        };
        let values: Vec<f64> = s.values.iter().copied().filter(|v| v.is_finite()).collect();
        if values.is_empty() {
            return Vec::new();
        }
        let (lo, hi) = self.y_range.unwrap_or_else(|| {
            values
                .iter()
                .fold((f64::MAX, f64::MIN), |(a, b), v| (a.min(*v), b.max(*v)))
        });
        let pad = 3.0;
        let y = Scale::new(lo, hi, self.height - pad, pad);
        let n = values.len();
        let pts: Vec<(f64, f64)> = values
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let x = if n == 1 {
                    self.width / 2.0
                } else {
                    pad + (self.width - 2.0 * pad) * i as f64 / (n - 1) as f64
                };
                (x, y.map(*v))
            })
            .collect();
        let line = if self.smooth {
            monotone_path(&pts)
        } else {
            polyline(&pts)
        };
        let last = pts[pts.len() - 1];
        vec![
            Shape::Path {
                cmds: line,
                role: Role::SeriesStroke(0),
                stroke_width: 2.0,
                opacity: 1.0,
            },
            Shape::Circle {
                cx: last.0,
                cy: last.1,
                r: 2.5,
                role: Role::Point(0),
            },
        ]
    }

    fn donut_shapes(&self) -> Vec<Shape> {
        let mut shapes = Vec::new();
        let values: Vec<f64> = self
            .series
            .first()
            .map(|s| s.values.iter().map(|v| v.max(0.0)).collect())
            .unwrap_or_default();
        let total: f64 = values.iter().filter(|v| v.is_finite()).sum();
        let cx = self.width / 2.0;
        let cy = self.height / 2.0;
        let outer = self.width.min(self.height) / 2.0 - 6.0;
        let inner = outer * 0.62;
        if total <= 0.0 {
            shapes.push(ring(cx, cy, outer, inner, -PI / 2.0, 1.5 * PI, Role::Track));
            return shapes;
        }
        let gap = if values.len() > 1 { 0.025 } else { 0.0 };
        let mut angle = -PI / 2.0;
        for (i, v) in values.iter().enumerate() {
            if !v.is_finite() || *v <= 0.0 {
                continue;
            }
            let sweep = 2.0 * PI * v / total;
            let (a0, a1) = (angle + gap / 2.0, angle + sweep - gap / 2.0);
            if a1 > a0 {
                shapes.push(ring(cx, cy, outer, inner, a0, a1, Role::SeriesFill(i)));
            }
            angle += sweep;
        }
        // Headline: the largest slice as a share of the whole.
        if let Some((index, top)) = values
            .iter()
            .enumerate()
            .filter(|(_, v)| v.is_finite())
            .max_by(|a, b| a.1.total_cmp(b.1))
        {
            let share = (top / total * 100.0).round();
            shapes.push(Shape::Text {
                x: cx,
                y: cy + outer * 0.08,
                text: format!("{share}%"),
                size: (outer * 0.3).round(),
                anchor: Anchor::Middle,
                role: Role::Value,
            });
            if let Some(label) = self.labels.get(index) {
                shapes.push(Shape::Text {
                    x: cx,
                    y: cy + outer * 0.08 + outer * 0.26,
                    text: label.clone(),
                    size: (outer * 0.14).round().max(10.0),
                    anchor: Anchor::Middle,
                    role: Role::Label,
                });
            }
        }
        shapes
    }

    fn gauge_shapes(&self) -> Vec<Shape> {
        let cx = self.width / 2.0;
        let cy = self.height / 2.0 + self.height * 0.04;
        let r = self.width.min(self.height) / 2.0 - 12.0;
        let thickness = r * 0.2;
        let start = 0.75 * PI;
        let sweep = 1.5 * PI;
        let span = self.max - self.min;
        let t = if span.abs() < f64::EPSILON {
            0.0
        } else {
            ((self.value - self.min) / span).clamp(0.0, 1.0)
        };
        let mut shapes = vec![arc_stroke(
            cx,
            cy,
            r,
            start,
            start + sweep,
            thickness,
            Role::Track,
        )];
        if t > 0.0 {
            shapes.push(arc_stroke(
                cx,
                cy,
                r,
                start,
                start + sweep * t,
                thickness,
                Role::SeriesStroke(0),
            ));
        }
        shapes.push(Shape::Text {
            x: cx,
            y: cy + r * 0.12,
            text: format!("{:.*}{}", self.decimals, self.value, self.unit),
            size: (r * 0.42).round(),
            anchor: Anchor::Middle,
            role: Role::Value,
        });
        if let Some(label) = self.labels.first() {
            shapes.push(Shape::Text {
                x: cx,
                y: cy + r * 0.12 + r * 0.32,
                text: label.clone(),
                size: (r * 0.16).round().max(10.0),
                anchor: Anchor::Middle,
                role: Role::Label,
            });
        }
        shapes
    }
}

fn polyline(pts: &[(f64, f64)]) -> Vec<PathCmd> {
    let mut cmds = vec![PathCmd::MoveTo(pts[0].0, pts[0].1)];
    cmds.extend(pts[1..].iter().map(|&(x, y)| PathCmd::LineTo(x, y)));
    cmds
}

/// Monotone cubic interpolation (Fritsch–Carlson): smooth, and never
/// overshoots between two points.
fn monotone_path(pts: &[(f64, f64)]) -> Vec<PathCmd> {
    let n = pts.len();
    if n < 3 {
        return polyline(pts);
    }
    let dx: Vec<f64> = (0..n - 1).map(|i| pts[i + 1].0 - pts[i].0).collect();
    let slope: Vec<f64> = (0..n - 1)
        .map(|i| {
            if dx[i] == 0.0 {
                0.0
            } else {
                (pts[i + 1].1 - pts[i].1) / dx[i]
            }
        })
        .collect();
    let mut tangent = vec![0.0; n];
    tangent[0] = slope[0];
    tangent[n - 1] = slope[n - 2];
    for i in 1..n - 1 {
        tangent[i] = if slope[i - 1] * slope[i] <= 0.0 {
            0.0
        } else {
            (slope[i - 1] + slope[i]) / 2.0
        };
    }
    for i in 0..n - 1 {
        if slope[i] == 0.0 {
            tangent[i] = 0.0;
            tangent[i + 1] = 0.0;
            continue;
        }
        let a = tangent[i] / slope[i];
        let b = tangent[i + 1] / slope[i];
        let h = a * a + b * b;
        if h > 9.0 {
            let t = 3.0 / h.sqrt();
            tangent[i] = t * a * slope[i];
            tangent[i + 1] = t * b * slope[i];
        }
    }
    let mut cmds = vec![PathCmd::MoveTo(pts[0].0, pts[0].1)];
    for i in 0..n - 1 {
        let (x0, y0) = pts[i];
        let (x1, y1) = pts[i + 1];
        let d = dx[i] / 3.0;
        cmds.push(PathCmd::CubicTo(
            x0 + d,
            y0 + tangent[i] * d,
            x1 - d,
            y1 - tangent[i + 1] * d,
            x1,
            y1,
        ));
    }
    cmds
}

fn ring(cx: f64, cy: f64, outer: f64, inner: f64, a0: f64, a1: f64, role: Role) -> Shape {
    Shape::Path {
        cmds: vec![
            PathCmd::Arc {
                cx,
                cy,
                r: outer,
                start: a0,
                end: a1,
            },
            PathCmd::Arc {
                cx,
                cy,
                r: inner,
                start: a1,
                end: a0,
            },
            PathCmd::Close,
        ],
        role,
        stroke_width: 0.0,
        opacity: 1.0,
    }
}

fn arc_stroke(cx: f64, cy: f64, r: f64, a0: f64, a1: f64, width: f64, role: Role) -> Shape {
    Shape::Path {
        cmds: vec![PathCmd::Arc {
            cx,
            cy,
            r,
            start: a0,
            end: a1,
        }],
        role,
        stroke_width: width,
        opacity: 1.0,
    }
}
