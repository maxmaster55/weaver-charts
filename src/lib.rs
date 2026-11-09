//! # weaver-charts
//!
//! Line, area, bar, sparkline, donut and gauge charts. A chart becomes a
//! list of plain [`Shape`]s (paths, rectangles, circles, text) that any
//! renderer can draw, or an SVG string with [`Chart::to_svg`]. No
//! dependencies.
//!
//! With the `motion` feature, [`AnimatedChart`] springs every value to new
//! data using the Weave motion engine.
//!
//! ```
//! use weaver_charts::{Chart, Theme};
//!
//! let bars = Chart::bar()
//!     .labels(["Q1", "Q2", "Q3", "Q4"])
//!     .series("2025", [12.0, 18.0, 15.0, 22.0])
//!     .series("2026", [16.0, 21.0, 19.0, 28.0]);
//! let svg = bars.to_svg_with(&Theme::dark());
//!
//! let gauge = Chart::gauge(72.0, 0.0, 100.0).unit("%").labels(["CPU"]);
//! let shapes = gauge.shapes(); // draw these yourself
//! # let _ = (svg, shapes);
//! ```

#![warn(missing_docs)]
#![forbid(unsafe_code)]

#[cfg(feature = "motion")]
mod animate;
mod chart;
mod scale;
mod shape;

#[cfg(feature = "motion")]
pub use animate::AnimatedChart;
pub use chart::{Chart, ChartKind, Series};
pub use scale::{format_tick, nice_bounds, Scale};
pub use shape::{path_data, to_svg, Anchor, PathCmd, Role, Shape, Theme};
