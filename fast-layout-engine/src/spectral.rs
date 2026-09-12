//! Generalized spectral layout L v = lambda D v, as in NetworkLayout.jl.
use crate::{
    eigen::{self, Eigenpairs, NormalizedLaplacian},
    graph::Graph,
    Request, Response,
};
use nalgebra::{linalg::SymmetricEigen, DMatrix};

pub(crate) fn run(req: &Request) -> Result<Response, String> {
    let graph = Graph::new(req)?;
    let components = graph.components();
    let mut positions = vec![vec![0.; req.dim]; req.nodes];
    let mut local = vec![0; req.nodes];
    let mut offset = 0.;
    let mut iterations = 0;
    let mut converged = true;
    for nodes in &components {
        let n = nodes.len();
        if n > 1 {
            for (i, &node) in nodes.iter().enumerate() {
                local[node] = i;
            }
            let node_weight = |i: usize| req.node_weights.get(i).copied().unwrap_or(1.);
            let degree: Vec<f64> = nodes
                .iter()
                .map(|&i| {
                    graph
                        .neighbors(i)
                        .iter()
                        .map(|&(j, w)| w * (node_weight(i) * node_weight(j)).sqrt())
                        .sum()
                })
                .collect();
            let mut edges = Vec::new();
            for (a, &i) in nodes.iter().enumerate() {
                for &(j, w) in graph.neighbors(i) {
                    let b = local[j];
                    if a < b {
                        edges.push((
                            a,
                            b,
                            w * (node_weight(i) * node_weight(j)).sqrt()
                                / (degree[a] * degree[b]).sqrt(),
                        ));
                    }
                }
            }
            let matrix = NormalizedLaplacian { degree, edges };
            let count = req.dim.min(n - 1);
            let result = if n >= 128 && count + 4 < n / 2 {
                eigen::partial(
                    &matrix,
                    count,
                    req.iterations,
                    if req.tolerance == 0. {
                        1e-10
                    } else {
                        req.tolerance
                    },
                    req.seed,
                )?
            } else {
                let eigen = SymmetricEigen::try_new(matrix.dense(), 1e-13, 100 * n)
                    .ok_or("fast-layout-engine: spectral eigensolver did not converge; reduce edge weight ratios")?;
                let mut order: Vec<_> = (0..n).collect();
                order.sort_by(|&a, &b| eigen.eigenvalues[a].total_cmp(&eigen.eigenvalues[b]));
                Eigenpairs {
                    values: order[1..=count]
                        .iter()
                        .map(|&j| eigen.eigenvalues[j])
                        .collect(),
                    vectors: DMatrix::from_fn(n, count, |i, j| {
                        eigen.eigenvectors[(i, order[j + 1])]
                    }),
                    iterations: 0,
                    converged: true,
                }
            };
            iterations = iterations.max(result.iterations);
            converged &= result.converged;
            let mut applied = vec![0.; n];
            for (d, vector) in result.vectors.column_iter().take(count).enumerate() {
                matrix.apply(vector.as_slice(), &mut applied);
                let residual = applied
                    .iter()
                    .zip(vector.iter())
                    .map(|(a, x)| (a - result.values[d] * x).powi(2))
                    .sum::<f64>()
                    .sqrt();
                if result.iterations == 0 && residual > 1e-8 {
                    return Err("fast-layout-engine: spectral eigenpair residual exceeds 1e-8; reduce weight ratios".into());
                }
                let largest = (0..n)
                    .max_by(|&a, &b| vector[a].abs().total_cmp(&vector[b].abs()))
                    .unwrap();
                let sign = if vector[largest] < 0. { -1. } else { 1. };
                for (a, &i) in nodes.iter().enumerate() {
                    positions[i][d] = sign * vector[a] / matrix.degree[a].sqrt();
                }
            }
        }
        // Separate disconnected components in x, keeping each component's
        // internal embedding. Connected graphs retain their eigenvector coordinates.
        if components.len() > 1 {
            let min = nodes
                .iter()
                .map(|&i| positions[i][0])
                .fold(f64::INFINITY, f64::min);
            let max = nodes
                .iter()
                .map(|&i| positions[i][0])
                .fold(f64::NEG_INFINITY, f64::max);
            for &i in nodes {
                positions[i][0] += offset - min;
            }
            offset += max - min + 1.;
        }
    }
    Ok(Response {
        positions,
        iterations,
        converged,
        objective: None,
    })
}

#[cfg(test)]
mod tests {
    use crate::{compute, Algorithm, Request};

    fn weighted_sparse_request(nodes: usize, dim: usize) -> Request {
        let mut edges = Vec::new();
        let mut weights = Vec::new();
        for i in 0..nodes {
            for offset in [1, 7, 19] {
                let j = (i + offset) % nodes;
                if i < j {
                    edges.push([i, j]);
                    weights.push(0.5 + ((i * 17 + j * 11) % 23) as f64 / 10.0);
                }
            }
        }
        let mut request = Request::new(nodes, edges, Algorithm::Spectral);
        request.dim = dim;
        request.edge_weights = weights;
        request.node_weights = (0..nodes).map(|i| 0.75 + (i % 9) as f64 / 8.0).collect();
        request
    }

    fn max_normalized_residual(request: &Request, positions: &[Vec<f64>]) -> f64 {
        let n = request.nodes;
        let mut affinity = vec![0.0; n * n];
        for (edge, weight) in request.edges.iter().zip(&request.edge_weights) {
            let scaled =
                weight * (request.node_weights[edge[0]] * request.node_weights[edge[1]]).sqrt();
            affinity[edge[0] * n + edge[1]] = scaled;
            affinity[edge[1] * n + edge[0]] = scaled;
        }
        let degree: Vec<f64> = (0..n)
            .map(|i| affinity[i * n..(i + 1) * n].iter().sum())
            .collect();
        let mut worst: f64 = 0.0;
        for (d, _) in positions[0].iter().enumerate() {
            let vector: Vec<f64> = (0..n).map(|i| degree[i].sqrt() * positions[i][d]).collect();
            let applied: Vec<f64> = (0..n)
                .map(|i| {
                    vector[i]
                        - (0..n)
                            .map(|j| {
                                affinity[i * n + j] * vector[j] / (degree[i] * degree[j]).sqrt()
                            })
                            .sum::<f64>()
                })
                .collect();
            let lambda = vector.iter().zip(&applied).map(|(x, y)| x * y).sum::<f64>()
                / vector.iter().map(|x| x * x).sum::<f64>();
            let residual = applied
                .iter()
                .zip(&vector)
                .map(|(a, x)| (a - lambda * x).powi(2))
                .sum::<f64>()
                .sqrt();
            worst = worst.max(residual);
        }
        worst
    }

    #[test]
    fn disconnected_and_undersized_graphs_are_finite_and_separated() {
        let mut req = Request::new(5, vec![[0, 1], [2, 3]], Algorithm::Spectral);
        req.dim = 5;
        let p = compute(&req).unwrap().positions;
        assert!(p[0][0].max(p[1][0]) < p[2][0].min(p[3][0]));
        assert!(p[2][0].max(p[3][0]) < p[4][0]);
        assert!(p.iter().all(|p| p[1..].iter().all(|x| *x == 0.)));
    }
    #[test]
    fn repeated_eigenvalue_subspace_has_expected_cycle_geometry() {
        let req = Request::new(4, vec![[0, 1], [1, 2], [2, 3], [3, 0]], Algorithm::Spectral);
        let p = compute(&req).unwrap().positions;
        let distance = |i: usize, j: usize| {
            p[i].iter()
                .zip(&p[j])
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
        };
        for i in 0..4 {
            assert!((distance(i, (i + 1) % 4) - 0.5).abs() < 1e-10);
        }
        assert!((distance(0, 2) - 1.).abs() < 1e-10);
    }

    #[test]
    fn partial_weighted_layout_has_small_normalized_residual() {
        let mut request = weighted_sparse_request(160, 3);
        request.tolerance = 1e-8;
        let response = compute(&request).unwrap();
        assert!(max_normalized_residual(&request, &response.positions) <= 1e-7);
    }

    #[test]
    fn one_partial_iteration_reports_unconverged() {
        let mut request = weighted_sparse_request(160, 3);
        request.iterations = 1;
        let response = compute(&request).unwrap();
        assert_eq!(response.iterations, 1);
        assert!(!response.converged);
    }

    #[test]
    fn partial_layout_is_deterministic_for_the_same_seed() {
        let request = weighted_sparse_request(160, 3);
        assert_eq!(compute(&request).unwrap(), compute(&request).unwrap());
    }

    #[test]
    fn higher_dimensional_sparse_layout_is_finite_with_small_residual() {
        let mut request = weighted_sparse_request(160, 5);
        request.tolerance = 1e-8;
        let response = compute(&request).unwrap();
        assert!(response.positions.iter().flatten().all(|x| x.is_finite()));
        assert!(max_normalized_residual(&request, &response.positions) <= 1e-7);
    }
}
