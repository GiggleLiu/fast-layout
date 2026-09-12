//! Stress majorization adapted from NetworkLayout.jl, using constrained solves.
use crate::{
    graph::Graph,
    schema::{distance_squared, response, Rng},
    Request, Response,
};
use nalgebra::{linalg::Cholesky, DMatrix, Dyn};
use std::collections::HashMap;

struct SolveGroup {
    free: Vec<usize>,
    axes: Vec<usize>,
    factor: Cholesky<f64, Dyn>,
    offset: DMatrix<f64>,
    rhs: DMatrix<f64>,
    center: bool,
}

pub(crate) fn run(req: &Request) -> Result<Response, String> {
    match req.dim {
        2 => run_dim::<2>(req),
        3 => run_dim::<3>(req),
        _ => run_dim::<0>(req),
    }
}

fn run_dim<const D: usize>(req: &Request) -> Result<Response, String> {
    let graph = Graph::new(req)?;
    let n = graph.n;
    let dim = if D == 0 { req.dim } else { D };
    let mut positions = req.initial_positions();
    if n < 2 {
        return Ok(response(positions, dim, 0, true, Some(0.0)));
    }
    let distances = graph.distances();
    let mut weights = distances.clone();
    for i in 0..n {
        for j in 0..n {
            weights[i * n + j] = if i == j {
                0.0
            } else {
                distances[i * n + j].recip().powi(2)
            };
        }
    }
    separate_coincident(req, &mut positions, &distances);
    let mut groups = prepare_solves(req, &weights, &positions)?;
    let mut old_stress = objective_dim::<D>(&positions, dim, &distances);
    if groups.is_empty() {
        return Ok(response(positions, dim, 0, true, Some(old_stress)));
    }
    let mut rhs = vec![0.0; n * dim];
    let mut next = positions.clone();
    for iteration in 1..=req.iteration_limit() {
        rhs.fill(0.0);
        for i in 0..n {
            for j in i + 1..n {
                let norm = distance_squared(
                    &positions[i * dim..(i + 1) * dim],
                    &positions[j * dim..(j + 1) * dim],
                )
                .sqrt();
                if norm == 0.0 {
                    continue;
                }
                let scale = 1.0 / (distances[i * n + j] * norm);
                for d in 0..dim {
                    let v = scale * (positions[i * dim + d] - positions[j * dim + d]);
                    rhs[i * dim + d] += v;
                    rhs[j * dim + d] -= v;
                }
            }
        }
        // Pinned entries remain copied from the original coordinates.
        for group in &mut groups {
            for (col, &d) in group.axes.iter().enumerate() {
                for (row, &i) in group.free.iter().enumerate() {
                    group.rhs[(row, col)] = rhs[i * dim + d] + group.offset[(row, col)];
                }
            }
            group.factor.solve_mut(&mut group.rhs);
            for (col, &d) in group.axes.iter().enumerate() {
                if group.center {
                    next[d] = 0.0;
                }
                for (row, &i) in group.free.iter().enumerate() {
                    next[i * dim + d] = group.rhs[(row, col)];
                }
                if group.center {
                    let mean = (0..n).map(|i| next[i * dim + d]).sum::<f64>() / n as f64;
                    for i in 0..n {
                        next[i * dim + d] -= mean;
                    }
                }
            }
        }
        let new_stress = objective_dim::<D>(&next, dim, &distances);
        if !new_stress.is_finite() {
            return Err(
                "fast-layout-engine: stress solve overflowed; reduce weight or coordinate ranges"
                    .into(),
            );
        }
        if new_stress > old_stress + 1e-8 * old_stress.max(1.0) {
            return Err("fast-layout-engine: stress solve lost numerical accuracy; reduce edge weight ratios".into());
        }
        let movement = distance_squared(&positions, &next).sqrt();
        let converged = req.tolerance > 0.0
            && ((new_stress - old_stress).abs() <= req.tolerance * new_stress.max(1e-12)
                || movement <= req.tolerance);
        std::mem::swap(&mut positions, &mut next);
        old_stress = new_stress;
        if converged {
            return Ok(response(positions, dim, iteration, true, Some(new_stress)));
        }
    }
    Ok(response(
        positions,
        dim,
        req.iteration_limit(),
        false,
        Some(old_stress),
    ))
}

fn prepare_solves(
    req: &Request,
    weights: &[f64],
    positions: &[f64],
) -> Result<Vec<SolveGroup>, String> {
    let n = req.nodes;
    let mut groups: Vec<SolveGroup> = Vec::new();
    let diagonal: Vec<f64> = weights
        .chunks_exact(n)
        .map(|row| row.iter().sum())
        .collect();
    let mut entries = 2 * n * n;
    for d in 0..req.dim {
        let center = !(0..n).any(|i| req.pinned(i, d));
        let free: Vec<_> = (0..n)
            .filter(|&i| !req.pinned(i, d) && (!center || i != 0))
            .collect();
        if free.is_empty() {
            continue;
        }
        if let Some(group) = groups
            .iter_mut()
            .find(|g| g.free == free && g.center == center)
        {
            group.axes.push(d);
            continue;
        }
        entries += free.len() * free.len();
        if entries > 64_000_000 {
            return Err("fast-layout-engine: stress matrices exceed 512 MB; reduce nodes or distinct pin masks".into());
        }
        // ponytail: dense O(n^3) factorization, use a residual-controlled iterative
        // solve if benchmarks require larger exact-stress graphs.
        let matrix = DMatrix::from_fn(free.len(), free.len(), |a, b| {
            if a == b {
                diagonal[free[a]]
            } else {
                -weights[free[a] * n + free[b]]
            }
        });
        let factor = Cholesky::new(matrix).ok_or(
            "fast-layout-engine: stress Laplacian is ill-conditioned; reduce edge weight ratios",
        )?;
        groups.push(SolveGroup {
            free,
            axes: vec![d],
            factor,
            offset: DMatrix::zeros(0, 0),
            rhs: DMatrix::zeros(0, 0),
            center,
        });
    }
    for group in &mut groups {
        group.offset = DMatrix::from_fn(group.free.len(), group.axes.len(), |row, col| {
            let i = group.free[row];
            let d = group.axes[col];
            (0..n)
                .filter(|&j| req.pinned(j, d))
                .map(|j| weights[i * n + j] * positions[j * req.dim + d])
                .sum()
        });
        group.rhs = DMatrix::zeros(group.free.len(), group.axes.len());
    }
    Ok(groups)
}

pub(crate) fn separate_coincident(req: &Request, positions: &mut [f64], distances: &[f64]) {
    let keys: Vec<Vec<u64>> = positions
        .chunks_exact(req.dim)
        .map(|point| {
            point
                .iter()
                .map(|x| if *x == 0. { 0 } else { x.to_bits() })
                .collect()
        })
        .collect();
    let mut counts = HashMap::new();
    for key in &keys {
        *counts.entry(key).or_insert(0usize) += 1;
    }
    let scale = distances.iter().copied().fold(0.0, f64::max).max(1.0) * 1e-6;
    let mut rng = Rng::new(req.seed);
    for (i, key) in keys.iter().enumerate() {
        if counts[key] > 1 {
            for d in 0..req.dim {
                if !req.pinned(i, d) {
                    positions[i * req.dim + d] += scale * (rng.uniform() - 0.5);
                }
            }
        }
    }
}

pub(crate) fn objective(positions: &[f64], dim: usize, distances: &[f64]) -> f64 {
    match dim {
        2 => objective_dim::<2>(positions, dim, distances),
        3 => objective_dim::<3>(positions, dim, distances),
        _ => objective_dim::<0>(positions, dim, distances),
    }
}

fn objective_dim<const D: usize>(positions: &[f64], dim: usize, distances: &[f64]) -> f64 {
    let dim = if D == 0 { dim } else { D };
    let n = positions.len() / dim;
    let mut total = 0.0;
    for i in 0..n {
        for j in i + 1..n {
            let d = distance_squared(
                &positions[i * dim..(i + 1) * dim],
                &positions[j * dim..(j + 1) * dim],
            )
            .sqrt();
            total += (d / distances[i * n + j] - 1.0).powi(2);
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compute, Algorithm, StressMethod};

    #[test]
    fn weighted_path_recovers_target_distances() {
        let mut req = Request::new(3, vec![[0, 1], [1, 2]], Algorithm::Stress);
        req.stress_method = StressMethod::Majorization;
        req.dim = 1;
        req.edge_weights = vec![2., 3.];
        req.initial = vec![Some(vec![0.]), Some(vec![1.]), Some(vec![2.])];
        let out = compute(&req).unwrap();
        assert!((out.positions[1][0] - out.positions[0][0] - 2.).abs() < 1e-9);
        assert!((out.positions[2][0] - out.positions[1][0] - 3.).abs() < 1e-9);
        assert!(out.objective.unwrap() < 1e-20);
    }

    #[test]
    fn constrained_majorization_decreases_stress_and_preserves_each_pin() {
        let mut req = Request::new(
            5,
            vec![[0, 1], [1, 2], [2, 3], [3, 4], [4, 0]],
            Algorithm::Stress,
        );
        req.stress_method = StressMethod::Majorization;
        req.dim = 3;
        req.initial = vec![
            Some(vec![0., 0., 0.]),
            Some(vec![1., 2., 3.]),
            Some(vec![-1., 1., 2.]),
            Some(vec![2., 0., 1.]),
            Some(vec![0., -2., 1.]),
        ];
        req.pins = vec![vec![true, true, true], vec![true, false, true]];
        req.iterations = Some(1);
        req.tolerance = 0.;
        let distances = Graph::new(&req).unwrap().distances();
        let mut old = objective(&req.initial_positions(), 3, &distances);
        for _ in 0..20 {
            let out = compute(&req).unwrap();
            assert_eq!(out.positions[0], vec![0., 0., 0.]);
            assert_eq!(out.positions[1][0], 1.);
            assert_eq!(out.positions[1][2], 3.);
            assert!(out.objective.unwrap() <= old + 1e-9);
            old = out.objective.unwrap();
            req.initial = out.positions.into_iter().map(Some).collect();
        }
    }

    #[test]
    fn coincident_free_node_separates_from_pin_in_either_order() {
        for pinned in 0..2 {
            let mut req = Request::new(2, vec![[0, 1]], Algorithm::Stress);
            req.stress_method = StressMethod::Majorization;
            req.initial = vec![Some(vec![0., 0.]); 2];
            req.pins = vec![vec![false; 2]; 2];
            req.pins[pinned].fill(true);
            let out = compute(&req).unwrap();
            assert_eq!(out.positions[pinned], vec![0., 0.]);
            assert!(out.objective.unwrap() < 1e-20);
        }
    }

    #[test]
    fn coincident_isolated_nodes_separate_without_nan() {
        let mut req = Request::new(4, vec![], Algorithm::Stress);
        req.stress_method = StressMethod::Majorization;
        req.initial = vec![Some(vec![0., 0.]); 4];
        let out = compute(&req).unwrap();
        assert!(out.objective.unwrap() < 0.3);
        assert!(out.positions.iter().flatten().all(|x| x.is_finite()));
    }
}
