use fast_layout_engine::{compute, layout, Algorithm, Request, Response, StressMethod};

fn path_request(algorithm: Algorithm) -> Request {
    Request::new(4, vec![[0, 1], [1, 2], [2, 3]], algorithm)
}

fn assert_shape(response: &Response, nodes: usize, dim: usize) {
    assert_eq!(response.positions.len(), nodes);
    assert!(response.positions.iter().all(|p| p.len() == dim));
    assert!(response.positions.iter().flatten().all(|x| x.is_finite()));
}

#[test]
fn all_layouts_return_finite_positions_in_node_order() {
    for algorithm in [Algorithm::Stress, Algorithm::Spring, Algorithm::Spectral] {
        assert_shape(&compute(&path_request(algorithm)).unwrap(), 4, 2);
    }
    assert_shape(
        &compute(&Request::new(4, Vec::new(), Algorithm::Shell)).unwrap(),
        4,
        2,
    );
    assert_shape(&compute(&path_request(Algorithm::Buchheim)).unwrap(), 4, 2);
}

#[test]
fn pins_are_exact_for_iterative_layouts() {
    for algorithm in [Algorithm::Stress, Algorithm::Spring] {
        let mut request = path_request(algorithm);
        request.initial = (0..4)
            .map(|i| {
                Some(if i == 0 {
                    vec![3.0, -2.0]
                } else {
                    vec![i as f64, -(i as f64)]
                })
            })
            .collect();
        request.pins = vec![vec![true, true]];
        assert_eq!(compute(&request).unwrap().positions[0], vec![3.0, -2.0]);
    }
}

#[test]
fn rejects_adversarial_inputs_before_allocating() {
    let mut request = Request::new(2, vec![[0, 2]], Algorithm::Stress);
    assert!(compute(&request).unwrap_err().contains("edge endpoint"));
    request.edges.clear();
    request.tolerance = f64::NAN;
    assert!(compute(&request).unwrap_err().contains("tolerance"));
    request.tolerance = 1e-5;
    request.nodes = 1_000_000;
    request.dim = 5;
    assert!(compute(&request)
        .unwrap_err()
        .contains("4,000,000 coordinates"));
}

#[test]
fn cbor_matches_native_and_is_byte_deterministic() {
    for stress_method in [StressMethod::Majorization, StressMethod::Sgd] {
        let mut request = path_request(Algorithm::Stress);
        request.stress_method = stress_method;
        let native = compute(&request).unwrap();
        let mut input = Vec::new();
        ciborium::into_writer(&request, &mut input).unwrap();
        let first = layout(&input).unwrap();
        let second = layout(&input).unwrap();
        assert_eq!(first, second);
        let decoded: Response = ciborium::from_reader(first.as_slice()).unwrap();
        assert_eq!(decoded, native);
    }
}

#[test]
fn rejects_unknown_or_inapplicable_stress_methods() {
    let mut request = path_request(Algorithm::Spring);
    request.stress_method = StressMethod::Sgd;
    assert!(compute(&request)
        .unwrap_err()
        .contains("stress_method is only supported for stress"));
    let value = serde_json::json!({"nodes": 1, "stress_method": "typo"});
    let mut input = Vec::new();
    ciborium::into_writer(&value, &mut input).unwrap();
    assert!(layout(&input).unwrap_err().contains("unknown variant"));
}

#[test]
fn cbor_rejects_unknown_fields_and_trailing_data() {
    let value = serde_json::json!({"nodes": 1, "edges": [], "algorithm": "stress", "extra": true});
    let mut input = Vec::new();
    ciborium::into_writer(&value, &mut input).unwrap();
    assert!(layout(&input).unwrap_err().contains("unknown field"));

    input.clear();
    ciborium::into_writer(&Request::default(), &mut input).unwrap();
    input.push(0);
    assert!(layout(&input).unwrap_err().contains("trailing bytes"));
}

#[test]
fn shell_and_tree_keep_basic_geometry_invariants() {
    let shell = compute(&Request::new(4, Vec::new(), Algorithm::Shell)).unwrap();
    let radii: Vec<_> = shell.positions.iter().map(|p| p[0].hypot(p[1])).collect();
    assert!(radii.iter().all(|r| (r - radii[0]).abs() < 1e-12));

    let tree = compute(&Request::new(3, vec![[0, 1], [0, 2]], Algorithm::Buchheim)).unwrap();
    assert!(tree.positions[1][0] < tree.positions[2][0]);
    assert!(
        (tree.positions[0][0] - (tree.positions[1][0] + tree.positions[2][0]) / 2.0).abs() < 1e-12
    );
}

#[test]
fn numeric_edges_normalize_direction_duplicates_and_loops() {
    for algorithm in [Algorithm::Stress, Algorithm::Spring, Algorithm::Spectral] {
        let base = path_request(algorithm);
        let expected = compute(&base).unwrap();
        let mut repeated = base.clone();
        repeated.edges.extend([[1, 0], [1, 2], [2, 2]]);
        assert_eq!(compute(&repeated).unwrap(), expected);
        repeated.edge_weights = vec![1.; repeated.edges.len()];
        repeated.edge_weights[3] = 2.;
        assert!(compute(&repeated).unwrap_err().contains("conflicting"));
    }
}

#[test]
fn tree_rejects_invalid_root_and_root_with_parent() {
    let mut request = path_request(Algorithm::Buchheim);
    request.root = 4;
    assert!(compute(&request).unwrap_err().contains("root"));
    request.root = 1;
    assert!(compute(&request).unwrap_err().contains("root"));
}
