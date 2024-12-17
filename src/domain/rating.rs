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

    pub fn score(&self) -> f64 {
        self.mu - self.sigma * 3.0
    }
}

impl Default for Rating {
    fn default() -> Self {
        Self {
            mu: 25.0,
            sigma: 25.0 / 3.0,
        }
    }
}
