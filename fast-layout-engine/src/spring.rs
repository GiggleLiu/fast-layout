//! Fruchterman–Reingold forces and cooling adapted from NetworkLayout.jl.
use crate::{
    barnes_hut::{repel, Tree},
    graph::Graph,
    schema::{distance_squared, response},
    Request, Response,
};

pub(crate) fn run(req: &Request) -> Result<Response, String> {
    match req.dim {
        2 => run_dim::<2>(req),
        3 => run_dim::<3>(req),
        _ => run_dim::<0>(req),
    }
}

fn run_dim<const D: usize>(req: &Request) -> Result<Response, String> {
    let graph = Graph::new(req)?;
    let dim = if D == 0 { req.dim } else { D };
    let n = req.nodes;
    let mut p = req.initial_positions();
    if n < 2 {
        return Ok(response(p, dim, 0, true, Some(0.)));
    }
    let k = req.c.unwrap_or(2.) * (4. / n as f64).sqrt();
    let temperature = req.temperature.unwrap_or(2.);
    let theta = req.theta.unwrap_or(if n >= 256 { 0.7 } else { 0. });
    let mut tree = if theta > 0. && matches!(dim, 2 | 3) {
        Some(Tree::new(dim, n))
    } else {
        None
    };
    let mut forces = vec![0.; p.len()];
    let mut pair = (D != 2).then(|| vec![0.; dim]);
    let mut converged = false;
    let mut iterations = 0;
    for iteration in 1..=req.iterations {
        forces.fill(0.);
        if let Some(tree) = tree.as_mut() {
            tree.rebuild(&p);
            tree.forces(&p, k * k, theta, req.seed, &mut forces);
        } else if D == 2 {
            repel_exact_2d(&p, k * k, req.seed, &mut forces);
        } else {
            let pair = pair.as_mut().unwrap();
            for i in 0..n {
                for j in i + 1..n {
                    pair.fill(0.);
                    repel(&p, dim, i, j, k * k, req.seed, pair);
                    for d in 0..dim {
                        forces[i * dim + d] += pair[d];
                        forces[j * dim + d] -= pair[d];
                    }
                }
            }
        }
        // As in Julia Spring, connectivity determines attraction; edge magnitudes
        // are deliberately ignored. Stress and Spectral use their weights.
        for &(i, j, _) in &graph.edges {
            let dist =
                distance_squared(&p[i * dim..(i + 1) * dim], &p[j * dim..(j + 1) * dim]).sqrt();
            for d in 0..dim {
                let f = (dist / k) * (p[j * dim + d] - p[i * dim + d]);
                forces[i * dim + d] += f;
                forces[j * dim + d] -= f;
            }
        }
        let mut max_move: f64 = 0.;
        for i in 0..n {
            let norm = forces[i * dim..(i + 1) * dim]
                .iter()
                .map(|x| x * x)
                .sum::<f64>()
                .sqrt();
            if !norm.is_finite() {
                return Err(
                    "fast-layout-engine: spring forces overflowed; reduce coordinate magnitudes"
                        .into(),
                );
            }
            if norm == 0. {
                continue;
            }
            let scale = (temperature / iteration as f64).min(norm) / norm;
            let mut moved = 0.;
            for d in 0..dim {
                if !req.pinned(i, d) {
                    let delta = forces[i * dim + d] * scale;
                    p[i * dim + d] += delta;
                    moved += delta * delta;
                }
            }
            max_move = max_move.max(moved.sqrt());
        }
        iterations = iteration;
        if req.tolerance > 0. && max_move <= req.tolerance {
            converged = true;
            break;
        }
    }
    // Exact energy would reintroduce O(n²) work into the accelerated path.
    // Report it only for exact runs; approximation quality is benchmarked separately.
    let energy = if tree.is_none() {
        Some(energy(&p, dim, &graph, k))
    } else {
        None
    };
    Ok(response(p, dim, iterations, converged, energy))
}

fn repel_exact_2d(points: &[f64], k2: f64, seed: u64, forces: &mut [f64]) {
    let (points, point_remainder) = points.as_chunks::<2>();
    let (forces, force_remainder) = forces.as_chunks_mut::<2>();
    debug_assert!(point_remainder.is_empty() && force_remainder.is_empty());

    for (i, &[x, y]) in points.iter().enumerate() {
        let (head, tail) = forces.split_at_mut(i + 1);
        let force_i = &mut head[i];
        for (offset, force_j) in tail.iter_mut().enumerate() {
            let j = i + offset + 1;
            let dx = x - points[j][0];
            let dy = y - points[j][1];
            let dist2 = dx.powi(2) + dy.powi(2);
            let pair = if dist2 == 0. {
                let mut pair = [0.; 2];
                repel(points.as_flattened(), 2, i, j, k2, seed, &mut pair);
                pair
            } else {
                let scale = k2 / dist2;
                [scale * dx, scale * dy]
            };
            force_i[0] += pair[0];
            force_i[1] += pair[1];
            force_j[0] -= pair[0];
            force_j[1] -= pair[1];
        }
    }
}

fn energy(p: &[f64], dim: usize, graph: &Graph, k: f64) -> f64 {
    let mut energy = 0.;
    for i in 0..graph.n {
        for j in i + 1..graph.n {
            let dist = distance_squared(&p[i * dim..(i + 1) * dim], &p[j * dim..(j + 1) * dim])
                .sqrt()
                .max(1e-15);
            energy -= k * k * dist.ln();
        }
    }
    for &(i, j, _) in &graph.edges {
        let dist = distance_squared(&p[i * dim..(i + 1) * dim], &p[j * dim..(j + 1) * dim]).sqrt();
        energy += dist.powi(3) / (3. * k);
    }
    energy
}

#[cfg(test)]
mod tests {
    use super::repel_exact_2d;
    use crate::barnes_hut::repel;
    use crate::{compute, Algorithm, Request};

    #[test]
    fn exact_2d_forces_match_generic_including_coincident_pairs() {
        let points = [0., 0., 2., -1., 0., 0., -3., 4.];
        let mut expected = [0.; 8];
        for i in 0..4 {
            for j in i + 1..4 {
                let mut pair = [0.; 2];
                repel(&points, 2, i, j, 1.7, 42, &mut pair);
                for d in 0..2 {
                    expected[i * 2 + d] += pair[d];
                    expected[j * 2 + d] -= pair[d];
                }
            }
        }
        let mut actual = [0.; 8];
        repel_exact_2d(&points, 1.7, 42, &mut actual);
        assert_eq!(actual, expected);
    }

    #[test]
    fn isolated_coincident_nodes_separate_with_pins_and_higher_dimensions() {
        let mut req = Request::new(4, vec![], Algorithm::Spring);
        req.dim = 5;
        req.initial = vec![Some(vec![0.; 5]); 4];
        req.pins = vec![vec![true; 5]];
        let p = compute(&req).unwrap().positions;
        assert_eq!(p[0], vec![0.; 5]);
        for point in p.iter().skip(1) {
            assert!(point.iter().any(|x| x.abs() > 0.1));
        }
    }
}
