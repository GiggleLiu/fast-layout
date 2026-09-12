use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Algorithm {
    #[default]
    Stress,
    Spring,
    Spectral,
    Shell,
    Buchheim,
}

/// Zero-based edge list and layout options. Empty vectors select defaults.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Request {
    pub nodes: usize,
    pub edges: Vec<[usize; 2]>,
    pub algorithm: Algorithm,
    pub dim: usize,
    pub seed: u64,
    pub iterations: usize,
    pub tolerance: f64,
    pub initial: Vec<Option<Vec<f64>>>,
    pub pins: Vec<Vec<bool>>,
    pub edge_weights: Vec<f64>,
    pub theta: Option<f64>,
    pub c: Option<f64>,
    pub temperature: Option<f64>,
    pub node_weights: Vec<f64>,
    pub shells: Vec<Vec<usize>>,
    pub node_sizes: Vec<f64>,
    pub root: usize,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            nodes: 0,
            edges: Vec::new(),
            algorithm: Algorithm::Stress,
            dim: 2,
            seed: 1,
            iterations: 100,
            tolerance: 1e-5,
            initial: Vec::new(),
            pins: Vec::new(),
            edge_weights: Vec::new(),
            theta: None,
            c: None,
            temperature: None,
            node_weights: Vec::new(),
            shells: Vec::new(),
            node_sizes: Vec::new(),
            root: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Response {
    pub positions: Vec<Vec<f64>>,
    /// Number of numerical updates, excluding initialization.
    pub iterations: usize,
    pub converged: bool,
    /// Stress objective or spring energy. Not available for geometric layouts.
    pub objective: Option<f64>,
}

impl Request {
    pub fn new(nodes: usize, edges: Vec<[usize; 2]>, algorithm: Algorithm) -> Self {
        Self {
            nodes,
            edges,
            algorithm,
            ..Self::default()
        }
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        let fail = |s: &str| Err(format!("fast-layout-engine: {s}"));
        if self.dim == 0 || self.dim > 4096 {
            return fail("dim must be between 1 and 4096");
        }
        if self.nodes > 1_000_000 || self.nodes.saturating_mul(self.dim) > 4_000_000 {
            return fail("layout exceeds 1,000,000 nodes or 4,000,000 coordinates");
        }
        if self.edges.len() > 2_000_000 {
            return fail("layout exceeds 2,000,000 input edges");
        }
        if matches!(self.algorithm, Algorithm::Stress | Algorithm::Spectral) && self.nodes > 4096 {
            return fail("stress and spectral currently support at most 4096 nodes; use spring for larger graphs");
        }
        if self.iterations == 0 || self.iterations > 100_000 {
            return fail("iterations must be between 1 and 100000");
        }
        if !self.tolerance.is_finite() || self.tolerance < 0.0 {
            return fail("tolerance must be finite and nonnegative");
        }
        if self.edges.iter().flatten().any(|&i| i >= self.nodes) {
            return fail("edge endpoint is outside 0..nodes");
        }
        if !self.edge_weights.is_empty() && self.edge_weights.len() != self.edges.len() {
            return fail("edge_weights must contain one weight per input edge");
        }
        for (name, values) in [
            ("edge_weights", &self.edge_weights),
            ("node_weights", &self.node_weights),
            ("node_sizes", &self.node_sizes),
        ] {
            if values
                .iter()
                .any(|v| !v.is_finite() || *v < 1e-9 || *v > 1e9)
            {
                return Err(format!(
                    "fast-layout-engine: {name} must be finite and in [1e-9, 1e9]"
                ));
            }
        }
        if !self.node_weights.is_empty() && self.node_weights.len() != self.nodes {
            return fail("node_weights must contain one weight per node");
        }
        if self.node_sizes.len() > self.nodes {
            return fail("node_sizes cannot contain more entries than nodes");
        }
        if self.initial.len() > self.nodes || self.pins.len() > self.nodes {
            return fail("initial and pins cannot contain more entries than nodes");
        }
        for p in self.initial.iter().flatten() {
            if p.len() != self.dim || p.iter().any(|x| !x.is_finite() || x.abs() > 1e9) {
                return fail("initial positions must have dim finite coordinates in [-1e9, 1e9]");
            }
        }
        if self
            .pins
            .iter()
            .any(|p| !p.is_empty() && p.len() != self.dim)
        {
            return fail("each pin mask must be empty or have dim booleans");
        }
        let iterative = matches!(self.algorithm, Algorithm::Stress | Algorithm::Spring);
        if !iterative && (!self.initial.is_empty() || !self.pins.is_empty()) {
            return fail("initial positions and pins are only supported for stress and spring");
        }
        if self.algorithm != Algorithm::Spring
            && (self.theta.is_some() || self.c.is_some() || self.temperature.is_some())
        {
            return fail("theta, c, and temperature are only supported for spring");
        }
        if self
            .theta
            .is_some_and(|x| !x.is_finite() || !(0.0..=2.0).contains(&x))
        {
            return fail("theta must be finite and in [0, 2]; use 0 for exact spring forces");
        }
        for (name, value) in [("c", self.c), ("temperature", self.temperature)] {
            if value.is_some_and(|x| !x.is_finite() || !(1e-9..=1e9).contains(&x)) {
                return Err(format!(
                    "fast-layout-engine: {name} must be finite and in [1e-9, 1e9]"
                ));
            }
        }
        if self.algorithm != Algorithm::Spectral && !self.node_weights.is_empty() {
            return fail("node_weights are only supported for spectral");
        }
        if self.algorithm != Algorithm::Shell && !self.shells.is_empty() {
            return fail("shells are only supported for shell");
        }
        if self.algorithm != Algorithm::Buchheim && (!self.node_sizes.is_empty() || self.root != 0)
        {
            return fail("node_sizes and root are only supported for buchheim");
        }
        if matches!(self.algorithm, Algorithm::Shell | Algorithm::Buchheim) {
            if self.dim != 2 {
                return fail("shell and buchheim produce 2D coordinates; dim must be 2");
            }
            if !self.edge_weights.is_empty() {
                return fail("shell and buchheim do not use edge_weights");
            }
        }
        Ok(())
    }

    pub(crate) fn pinned(&self, i: usize, d: usize) -> bool {
        self.pins
            .get(i)
            .and_then(|p| p.get(d))
            .copied()
            .unwrap_or(false)
    }

    pub(crate) fn initial_positions(&self) -> Vec<f64> {
        let mut rng = Rng::new(self.seed);
        let mut data = Vec::with_capacity(self.nodes * self.dim);
        for i in 0..self.nodes {
            for d in 0..self.dim {
                let random = 2.0 * rng.uniform() - 1.0;
                data.push(
                    self.initial
                        .get(i)
                        .and_then(|p| p.as_ref())
                        .map_or(random, |p| p[d]),
                );
            }
        }
        data
    }
}

pub(crate) fn response(
    data: Vec<f64>,
    dim: usize,
    iterations: usize,
    converged: bool,
    objective: Option<f64>,
) -> Response {
    Response {
        positions: data.chunks_exact(dim).map(<[f64]>::to_vec).collect(),
        iterations,
        converged,
        objective,
    }
}

// Seeded xorshift64* sequence adapted from chalks-engine; no system entropy.
pub(crate) struct Rng(u64);
impl Rng {
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    pub(crate) fn uniform(&mut self) -> f64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
    }
}

pub(crate) fn distance_squared(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum()
}
