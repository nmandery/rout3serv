use ordered_float::OrderedFloat;

pub type Weight = OrderedFloat<f64>;

pub trait ToFloatWeight {
    fn to_float_weight(&self) -> f64;
}

impl ToFloatWeight for Weight {
    fn to_float_weight(&self) -> f64 {
        self.0
    }
}
