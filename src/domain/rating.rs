#[derive(Debug, Clone, Copy)]
pub struct Rating {
    /// rating value
    pub mu: f64,
    /// uncertainty value
    pub sigma: f64,
}

impl Rating {
    pub fn new(mu: f64, sigma: f64) -> Rating {
        Self { mu, sigma }
    }

    pub fn score(&self, uncertainty_coefficient: f64) -> f64 {
        self.mu - self.sigma * uncertainty_coefficient
    }
}
