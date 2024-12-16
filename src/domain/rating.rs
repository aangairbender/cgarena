#[derive(Debug, Clone, Copy)]
pub struct Rating {
    /// rating value
    pub mu: f64,
    /// uncertainty value
    pub sigma: f64,
}

impl Default for Rating {
    fn default() -> Self {
        Self {
            mu: 25.0,
            sigma: 25.0 / 3.0,
        }
    }
}
