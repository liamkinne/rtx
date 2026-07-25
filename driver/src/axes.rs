use core::range::RangeInclusive;
use nc::Unit;

pub struct AxisMeta {
    /// Unit of measurement.
    #[allow(unused)]
    pub uom: Unit,
    /// Encoder counts to real units scaling factor.
    pub scale: f32,
    /// Movement range.
    pub range: RangeInclusive<f32>,
    /// Zero position in real units.
    pub zero: f32,
}

impl AxisMeta {
    /// Convert encoder counts to real coordinates.
    pub fn from_counts(&self, counts: i16) -> f32 {
        (counts as f32 * self.scale) + self.zero
    }

    /// Convert real coordinate into encoder counts.
    pub fn to_counts(&self, real: f32) -> i16 {
        ((real - self.zero) / self.scale) as i16
    }
}

pub const ZED: AxisMeta = AxisMeta {
    uom: Unit::Deg,
    scale: -0.2267,
    range: RangeInclusive {
        start: 0.0,
        last: 940.0,
    },
    zero: 470.0,
};
pub const SHOULDER: AxisMeta = AxisMeta {
    uom: Unit::Deg,
    scale: 0.03422,
    range: RangeInclusive {
        start: -90.0,
        last: 90.0,
    },
    zero: 0.0,
};
pub const ELBOW: AxisMeta = AxisMeta {
    uom: Unit::Deg,
    scale: 0.06844,
    range: RangeInclusive {
        start: -165.0,
        last: 165.0,
    },
    zero: 0.0,
};
pub const YAW: AxisMeta = AxisMeta {
    uom: Unit::Deg,
    scale: 0.10267,
    range: RangeInclusive {
        start: -110.0,
        last: 110.0,
    },
    zero: 0.0,
};
pub const WRIST_PITCH: AxisMeta = AxisMeta {
    uom: Unit::Deg,
    scale: 0.07415,
    range: RangeInclusive {
        start: -4.0,
        last: 98.0,
    },
    zero: 0.0,
};
pub const WRIST_ROLL: AxisMeta = AxisMeta {
    uom: Unit::Deg,
    scale: 0.07415,
    range: RangeInclusive {
        start: -181.0,
        last: 132.0,
    },
    zero: 0.0,
};
pub const GRIP: AxisMeta = AxisMeta {
    uom: Unit::Mm,
    scale: 0.0718, // linear fit, small error
    range: RangeInclusive {
        start: 0.0,
        last: 90.0,
    },
    zero: 0.0,
};

pub mod gains {
    use motion::pid::Gains;

    pub const ZED: Gains = Gains {
        kp: 0.01,
        ki: 0.01,
        kd: 0.0002,
    };

    pub const OTHER: Gains = Gains {
        kp: 0.01,
        ki: 0.01,
        kd: 0.001,
    };
}
