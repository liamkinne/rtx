use num_traits::Float;

/// Exponential moving average filter.
pub struct ExpMovingAvg<T: Float> {
    alpha: T,
    value: Option<T>,
}

impl<T: Float> ExpMovingAvg<T> {
    pub fn new(alpha: T) -> Self {
        assert!(
            alpha > T::zero() && alpha <= T::one(),
            "alpha must be in (0, 1]"
        );
        Self { alpha, value: None }
    }

    pub fn update(&mut self, sample: T) -> T {
        let new_value = match self.value {
            Some(prev) => self.alpha * sample + (T::one() - self.alpha) * prev,
            None => sample,
        };
        self.value = Some(new_value);
        new_value
    }

    pub fn value(&self) -> Option<T> {
        self.value
    }

    pub fn reset(&mut self) {
        self.value = None;
    }
}
