//! Pairwise stress SGD, Zheng, Pawar & Goodman (2019), equations 7–12.
//! Independently implemented from https://arxiv.org/abs/1710.04626.
use crate::{
    graph::Graph,
    schema::{response, Rng},
    stress, Request, Response,
};

pub(crate) fn run(req: &Request) -> Result<Response, String> {
    match req.dim {
        2 => optimize::<2>(req),
        3 => optimize::<3>(req),
        _ => optimize::<0>(req),
    }
}

fn optimize<const D: usize>(req: &Request) -> Result<Response, String> {
    let dim = if D == 0 { req.dim } else { D };
    let n = req.nodes;
    let graph = Graph::new(req)?;
    let mut positions = req.initial_positions();
    if n < 2 {
        return Ok(response(positions, dim, 0, true, Some(0.)));
    }
    let distances = graph.distances();
    stress::separate_coincident(req, &mut positions, &distances);
    let pinned: Vec<_> = (0..n)
        .flat_map(|i| (0..dim).map(move |d| req.pinned(i, d)))
        .collect();
    let mut pairs = Vec::with_capacity(n * (n - 1) / 2);
    let mut min_weight = f64::INFINITY;
    let mut max_weight: f64 = 0.;
    for i in 0..n {
        for j in i + 1..n {
            let distance = distances[i * n + j];
            let weight = 1. / (distance * distance);
            min_weight = min_weight.min(weight);
            max_weight = max_weight.max(weight);
            pairs.push((i, j, distance, weight));
        }
    }
    if pinned.iter().all(|x| *x) {
        let objective = stress::objective(&positions, dim, &distances);
        return Ok(response(positions, dim, 0, true, Some(objective)));
    }
    let unpinned = pinned.iter().all(|x| !x);
    let mut order: Vec<_> = (0..pairs.len()).collect();
    let mut rng = Rng::new(req.seed);
    // Exponential schedule, from 1/min(weight) to epsilon/max(weight).
    // epsilon=0.1 is the paper's fixed-budget recommendation, not a convergence test.
    let eta_max = 1. / min_weight;
    let eta_min = 0.1 / max_weight;
    let decay = if req.iterations > 1 {
        (eta_min / eta_max).ln() / (req.iterations - 1) as f64
    } else {
        0.
    };
    let mut iterations = 0;
    let mut converged = false;
    for iteration in 0..req.iterations {
        for k in (1..order.len()).rev() {
            let j = (rng.uniform() * (k + 1) as f64) as usize;
            order.swap(k, j);
        }
        let eta = eta_max * (decay * iteration as f64).exp();
        let mut max_move2: f64 = 0.;
        for &pair in &order {
            let (i, j, target, weight) = pairs[pair];
            if D == 2 && unpinned {
                max_move2 = max_move2.max(step_2d(
                    &mut positions,
                    i,
                    j,
                    target,
                    (eta * weight).min(1.),
                ));
                continue;
            }
            let i = i * dim;
            let j = j * dim;
            let mut distance2 = 0.;
            for d in 0..dim {
                distance2 += (positions[i + d] - positions[j + d]).powi(2);
            }
            let distance = distance2.sqrt();
            if distance == 0. {
                continue;
            }
            let scale = (eta * weight).min(1.) * (distance - target) / distance;
            let mut moved_i = 0.;
            let mut moved_j = 0.;
            for d in 0..dim {
                let movable = usize::from(!pinned[i + d]) + usize::from(!pinned[j + d]);
                if movable == 0 {
                    continue;
                }
                let step = scale * (positions[i + d] - positions[j + d]) / movable as f64;
                if !pinned[i + d] {
                    positions[i + d] -= step;
                    moved_i += step * step;
                }
                if !pinned[j + d] {
                    positions[j + d] += step;
                    moved_j += step * step;
                }
            }
            max_move2 = max_move2.max(moved_i).max(moved_j);
        }
        iterations = iteration + 1;
        // Stopping applies only once the schedule no longer clips any pair step.
        if req.tolerance > 0. && eta * max_weight <= 1. && max_move2.sqrt() <= req.tolerance {
            converged = true;
            break;
        }
    }
    let objective = stress::objective(&positions, dim, &distances);
    Ok(response(
        positions,
        dim,
        iterations,
        converged,
        Some(objective),
    ))
}

// Fixed-size point access lets the compiler remove per-coordinate bounds checks.
#[inline]
fn step_2d(positions: &mut [f64], i: usize, j: usize, target: f64, mu: f64) -> f64 {
    let (points, _) = positions.as_chunks_mut::<2>();
    let (left, right) = points.split_at_mut(j);
    let a = &mut left[i];
    let b = &mut right[0];
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let distance = (dx * dx + dy * dy).sqrt();
    if distance == 0. {
        return 0.;
    }
    let scale = 0.5 * mu * (distance - target) / distance;
    let x = scale * dx;
    let y = scale * dy;
    a[0] -= x;
    a[1] -= y;
    b[0] += x;
    b[1] += y;
    x * x + y * y
}

#[cfg(test)]
mod tests {
    use crate::{compute, Algorithm, Request, StressMethod};
    #[test]
    fn weighted_pair_converges_and_preserves_pins_in_any_dimension() {
        for dim in [1, 2, 3, 5] {
            for pin in 0..2 {
                let mut req = Request::new(2, vec![[0, 1]], Algorithm::Stress);
                req.dim = dim;
                req.stress_method = StressMethod::Sgd;
                req.edge_weights = vec![3.];
                req.initial = vec![Some(vec![0.; dim]); 2];
                req.pins = vec![vec![false; dim]; 2];
                req.pins[pin].fill(true);
                req.iterations = 15;
                let result = compute(&req).unwrap();
                assert_eq!(result.positions[pin], vec![0.; dim]);
                assert!(result.objective.unwrap() < 1e-20);
                assert_eq!(result, compute(&req).unwrap());
            }
        }
    }
    #[test]
    fn fixed_budget_does_not_claim_convergence() {
        let mut req = Request::new(
            5,
            vec![[0, 1], [1, 2], [2, 3], [3, 4], [4, 0]],
            Algorithm::Stress,
        );
        req.stress_method = StressMethod::Sgd;
        req.iterations = 15;
        req.tolerance = 0.;
        let result = compute(&req).unwrap();
        assert_eq!(result.iterations, 15);
        assert!(!result.converged);
        assert!(result.objective.unwrap() < 0.5);
    }

    #[test]
    fn weighted_path_respects_partial_pins_and_disconnected_nodes() {
        for dim in [1, 2, 3, 5] {
            let mut req = Request::new(3, vec![[0, 1], [1, 2]], Algorithm::Stress);
            req.stress_method = StressMethod::Sgd;
            req.dim = dim;
            req.edge_weights = vec![2., 3.];
            req.initial = vec![Some(vec![0.; dim]); 3];
            req.initial[2].as_mut().unwrap()[0] = 5.;
            req.pins = vec![vec![true; dim], vec![false; dim], vec![false; dim]];
            req.pins[2][0] = true;
            req.iterations = 100;
            let result = compute(&req).unwrap();
            assert_eq!(result.positions[0], vec![0.; dim]);
            assert_eq!(result.positions[2][0], 5.);
            assert!(result.objective.unwrap() < 1e-4, "{result:?}");

            req.nodes = 4; // An isolated node exercises finite fallback distances.
            let result = compute(&req).unwrap();
            assert_eq!(result.positions[0], vec![0.; dim]);
            assert_eq!(result.positions[2][0], 5.);
            assert!(result.positions[3].iter().any(|x| x.abs() > 1.));
            assert!(result.objective.unwrap().is_finite());
            assert_eq!(result, compute(&req).unwrap());

            req.nodes = 3;
            req.pins = vec![vec![true; dim]; 3];
            let result = compute(&req).unwrap();
            assert_eq!(result.iterations, 0);
            assert!(result.converged);
            assert_eq!(
                result.positions,
                req.initial
                    .into_iter()
                    .map(Option::unwrap)
                    .collect::<Vec<_>>()
            );
        }
    }
}
