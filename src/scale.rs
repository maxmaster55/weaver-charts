//! Mapping data values to screen positions, and picking readable ticks.

/// A linear map from a data `domain` to a screen `range`.
///
/// ```
/// use weaver_charts::Scale;
///
/// let y = Scale::new(0.0, 100.0, 200.0, 0.0); // screen y grows downwards
/// assert_eq!(y.map(25.0), 150.0);
/// assert_eq!(y.invert(150.0), 25.0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scale {
    /// Smallest data value.
    pub domain_min: f64,
    /// Largest data value.
    pub domain_max: f64,
    /// Screen position of `domain_min`.
    pub range_start: f64,
    /// Screen position of `domain_max`.
    pub range_end: f64,
}

impl Scale {
    /// A scale from `domain_min..domain_max` to `range_start..range_end`.
    pub fn new(domain_min: f64, domain_max: f64, range_start: f64, range_end: f64) -> Self {
        Scale {
            domain_min,
            domain_max,
            range_start,
            range_end,
        }
    }

    /// The screen position of a data value.
    pub fn map(&self, value: f64) -> f64 {
        let span = self.domain_max - self.domain_min;
        let t = if span.abs() < f64::EPSILON {
            0.5
        } else {
            (value - self.domain_min) / span
        };
        self.range_start + t * (self.range_end - self.range_start)
    }

    /// The data value at a screen position.
    pub fn invert(&self, position: f64) -> f64 {
        let span = self.range_end - self.range_start;
        let t = if span.abs() < f64::EPSILON {
            0.5
        } else {
            (position - self.range_start) / span
        };
        self.domain_min + t * (self.domain_max - self.domain_min)
    }

    /// Widen the domain to round numbers and return about `count` ticks on
    /// it, such as `0, 20, 40, 60, 80, 100`.
    pub fn nice(&mut self, count: usize) -> Vec<f64> {
        let (min, max, step) = nice_bounds(self.domain_min, self.domain_max, count);
        self.domain_min = min;
        self.domain_max = max;
        ticks(min, max, step)
    }
}

/// Round `min..max` out to numbers that read well, with a step of 1, 2 or
/// 5 times a power of ten. Returns `(min, max, step)`.
pub fn nice_bounds(min: f64, max: f64, count: usize) -> (f64, f64, f64) {
    let (min, max) = if min <= max { (min, max) } else { (max, min) };
    let (min, max) = if (max - min).abs() < f64::EPSILON {
        let pad = if min == 0.0 { 1.0 } else { min.abs() * 0.1 };
        (min - pad, max + pad)
    } else {
        (min, max)
    };
    let step = nice_step((max - min) / count.max(1) as f64);
    let lo = (min / step).floor() * step;
    let hi = (max / step).ceil() * step;
    (lo, hi, step)
}

fn nice_step(raw: f64) -> f64 {
    let exponent = raw.abs().log10().floor();
    let base = 10f64.powf(exponent);
    let fraction = raw / base;
    let nice = if fraction < 1.5 {
        1.0
    } else if fraction < 3.0 {
        2.0
    } else if fraction < 7.0 {
        5.0
    } else {
        10.0
    };
    nice * base
}

fn ticks(min: f64, max: f64, step: f64) -> Vec<f64> {
    let count = ((max - min) / step).round() as usize;
    (0..=count)
        .map(|i| {
            clean(min + step * i as f64, step)
        })
        .collect()
}

/// Round away floating-point dust such as `0.6000000000000001`, keeping as
/// many decimals as `step` needs.
pub(crate) fn clean(value: f64, step: f64) -> f64 {
    let decimals = (-step.abs().log10().floor()).clamp(0.0, 12.0) as i32;
    let factor = 10f64.powi(decimals);
    (value * factor).round() / factor
}

/// Format a tick label compactly: `1200` becomes `1.2k`, `0.5` stays `0.5`.
pub fn format_tick(value: f64) -> String {
    let abs = value.abs();
    let (scaled, suffix) = if abs >= 1e9 {
        (value / 1e9, "G")
    } else if abs >= 1e6 {
        (value / 1e6, "M")
    } else if abs >= 1e3 {
        (value / 1e3, "k")
    } else {
        (value, "")
    };
    let mut text = format!("{scaled:.2}");
    while text.contains('.') && (text.ends_with('0') || text.ends_with('.')) {
        text.pop();
    }
    if text == "-0" {
        text = "0".into();
    }
    text + suffix
}
