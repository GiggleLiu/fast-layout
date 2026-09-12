use fast_layout_engine::{compute, Algorithm, Request};
use serde::Deserialize;
use serde_json::Value;

struct Fixture {
    request: Request,
    positions: Vec<Vec<f64>>,
    distances: Vec<f64>,
    objective: Option<f64>,
    eigenvalues: Vec<f64>,
}

fn fixture(text: &str) -> Fixture {
    let mut value: Value = serde_json::from_str(text).unwrap();
    let object = value.as_object_mut().unwrap();
    let positions = serde_json::from_value(object.remove("positions").unwrap()).unwrap();
    let distances = serde_json::from_value(object.remove("pairwise_distances").unwrap()).unwrap();
    let objective = serde_json::from_value(object.remove("objective").unwrap()).unwrap();
    let eigenvalues = object
        .remove("eigenvalues")
        .map(serde_json::from_value)
        .transpose()
        .unwrap()
        .unwrap_or_default();
    object.remove("name");
    object.remove("ideal_distances");
    object.remove("degree");
    Fixture {
        request: serde_json::from_value(value).unwrap(),
        positions,
        distances,
        objective,
        eigenvalues,
    }
}

fn pairwise(positions: &[Vec<f64>]) -> Vec<f64> {
    positions
        .iter()
        .flat_map(|a| {
            positions.iter().map(move |b| {
                a.iter()
                    .zip(b)
                    .map(|(x, y)| (x - y).powi(2))
                    .sum::<f64>()
                    .sqrt()
            })
        })
        .collect()
}

fn close(actual: &[f64], expected: &[f64], tolerance: f64) {
    assert_eq!(actual.len(), expected.len());
    let error = actual
        .iter()
        .zip(expected)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0, f64::max);
    assert!(
        error <= tolerance,
        "maximum error {error} exceeds {tolerance}"
    );
}

const SPRING_2D: &str = include_str!("fixtures/spring_exact_2d.json");
const SPRING_3D: &str = include_str!("fixtures/spring_exact_3d.json");
const STRESS_CONNECTED_2D: &str = include_str!("fixtures/stress_weighted_connected_2d.json");
const STRESS_CONNECTED_3D: &str = include_str!("fixtures/stress_weighted_connected_3d.json");
const STRESS_DISCONNECTED_2D: &str = include_str!("fixtures/stress_weighted_disconnected_2d.json");
const STRESS_DISCONNECTED_3D: &str = include_str!("fixtures/stress_weighted_disconnected_3d.json");
const SPECTRAL_2D: &str = include_str!("fixtures/spectral_weighted_2d.json");
const SPECTRAL_3D: &str = include_str!("fixtures/spectral_weighted_3d.json");
const SPECTRAL_5D: &str = include_str!("fixtures/spectral_weighted_5d.json");
const SHELL: &str = include_str!("fixtures/shell_geometry.json");
const BUCHHEIM: &str = include_str!("fixtures/buchheim_varying_size.json");
const BUCHHEIM_RANDOM: &str = include_str!("fixtures/buchheim_random.json");

#[test]
fn spring_matches_exact_fixed_initial_updates() {
    for text in [SPRING_2D, SPRING_3D] {
        let fixture = fixture(text);
        let response = compute(&fixture.request).unwrap();
        assert_eq!(response.iterations, fixture.request.iterations);
        assert!(!response.converged);
        close(
            &response.positions.concat(),
            &fixture.positions.concat(),
            2e-10,
        );
        close(&pairwise(&response.positions), &fixture.distances, 2e-10);
    }
}

#[test]
fn stress_matches_weighted_connected_and_disconnected_geometry() {
    for text in [
        STRESS_CONNECTED_2D,
        STRESS_CONNECTED_3D,
        STRESS_DISCONNECTED_2D,
        STRESS_DISCONNECTED_3D,
    ] {
        let fixture = fixture(text);
        let response = compute(&fixture.request).unwrap();
        assert_eq!(response.iterations, fixture.request.iterations);
        assert!(!response.converged);
        close(&pairwise(&response.positions), &fixture.distances, 2e-8);
        let scale = fixture.objective.unwrap().abs().max(1.0);
        assert!((response.objective.unwrap() - fixture.objective.unwrap()).abs() <= 2e-8 * scale);
    }
}

#[test]
fn spectral_matches_generalized_eigenvalues_and_residuals() {
    for text in [SPECTRAL_2D, SPECTRAL_3D, SPECTRAL_5D] {
        let fixture = fixture(text);
        let response = compute(&fixture.request).unwrap();
        let n = fixture.request.nodes;
        let mut adjacency = vec![0.0; n * n];
        for (edge, weight) in fixture
            .request
            .edges
            .iter()
            .zip(&fixture.request.edge_weights)
        {
            let factor = (fixture.request.node_weights[edge[0]]
                * fixture.request.node_weights[edge[1]])
                .sqrt();
            adjacency[edge[0] * n + edge[1]] = weight * factor;
            adjacency[edge[1] * n + edge[0]] = weight * factor;
        }
        let degree: Vec<f64> = (0..n)
            .map(|i| adjacency[i * n..(i + 1) * n].iter().sum())
            .collect();
        for d in 0..fixture.request.dim {
            let x: Vec<f64> = response.positions.iter().map(|p| p[d]).collect();
            let dx = degree
                .iter()
                .zip(&x)
                .map(|(degree, x)| degree * x)
                .collect::<Vec<_>>();
            let lx = (0..n)
                .map(|i| dx[i] - (0..n).map(|j| adjacency[i * n + j] * x[j]).sum::<f64>())
                .collect::<Vec<_>>();
            let lambda = x.iter().zip(&lx).map(|(a, b)| a * b).sum::<f64>()
                / x.iter().zip(&dx).map(|(a, b)| a * b).sum::<f64>();
            assert!((lambda - fixture.eigenvalues[d + 1]).abs() < 2e-9);
            let residual = lx
                .iter()
                .zip(&dx)
                .map(|(a, b)| (a - lambda * b).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(residual < 2e-9, "generalized eigen residual {residual}");
        }
        close(&pairwise(&response.positions), &fixture.distances, 2e-8);
    }
}

#[test]
fn shell_and_buchheim_match_upstream_geometry() {
    for text in [SHELL, BUCHHEIM] {
        let fixture = fixture(text);
        let response = compute(&fixture.request).unwrap();
        close(
            &response.positions.concat(),
            &fixture.positions.concat(),
            2e-12,
        );
        close(&pairwise(&response.positions), &fixture.distances, 2e-12);
    }
}

#[derive(Deserialize)]
struct TreeFixture {
    name: String,
    nodes: usize,
    edges: Vec<[usize; 2]>,
    node_sizes: Vec<f64>,
    positions: Vec<Vec<f64>>,
}

#[test]
fn buchheim_matches_varied_upstream_trees() {
    let fixtures: Vec<TreeFixture> = serde_json::from_str(BUCHHEIM_RANDOM).unwrap();
    assert!(fixtures.len() >= 20);
    for fixture in fixtures {
        let mut request = Request::new(fixture.nodes, fixture.edges, Algorithm::Buchheim);
        request.node_sizes = fixture.node_sizes;
        let response = compute(&request).unwrap();
        let actual = response.positions.concat();
        let expected = fixture.positions.concat();
        let error = actual
            .iter()
            .zip(&expected)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0, f64::max);
        assert!(error <= 2e-12, "{} maximum error {error}", fixture.name);
    }
}

#[test]
fn singleton_spring_is_finite() {
    let response = compute(&Request::new(1, Vec::new(), Algorithm::Spring)).unwrap();
    assert_eq!(response.positions.len(), 1);
    assert!(response.positions[0].iter().all(|x| x.is_finite()));
}

#[test]
fn original_jagmesh_fixture_is_intact() {
    let data = include_str!("../../upstream/NetworkLayout.jl/test/jagmesh1.mtx");
    let entries: Vec<[usize; 2]> = data
        .lines()
        .map(|line| {
            let mut values = line.split_whitespace().map(|x| x.parse().unwrap());
            [values.next().unwrap(), values.next().unwrap()]
        })
        .collect();
    assert_eq!(entries.len(), 3600);
    assert_eq!(entries.iter().flatten().copied().max(), Some(936));
}

fn jagmesh_request(dim: usize) -> Request {
    let data = include_str!("../../upstream/NetworkLayout.jl/test/jagmesh1.mtx");
    let edges = data
        .lines()
        .map(|line| {
            let mut values = line
                .split_whitespace()
                .map(|x| x.parse::<usize>().unwrap() - 1);
            [values.next().unwrap(), values.next().unwrap()]
        })
        .collect();
    let mut request = Request::new(936, edges, Algorithm::Stress);
    request.dim = dim;
    request.iterations = 10;
    request.tolerance = 0.0;
    request.initial = (1..=936)
        .map(|i| {
            Some(if dim == 2 {
                vec![(i as f64).sin(), (i as f64).cos()]
            } else {
                vec![(i as f64).sin(), (i as f64).cos(), (2.0 * i as f64).sin()]
            })
        })
        .collect();
    request
}

#[test]
#[ignore = "936-node dense reference case; run with cargo test --release --test upstream -- --ignored"]
fn jagmesh_stress_runs_in_2d_and_3d() {
    for dim in [2, 3] {
        let response = compute(&jagmesh_request(dim)).unwrap();
        assert_eq!(response.iterations, 10);
        assert!(response
            .objective
            .is_some_and(|x| x.is_finite() && x >= 0.0));
        let coordinates = response.positions.concat();
        let mean = coordinates.iter().sum::<f64>() / coordinates.len() as f64;
        let variance =
            coordinates.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / coordinates.len() as f64;
        assert!(variance > 1e-6, "jagmesh layout collapsed");
    }
}

#[test]
fn wheel_is_deterministic_with_fixed_initial_positions() {
    let edges = vec![
        [0, 1],
        [0, 2],
        [0, 3],
        [0, 4],
        [1, 2],
        [2, 3],
        [3, 4],
        [4, 1],
    ];
    for algorithm in [Algorithm::Stress, Algorithm::Spring] {
        let mut request = Request::new(5, edges.clone(), algorithm);
        request.iterations = 4;
        request.tolerance = 0.0;
        request.theta = (algorithm == Algorithm::Spring).then_some(0.0);
        request.initial = (1..=5)
            .map(|i| Some(vec![(i as f64).sin(), (i as f64).cos()]))
            .collect();
        let first = compute(&request).unwrap();
        let second = compute(&request).unwrap();
        assert_eq!(first, second);
        assert!(pairwise(&first.positions).iter().all(|x| x.is_finite()));
    }
}
