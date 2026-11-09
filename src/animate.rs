//! Spring-animated data changes (the `motion` feature).
//!
//! [`AnimatedChart`] wraps a [`Chart`] and moves every value towards new
//! data with a spring. Bars grow, lines bend and gauges sweep instead of
//! jumping. Changing the data mid-flight keeps each value's velocity.
//!
//! ```
//! use weaver_charts::{AnimatedChart, Chart};
//!
//! let mut chart = AnimatedChart::grow_in(Chart::bar().series("Sales", [3.0, 5.0, 4.0]));
//! chart.advance(0.1);
//! let mid = chart.chart().all_series()[0].values[1];
//! assert!(mid > 0.0 && mid < 5.5);
//!
//! chart.set_series(0, [6.0, 2.0, 8.0]);
//! while chart.advance(1.0 / 60.0) {}
//! assert_eq!(chart.chart().all_series()[0].values, vec![6.0, 2.0, 8.0]);
//! ```

use weave_core::{Animated, MotionToken, SpringConfig};

use crate::Chart;

/// A chart whose values follow their targets with springs.
#[derive(Clone, Debug)]
pub struct AnimatedChart {
    chart: Chart,
    values: Vec<Vec<Animated<f64>>>,
    gauge: Animated<f64>,
    spring: SpringConfig,
}

impl AnimatedChart {
    /// Start at the chart's current values (nothing moves yet).
    pub fn new(chart: Chart) -> Self {
        let spring = MotionToken::DefaultSpatial
            .spring_config()
            .expect("spatial tokens are springs");
        let values = chart
            .all_series()
            .iter()
            .map(|s| {
                s.values
                    .iter()
                    .map(|v| Animated::resting(*v, spring))
                    .collect()
            })
            .collect();
        let gauge = Animated::resting(chart.value(), spring);
        AnimatedChart {
            chart,
            values,
            gauge,
            spring,
        }
    }

    /// Start every value at zero and grow into the data: a nice entrance.
    pub fn grow_in(chart: Chart) -> Self {
        let mut animated = AnimatedChart::new(chart);
        for series in &mut animated.values {
            for value in series {
                let target = value.target();
                *value = Animated::new(0.0, target, animated.spring);
            }
        }
        let target = animated.gauge.target();
        animated.gauge = Animated::new(0.0, target, animated.spring);
        animated.sync();
        animated
    }

    /// Use your own spring.
    pub fn with_spring(mut self, spring: SpringConfig) -> Self {
        self.spring = spring;
        for series in &mut self.values {
            for value in series {
                value.set_config(spring);
            }
        }
        self.gauge.set_config(spring);
        self
    }

    /// New data for series `index`. Added points grow from zero; removed
    /// points disappear.
    pub fn set_series(&mut self, index: usize, values: impl IntoIterator<Item = f64>) {
        let Some(series) = self.values.get_mut(index) else {
            return;
        };
        let targets: Vec<f64> = values.into_iter().collect();
        series.truncate(targets.len());
        for (i, target) in targets.iter().enumerate() {
            match series.get_mut(i) {
                Some(value) => value.set_target(*target),
                None => series.push(Animated::new(0.0, *target, self.spring)),
            }
        }
    }

    /// New gauge value.
    pub fn set_value(&mut self, value: f64) {
        self.gauge.set_target(value);
    }

    /// Snap everything to its target.
    pub fn settle(&mut self) {
        for value in self.values.iter_mut().flatten() {
            value.settle();
        }
        self.gauge.settle();
        self.sync();
    }

    /// Advance by `dt` seconds. Returns whether anything is still moving.
    pub fn advance(&mut self, dt: f64) -> bool {
        let mut active = self.gauge.advance(dt);
        for value in self.values.iter_mut().flatten() {
            active |= value.advance(dt);
        }
        self.sync();
        active
    }

    /// The chart with the values as they are right now.
    pub fn chart(&self) -> &Chart {
        &self.chart
    }

    fn sync(&mut self) {
        for (i, series) in self.values.iter().enumerate() {
            self.chart
                .set_values(i, series.iter().map(Animated::value).collect());
        }
        self.chart.set_value(self.gauge.value());
    }
}
