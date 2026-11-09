//! The output of a chart: plain shapes any renderer can draw, plus a
//! built-in SVG writer.

use std::fmt::Write as _;

/// One step of a path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathCmd {
    /// Start a new sub-path.
    MoveTo(f64, f64),
    /// Straight line.
    LineTo(f64, f64),
    /// Cubic Bézier: two control points, then the end point.
    CubicTo(f64, f64, f64, f64, f64, f64),
    /// Circular arc around `(cx, cy)` with radius `r`, from `start` to `end`
    /// radians (0 is 3 o'clock, angles grow clockwise on screen).
    Arc {
        /// Centre x.
        cx: f64,
        /// Centre y.
        cy: f64,
        /// Radius.
        r: f64,
        /// Start angle in radians.
        start: f64,
        /// End angle in radians.
        end: f64,
    },
    /// Close the sub-path.
    Close,
}

/// What a shape represents, so a renderer can style it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    /// Background grid lines.
    Grid,
    /// Axis lines.
    Axis,
    /// Tick and category labels.
    Label,
    /// Data drawn as a stroke (line chart).
    SeriesStroke(usize),
    /// Data drawn as a fill (area, bar, slice).
    SeriesFill(usize),
    /// A data point marker.
    Point(usize),
    /// The unfilled part of a gauge or progress track.
    Track,
    /// The headline value of a gauge or donut.
    Value,
}

/// How text lines up with its anchor point.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    /// The anchor is at the start of the text.
    Start,
    /// The anchor is at the middle.
    Middle,
    /// The anchor is at the end.
    End,
}

/// One drawable thing.
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    /// A path, stroked or filled depending on its [`Role`].
    Path {
        /// The commands.
        cmds: Vec<PathCmd>,
        /// What it represents.
        role: Role,
        /// Stroke width (0 for fills).
        stroke_width: f64,
        /// Opacity from 0 to 1.
        opacity: f64,
    },
    /// An axis-aligned rectangle with rounded corners.
    Rect {
        /// Left.
        x: f64,
        /// Top.
        y: f64,
        /// Width.
        width: f64,
        /// Height.
        height: f64,
        /// Corner radius.
        radius: f64,
        /// What it represents.
        role: Role,
    },
    /// A circle.
    Circle {
        /// Centre x.
        cx: f64,
        /// Centre y.
        cy: f64,
        /// Radius.
        r: f64,
        /// What it represents.
        role: Role,
    },
    /// A straight line.
    Line {
        /// Start x.
        x1: f64,
        /// Start y.
        y1: f64,
        /// End x.
        x2: f64,
        /// End y.
        y2: f64,
        /// What it represents.
        role: Role,
    },
    /// A text label.
    Text {
        /// Anchor x.
        x: f64,
        /// Baseline y.
        y: f64,
        /// The text.
        text: String,
        /// Font size.
        size: f64,
        /// Alignment around `x`.
        anchor: Anchor,
        /// What it represents.
        role: Role,
    },
}

/// Colours used by [`to_svg`]. Bring your own palette with
/// [`Theme::with_series`].
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    /// Page background; `None` for transparent.
    pub background: Option<String>,
    /// Grid lines.
    pub grid: String,
    /// Axis lines.
    pub axis: String,
    /// Labels.
    pub label: String,
    /// Gauge track.
    pub track: String,
    /// Headline values.
    pub value: String,
    /// One colour per series, reused cyclically.
    pub series: Vec<String>,
    /// Font family for labels.
    pub font: String,
}

impl Theme {
    /// Light background.
    pub fn light() -> Self {
        Theme {
            background: Some("#ffffff".into()),
            grid: "#e8ebf2".into(),
            axis: "#c3c9d6".into(),
            label: "#6b7385".into(),
            track: "#e8ebf2".into(),
            value: "#1b2130".into(),
            series: DEFAULT_SERIES.iter().map(|s| s.to_string()).collect(),
            font: "DejaVu Sans, Helvetica, Arial, sans-serif".into(),
        }
    }

    /// Dark background.
    pub fn dark() -> Self {
        Theme {
            background: Some("#0f1420".into()),
            grid: "#1f2738".into(),
            axis: "#334059".into(),
            label: "#8d97ad".into(),
            track: "#1f2738".into(),
            value: "#eef2fb".into(),
            ..Theme::light()
        }
    }

    /// Replace the series colours.
    pub fn with_series<S: Into<String>>(mut self, colors: impl IntoIterator<Item = S>) -> Self {
        self.series = colors.into_iter().map(Into::into).collect();
        self
    }

    /// The colour of series `index`.
    pub fn series_color(&self, index: usize) -> &str {
        if self.series.is_empty() {
            "#6c8cff"
        } else {
            &self.series[index % self.series.len()]
        }
    }

    fn color(&self, role: Role) -> &str {
        match role {
            Role::Grid => &self.grid,
            Role::Axis => &self.axis,
            Role::Label => &self.label,
            Role::Track => &self.track,
            Role::Value => &self.value,
            Role::SeriesStroke(i) | Role::SeriesFill(i) | Role::Point(i) => self.series_color(i),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme::light()
    }
}

const DEFAULT_SERIES: [&str; 6] = [
    "#6c8cff", "#ff8a65", "#2fbf8f", "#f2b632", "#a77bf3", "#36b3e0",
];

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The SVG `d` attribute for a list of path commands.
pub fn path_data(cmds: &[PathCmd]) -> String {
    let mut d = String::new();
    for cmd in cmds {
        match *cmd {
            PathCmd::MoveTo(x, y) => {
                let _ = write!(d, "M{x:.2} {y:.2}");
            }
            PathCmd::LineTo(x, y) => {
                let _ = write!(d, "L{x:.2} {y:.2}");
            }
            PathCmd::CubicTo(a, b, c, e, x, y) => {
                let _ = write!(d, "C{a:.2} {b:.2} {c:.2} {e:.2} {x:.2} {y:.2}");
            }
            PathCmd::Arc {
                cx,
                cy,
                r,
                start,
                end,
            } => {
                // SVG arcs cannot draw a full circle in one go; split into
                // pieces of at most half a turn.
                let sweep = end - start;
                let pieces = (sweep.abs() / std::f64::consts::PI).ceil().max(1.0) as usize;
                let (sx, sy) = (cx + r * start.cos(), cy + r * start.sin());
                if d.is_empty() {
                    let _ = write!(d, "M{sx:.2} {sy:.2}");
                } else {
                    let _ = write!(d, "L{sx:.2} {sy:.2}");
                }
                for i in 1..=pieces {
                    let a = start + sweep * i as f64 / pieces as f64;
                    let (x, y) = (cx + r * a.cos(), cy + r * a.sin());
                    let flag = if sweep >= 0.0 { 1 } else { 0 };
                    let _ = write!(d, "A{r:.2} {r:.2} 0 0 {flag} {x:.2} {y:.2}");
                }
            }
            PathCmd::Close => d.push('Z'),
        }
    }
    d
}

/// Render shapes as a standalone SVG document.
pub fn to_svg(shapes: &[Shape], width: f64, height: f64, theme: &Theme) -> String {
    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}" font-family="{}">"#,
        escape(&theme.font)
    );
    if let Some(bg) = &theme.background {
        let _ = write!(svg, r#"<rect width="100%" height="100%" fill="{bg}"/>"#);
    }
    for shape in shapes {
        match shape {
            Shape::Path {
                cmds,
                role,
                stroke_width,
                opacity,
            } => {
                let color = theme.color(*role);
                let d = path_data(cmds);
                if *stroke_width > 0.0 {
                    let _ = write!(
                        svg,
                        r#"<path d="{d}" fill="none" stroke="{color}" stroke-width="{stroke_width}" stroke-linecap="round" stroke-linejoin="round" opacity="{opacity}"/>"#
                    );
                } else {
                    let _ = write!(svg, r#"<path d="{d}" fill="{color}" opacity="{opacity}"/>"#);
                }
            }
            Shape::Rect {
                x,
                y,
                width,
                height,
                radius,
                role,
            } => {
                let _ = write!(
                    svg,
                    r#"<rect x="{x:.2}" y="{y:.2}" width="{:.2}" height="{:.2}" rx="{radius:.2}" fill="{}"/>"#,
                    width.max(0.0),
                    height.max(0.0),
                    theme.color(*role)
                );
            }
            Shape::Circle { cx, cy, r, role } => {
                let _ = write!(
                    svg,
                    r#"<circle cx="{cx:.2}" cy="{cy:.2}" r="{r:.2}" fill="{}"/>"#,
                    theme.color(*role)
                );
            }
            Shape::Line {
                x1,
                y1,
                x2,
                y2,
                role,
            } => {
                let _ = write!(
                    svg,
                    r#"<line x1="{x1:.2}" y1="{y1:.2}" x2="{x2:.2}" y2="{y2:.2}" stroke="{}" stroke-width="1"/>"#,
                    theme.color(*role)
                );
            }
            Shape::Text {
                x,
                y,
                text,
                size,
                anchor,
                role,
            } => {
                let anchor = match anchor {
                    Anchor::Start => "start",
                    Anchor::Middle => "middle",
                    Anchor::End => "end",
                };
                let weight = if *role == Role::Value { 700 } else { 400 };
                let _ = write!(
                    svg,
                    r#"<text x="{x:.2}" y="{y:.2}" font-size="{size}" font-weight="{weight}" text-anchor="{anchor}" fill="{}">{}</text>"#,
                    theme.color(*role),
                    escape(text)
                );
            }
        }
    }
    svg.push_str("</svg>");
    svg
}
