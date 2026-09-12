//! Flat quadtree/octree for spring repulsion. No platform-specific intrinsics.
use crate::schema::Rng;
const NONE: usize = usize::MAX;

struct Cell {
    start: usize,
    end: usize,
    center: [f64; 3],
    half: f64,
    mass_center: [f64; 3],
    children: [usize; 8],
}

pub(crate) struct Tree {
    cells: Vec<Cell>,
    order: Vec<usize>,
    scratch: Vec<usize>,
    dim: usize,
}

impl Tree {
    pub fn new(dim: usize, n: usize) -> Self {
        Self {
            cells: Vec::with_capacity(n),
            order: (0..n).collect(),
            scratch: vec![0; n],
            dim,
        }
    }

    pub fn rebuild(&mut self, positions: &[f64]) {
        self.cells.clear();
        let mut low = [f64::INFINITY; 3];
        let mut high = [f64::NEG_INFINITY; 3];
        for p in positions.chunks_exact(self.dim) {
            for d in 0..self.dim {
                low[d] = low[d].min(p[d]);
                high[d] = high[d].max(p[d]);
            }
        }
        let mut center = [0.; 3];
        let mut half: f64 = 1e-12;
        for d in 0..self.dim {
            center[d] = (low[d] + high[d]) * 0.5;
            half = half.max((high[d] - low[d]) * 0.50000001);
        }
        self.build(positions, 0, self.order.len(), center, half, 0);
    }

    fn build(
        &mut self,
        p: &[f64],
        start: usize,
        end: usize,
        center: [f64; 3],
        half: f64,
        depth: usize,
    ) -> usize {
        let index = self.cells.len();
        let mut mass_center = [0.; 3];
        for &i in &self.order[start..end] {
            for d in 0..self.dim {
                mass_center[d] += p[i * self.dim + d];
            }
        }
        for x in &mut mass_center {
            *x /= (end - start) as f64;
        }
        self.cells.push(Cell {
            start,
            end,
            center,
            half,
            mass_center,
            children: [NONE; 8],
        });
        if end - start <= 8 || depth >= 48 {
            return index;
        }
        let octant = |i: usize| -> usize {
            (0..self.dim).fold(0, |bits, d| {
                bits | (usize::from(p[i * self.dim + d] >= center[d]) << d)
            })
        };
        let mut counts = [0; 8];
        for &i in &self.order[start..end] {
            counts[octant(i)] += 1;
        }
        let mut starts = [start; 8];
        for j in 1..8 {
            starts[j] = starts[j - 1] + counts[j - 1];
        }
        let mut cursors = starts;
        for &i in &self.order[start..end] {
            let j = octant(i);
            self.scratch[cursors[j]] = i;
            cursors[j] += 1;
        }
        self.order[start..end].copy_from_slice(&self.scratch[start..end]);
        for j in 0..1 << self.dim {
            if counts[j] == 0 {
                continue;
            }
            let mut child_center = center;
            for (d, x) in child_center.iter_mut().enumerate().take(self.dim) {
                *x += if j & (1 << d) == 0 {
                    -half * 0.5
                } else {
                    half * 0.5
                };
            }
            let child = self.build(
                p,
                starts[j],
                starts[j] + counts[j],
                child_center,
                half * 0.5,
                depth + 1,
            );
            self.cells[index].children[j] = child;
        }
        index
    }

    pub fn forces(&self, p: &[f64], k2: f64, theta: f64, seed: u64, out: &mut [f64]) {
        out.fill(0.);
        for (i, force) in out.chunks_exact_mut(self.dim).enumerate() {
            self.accumulate(0, i, p, k2, theta * theta, seed, force);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn accumulate(
        &self,
        index: usize,
        i: usize,
        p: &[f64],
        k2: f64,
        theta2: f64,
        seed: u64,
        force: &mut [f64],
    ) {
        let cell = &self.cells[index];
        let leaf = cell.children.iter().all(|&c| c == NONE);
        if leaf {
            for &j in &self.order[cell.start..cell.end] {
                if i != j {
                    repel(p, self.dim, i, j, k2, seed, force);
                }
            }
            return;
        }
        let contains =
            (0..self.dim).all(|d| (p[i * self.dim + d] - cell.center[d]).abs() <= cell.half);
        let dist2: f64 = (0..self.dim)
            .map(|d| (p[i * self.dim + d] - cell.mass_center[d]).powi(2))
            .sum();
        if !contains && 4. * cell.half * cell.half < theta2 * dist2 {
            let scale = k2 * (cell.end - cell.start) as f64 / dist2;
            for d in 0..self.dim {
                force[d] += scale * (p[i * self.dim + d] - cell.mass_center[d]);
            }
        } else {
            for &child in &cell.children {
                if child != NONE {
                    self.accumulate(child, i, p, k2, theta2, seed, force);
                }
            }
        }
    }
}

pub(crate) fn repel(
    p: &[f64],
    dim: usize,
    i: usize,
    j: usize,
    k2: f64,
    seed: u64,
    force: &mut [f64],
) {
    let dist2: f64 = (0..dim)
        .map(|d| (p[i * dim + d] - p[j * dim + d]).powi(2))
        .sum();
    if dist2 == 0. {
        let mut rng = Rng::new(
            seed ^ (i.min(j) as u64).wrapping_mul(0x9e3779b97f4a7c15)
                ^ (i.max(j) as u64).wrapping_mul(0xbf58476d1ce4e5b9),
        );
        let sign = if i < j { 1. } else { -1. };
        for f in force {
            *f += sign * (2. * rng.uniform() - 1.);
        }
    } else {
        let scale = k2 / dist2;
        for d in 0..dim {
            force[d] += scale * (p[i * dim + d] - p[j * dim + d]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn approximation_matches_exact_forces_and_tighter_theta_improves_error() {
        for dim in [2, 3] {
            let mut rng = Rng::new(12);
            let p: Vec<_> = (0..300 * dim).map(|_| 2. * rng.uniform() - 1.).collect();
            let mut tree = Tree::new(dim, 300);
            tree.rebuild(&p);
            let mut exact = vec![0.; p.len()];
            for i in 0..300 {
                for j in 0..300 {
                    if i != j {
                        repel(&p, dim, i, j, 1., 12, &mut exact[i * dim..(i + 1) * dim]);
                    }
                }
            }
            let mut approximate = exact.clone();
            let mut errors = Vec::new();
            for theta in [0.7, 0.3, 0.] {
                tree.forces(&p, 1., theta, 12, &mut approximate);
                let error = approximate
                    .iter()
                    .zip(&exact)
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f64>()
                    .sqrt()
                    / exact.iter().map(|a| a * a).sum::<f64>().sqrt();
                errors.push(error);
            }
            assert!(errors[0] < 0.05, "dim={dim}: {errors:?}");
            assert!(errors[1] < errors[0]);
            assert!(errors[2] < 1e-12);
        }
    }

    #[test]
    fn coincident_points_have_finite_force_and_singleton_has_no_self_force() {
        let mut tree = Tree::new(2, 50);
        let p = vec![0.; 100];
        tree.rebuild(&p);
        let mut force = vec![0.; 100];
        tree.forces(&p, 1., 0.7, 1, &mut force);
        assert!(force.iter().all(|x| x.is_finite()));
        for d in 0..2 {
            assert!((0..50).map(|i| force[2 * i + d]).sum::<f64>().abs() < 1e-10);
        }
        let mut tree = Tree::new(2, 1);
        tree.rebuild(&[1., 2.]);
        let mut force = [99.; 2];
        tree.forces(&[1., 2.], 1., 0.7, 1, &mut force);
        assert_eq!(force, [0.; 2]);
    }
}
