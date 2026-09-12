//! Fruchterman–Reingold forces and cooling adapted from NetworkLayout.jl.
use crate::{
    barnes_hut::{repel, Tree},
    graph::Graph,
    schema::{distance_squared, response},
    Request, Response,
};

pub(crate) fn run(req: &Request) -> Result<Response, String> {
    let graph = Graph::new(req)?;
    let dim = req.dim;
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
    let mut pair = vec![0.; dim];
    let mut converged = false;
    let mut iterations = 0;
    for iteration in 1..=req.iterations {
        forces.fill(0.);
        if let Some(tree) = tree.as_mut() {
            tree.rebuild(&p);
            tree.forces(&p, k * k, theta, req.seed, &mut forces);
        } else {
            for i in 0..n {
                for j in i + 1..n {
                    pair.fill(0.);
                    repel(&p, dim, i, j, k * k, req.seed, &mut pair);
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
    use crate::{compute, Algorithm, Request};
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
