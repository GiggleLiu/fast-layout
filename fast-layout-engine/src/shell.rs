use std::f64::consts::TAU;

pub(crate) fn run(req: &crate::Request) -> Result<crate::Response, String> {
    if req.dim != 2 {
        return Err("shell layout requires dim = 2".into());
    }
    if req.nodes == 0 {
        if req.shells.is_empty() {
            return Ok(response(Vec::new()));
        }
        if let Some(group) = req.shells.iter().position(Vec::is_empty) {
            return Err(format!("shell group {group} is empty"));
        }
        return Err("shell nodes are out of range because the graph is empty".into());
    }

    let mut shells = req.shells.clone();
    let mut listed = vec![false; req.nodes];
    for (group, nodes) in shells.iter().enumerate() {
        if nodes.is_empty() {
            return Err(format!("shell group {group} is empty"));
        }
        for &node in nodes {
            if node >= req.nodes {
                return Err(format!(
                    "shell node {node} is out of range for {} nodes",
                    req.nodes
                ));
            }
            if std::mem::replace(&mut listed[node], true) {
                return Err(format!("shell node {node} appears more than once"));
            }
        }
    }
    let missing: Vec<_> = (0..req.nodes).filter(|&node| !listed[node]).collect();
    if !missing.is_empty() {
        shells.push(missing);
    }

    let mut positions = vec![vec![0.0; 2]; req.nodes];
    let mut radius = if shells[0].len() > 1 { 1.0 } else { 0.0 };
    for nodes in shells {
        let count = nodes.len() as f64;
        for (i, node) in nodes.into_iter().enumerate() {
            let angle = TAU * i as f64 / count;
            positions[node] = vec![radius * angle.cos(), radius * angle.sin()];
        }
        radius += 1.0;
    }
    Ok(response(positions))
}

fn response(positions: Vec<Vec<f64>>) -> crate::Response {
    crate::Response {
        positions,
        iterations: 0,
        converged: true,
        objective: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Algorithm;

    fn request(nodes: usize, shells: Vec<Vec<usize>>) -> crate::Request {
        let mut request = crate::Request::new(nodes, Vec::new(), Algorithm::Shell);
        request.shells = shells;
        request
    }

    #[test]
    fn places_shells_and_appends_missing_nodes() {
        let result = run(&request(5, vec![vec![2], vec![0, 4]])).unwrap();
        assert_eq!(result.positions[2], vec![0.0, 0.0]);
        assert_eq!(result.positions[0], vec![1.0, 0.0]);
        assert!((result.positions[4][0] + 1.0).abs() < 1e-12);
        assert_eq!(result.positions[1], vec![2.0, 0.0]);
        assert!((result.positions[3][0] + 2.0).abs() < 1e-12);
    }

    #[test]
    fn starts_a_multi_node_inner_shell_at_radius_one() {
        let result = run(&request(2, vec![vec![0, 1]])).unwrap();
        assert_eq!(result.positions[0], vec![1.0, 0.0]);
        assert!((result.positions[1][0] + 1.0).abs() < 1e-12);
    }

    #[test]
    fn handles_empty_graph_and_rejects_bad_groups() {
        assert!(run(&request(0, vec![])).unwrap().positions.is_empty());
        assert!(run(&request(2, vec![vec![]]))
            .unwrap_err()
            .contains("empty"));
        assert!(run(&request(2, vec![vec![0, 0]]))
            .unwrap_err()
            .contains("more than once"));
        assert!(run(&request(2, vec![vec![2]]))
            .unwrap_err()
            .contains("out of range"));
    }
}
