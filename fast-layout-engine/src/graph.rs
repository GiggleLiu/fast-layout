use crate::Request;
use std::{cmp::Ordering, collections::BinaryHeap};

pub(crate) struct Graph {
    pub n: usize,
    pub edges: Vec<(usize, usize, f64)>,
    offsets: Vec<usize>,
    neighbors: Vec<(usize, f64)>,
}

impl Graph {
    pub fn new(req: &Request) -> Result<Self, String> {
        let mut edges: Vec<_> = req
            .edges
            .iter()
            .enumerate()
            .filter(|(_, e)| e[0] != e[1])
            .map(|(k, e)| {
                (
                    e[0].min(e[1]),
                    e[0].max(e[1]),
                    req.edge_weights.get(k).copied().unwrap_or(1.0),
                )
            })
            .collect();
        edges.sort_unstable_by_key(|&(a, b, _)| (a, b));
        for pair in edges.windows(2) {
            if pair[0].0 == pair[1].0 && pair[0].1 == pair[1].1 && pair[0].2 != pair[1].2 {
                return Err(
                    "fast-layout-engine: duplicate undirected edges have conflicting weights"
                        .into(),
                );
            }
        }
        edges.dedup();
        let mut offsets = vec![0; req.nodes + 1];
        for &(a, b, _) in &edges {
            offsets[a + 1] += 1;
            offsets[b + 1] += 1;
        }
        for i in 1..offsets.len() {
            offsets[i] += offsets[i - 1];
        }
        let mut cursor = offsets.clone();
        let mut neighbors = vec![(0, 0.0); 2 * edges.len()];
        for &(a, b, w) in &edges {
            neighbors[cursor[a]] = (b, w);
            cursor[a] += 1;
            neighbors[cursor[b]] = (a, w);
            cursor[b] += 1;
        }
        Ok(Self {
            n: req.nodes,
            edges,
            offsets,
            neighbors,
        })
    }

    pub fn neighbors(&self, node: usize) -> &[(usize, f64)] {
        &self.neighbors[self.offsets[node]..self.offsets[node + 1]]
    }

    pub fn components(&self) -> Vec<Vec<usize>> {
        let mut seen = vec![false; self.n];
        let mut components = Vec::new();
        for i in 0..self.n {
            if seen[i] {
                continue;
            }
            let mut queue = vec![i];
            seen[i] = true;
            let mut k = 0;
            while k < queue.len() {
                for &(j, _) in self.neighbors(queue[k]) {
                    if !seen[j] {
                        seen[j] = true;
                        queue.push(j);
                    }
                }
                k += 1;
            }
            components.push(queue);
        }
        components
    }

    pub fn distances(&self) -> Vec<f64> {
        let mut distances = vec![f64::INFINITY; self.n * self.n];
        let unit = self.edges.iter().all(|e| e.2 == 1.0);
        let mut queue = Vec::with_capacity(self.n);
        let mut heap = BinaryHeap::new();
        for source in 0..self.n {
            let row = &mut distances[source * self.n..(source + 1) * self.n];
            row[source] = 0.0;
            if unit {
                queue.clear();
                queue.push(source);
                let mut head = 0;
                while head < queue.len() {
                    let i = queue[head];
                    head += 1;
                    for &(j, _) in self.neighbors(i) {
                        if row[j].is_infinite() {
                            row[j] = row[i] + 1.0;
                            queue.push(j);
                        }
                    }
                }
            } else {
                heap.clear();
                heap.push(Visit(0.0, source));
                while let Some(Visit(d, i)) = heap.pop() {
                    if d > row[i] {
                        continue;
                    }
                    for &(j, w) in self.neighbors(i) {
                        if d + w < row[j] {
                            row[j] = d + w;
                            heap.push(Visit(d + w, j));
                        }
                    }
                }
            }
        }
        let max_distance = distances
            .iter()
            .copied()
            .filter(|d| d.is_finite())
            .fold(0.0, f64::max);
        if distances.iter().any(|d| d.is_infinite()) {
            let fallback = if max_distance == 0.0 {
                1.0
            } else {
                max_distance
            };
            let separation = fallback * (self.components().len() as f64).cbrt();
            for d in &mut distances {
                if d.is_infinite() {
                    *d = separation;
                }
            }
        }
        distances
    }
}

#[derive(PartialEq)]
struct Visit(f64, usize);
impl Eq for Visit {}
impl Ord for Visit {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .0
            .total_cmp(&self.0)
            .then_with(|| other.1.cmp(&self.1))
    }
}
impl PartialOrd for Visit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Algorithm;
    #[test]
    fn distances_match_julia_cycle_fixture_and_weighted_shortcuts() {
        let req = Request::new(
            5,
            vec![[0, 1], [0, 4], [1, 3], [2, 3], [2, 4]],
            Algorithm::Stress,
        );
        assert_eq!(
            Graph::new(&req).unwrap().distances(),
            vec![
                0., 1., 2., 2., 1., 1., 0., 2., 1., 2., 2., 2., 0., 1., 1., 2., 1., 1., 0., 2., 1.,
                2., 1., 2., 0.
            ]
        );
        let mut req = Request::new(3, vec![[0, 1], [1, 2], [0, 2]], Algorithm::Stress);
        req.edge_weights = vec![2., 3., 10.];
        assert_eq!(
            Graph::new(&req).unwrap().distances(),
            vec![0., 2., 5., 2., 0., 3., 5., 3., 0.]
        );
    }
}
