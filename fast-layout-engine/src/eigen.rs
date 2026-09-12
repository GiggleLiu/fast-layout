//! Partial symmetric eigenvectors by inverse subspace iteration and sparse CG.
use crate::schema::Rng;
use nalgebra::{linalg::SymmetricEigen, DMatrix};

pub(crate) struct NormalizedLaplacian {
    pub degree: Vec<f64>,
    pub edges: Vec<(usize, usize, f64)>,
}

impl NormalizedLaplacian {
    pub fn apply(&self, x: &[f64], out: &mut [f64]) {
        out.copy_from_slice(x);
        for &(i, j, w) in &self.edges {
            out[i] -= w * x[j];
            out[j] -= w * x[i];
        }
    }

    pub fn dense(&self) -> DMatrix<f64> {
        let n = self.degree.len();
        let mut a = DMatrix::identity(n, n);
        for &(i, j, w) in &self.edges {
            a[(i, j)] = -w;
            a[(j, i)] = -w;
        }
        a
    }
}

pub(crate) struct Eigenpairs {
    pub values: Vec<f64>,
    pub vectors: DMatrix<f64>,
    pub iterations: usize,
    pub converged: bool,
}

pub(crate) fn partial(
    a: &NormalizedLaplacian,
    count: usize,
    max_iterations: usize,
    tolerance: f64,
    seed: u64,
) -> Result<Eigenpairs, String> {
    let n = a.degree.len();
    let width = (count + 4).min(n - 1);
    let mut rng = Rng::new(seed);
    let mut q = DMatrix::from_fn(n, width, |_, _| 2. * rng.uniform() - 1.);
    let mut null: Vec<_> = a.degree.iter().map(|x| x.sqrt()).collect();
    let norm = dot(&null, &null).sqrt();
    for x in &mut null {
        *x /= norm;
    }
    orthonormalize(&mut q, &null)?;
    let mut work = CgWork::new(n);
    let mut aq = DMatrix::zeros(n, width);
    let mut values = vec![0.; width];
    let mut shift = 0.01;
    for iteration in 1..=max_iterations {
        for col in 0..width {
            let rhs = q.column(col);
            work.solve(a, rhs.as_slice(), shift)?;
            q.column_mut(col).copy_from_slice(&work.x);
        }
        orthonormalize(&mut q, &null)?;
        for col in 0..width {
            a.apply(q.column(col).as_slice(), aq.column_mut(col).as_mut_slice());
        }
        let projected = q.transpose() * &aq;
        let eigen = SymmetricEigen::try_new(projected, 1e-14, 100 * width)
            .ok_or("fast-layout-engine: spectral subspace eigensolve did not converge")?;
        let mut order: Vec<_> = (0..width).collect();
        order.sort_by(|&i, &j| eigen.eigenvalues[i].total_cmp(&eigen.eigenvalues[j]));
        let rotation = DMatrix::from_fn(width, width, |i, j| eigen.eigenvectors[(i, order[j])]);
        q *= &rotation;
        aq *= &rotation;
        for col in 0..width {
            values[col] = eigen.eigenvalues[order[col]];
        }
        let residual = (0..count)
            .map(|col| {
                (0..n)
                    .map(|i| (aq[(i, col)] - values[col] * q[(i, col)]).powi(2))
                    .sum::<f64>()
                    .sqrt()
            })
            .fold(0., f64::max);
        if !residual.is_finite() {
            return Err(
                "fast-layout-engine: spectral iteration produced a non-finite residual".into(),
            );
        }
        if residual <= tolerance {
            return Ok(Eigenpairs {
                values,
                vectors: q,
                iterations: iteration,
                converged: true,
            });
        }
        // Follow the low end of the spectrum without making the shifted solve
        // singular. Oversampling separates the desired modes from the block edge.
        shift = (0.1 * values[width - 1]).clamp(1e-8, 0.01);
    }
    Ok(Eigenpairs {
        values,
        vectors: q,
        iterations: max_iterations,
        converged: false,
    })
}

fn orthonormalize(q: &mut DMatrix<f64>, null: &[f64]) -> Result<(), String> {
    let n = q.nrows();
    for col in 0..q.ncols() {
        let (before, rest) = q.as_mut_slice().split_at_mut(col * n);
        let v = &mut rest[..n];
        // Two passes keep nearly converged inverse iterates independent.
        for _ in 0..2 {
            let coefficient = dot(v, null);
            for i in 0..n {
                v[i] -= coefficient * null[i];
            }
            for previous in before.chunks_exact(n) {
                let coefficient = dot(v, previous);
                for i in 0..n {
                    v[i] -= coefficient * previous[i];
                }
            }
        }
        let norm = dot(v, v).sqrt();
        if norm < 1e-14 || !norm.is_finite() {
            return Err("fast-layout-engine: spectral subspace lost rank; try a different seed or fewer dimensions".into());
        }
        for x in v {
            *x /= norm;
        }
    }
    Ok(())
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}

struct CgWork {
    x: Vec<f64>,
    r: Vec<f64>,
    p: Vec<f64>,
    ap: Vec<f64>,
}
impl CgWork {
    fn new(n: usize) -> Self {
        Self {
            x: vec![0.; n],
            r: vec![0.; n],
            p: vec![0.; n],
            ap: vec![0.; n],
        }
    }
    fn solve(&mut self, a: &NormalizedLaplacian, b: &[f64], shift: f64) -> Result<(), String> {
        self.x.fill(0.);
        self.r.copy_from_slice(b);
        self.p.copy_from_slice(b);
        let mut rr = dot(b, b);
        let target = rr * 1e-20;
        for _ in 0..(4 * b.len()).max(64) {
            a.apply(&self.p, &mut self.ap);
            for i in 0..b.len() {
                self.ap[i] += shift * self.p[i];
            }
            let denominator = dot(&self.p, &self.ap);
            if denominator <= 0. || !denominator.is_finite() {
                return Err(
                    "fast-layout-engine: spectral shifted solve lost positive definiteness".into(),
                );
            }
            let alpha = rr / denominator;
            for i in 0..b.len() {
                self.x[i] += alpha * self.p[i];
                self.r[i] -= alpha * self.ap[i];
            }
            let next = dot(&self.r, &self.r);
            if next <= target {
                // Verify the true residual, not just the recursively updated one.
                a.apply(&self.x, &mut self.ap);
                let error = (0..b.len())
                    .map(|i| (self.ap[i] + shift * self.x[i] - b[i]).powi(2))
                    .sum::<f64>();
                if error <= dot(b, b) * 1e-16 {
                    return Ok(());
                }
                return Err(
                    "fast-layout-engine: spectral shifted solve residual exceeds 1e-8".into(),
                );
            }
            let beta = next / rr;
            for i in 0..b.len() {
                self.p[i] = self.r[i] + beta * self.p[i];
            }
            rr = next;
        }
        Err("fast-layout-engine: spectral shifted solve did not converge; reduce edge weight ratios".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_cycle_matches_known_low_modes_including_multiplicity() {
        let n = 160;
        let a = NormalizedLaplacian {
            degree: vec![2.; n],
            edges: (0..n).map(|i| (i, (i + 1) % n, 0.5)).collect(),
        };
        let out = partial(&a, 3, 100, 1e-9, 1).unwrap();
        assert!(out.converged);
        for (col, k) in [1., 1., 2.].into_iter().enumerate() {
            let expected = 1. - (std::f64::consts::TAU * k / n as f64).cos();
            assert!((out.values[col] - expected).abs() < 1e-9);
        }
    }
}
