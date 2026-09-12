const NONE: usize = usize::MAX;

pub(crate) fn run(req: &crate::Request) -> Result<crate::Response, String> {
    if req.dim != 2 {
        return Err("buchheim layout requires dim = 2".into());
    }
    let n = req.nodes;
    if n == 0 {
        if !req.edges.is_empty() {
            return Err("buchheim edges cannot reference an empty graph".into());
        }
        return Ok(response(Vec::new()));
    }
    if req.root >= n {
        return Err(format!(
            "buchheim root {} is out of range for {n} nodes",
            req.root
        ));
    }

    let mut children = vec![Vec::new(); n];
    let mut parent = vec![NONE; n];
    let mut sibling = vec![0; n];
    for &[from, to] in &req.edges {
        if from >= n || to >= n {
            return Err(format!(
                "buchheim edge [{from}, {to}] is out of range for {n} nodes"
            ));
        }
        if parent[to] != NONE {
            return Err(format!("buchheim node {to} has multiple parents"));
        }
        sibling[to] = children[from].len();
        parent[to] = from;
        children[from].push(to);
    }
    if parent[req.root] != NONE {
        return Err(format!("buchheim root {} must not have a parent", req.root));
    }
    for (node, &p) in parent.iter().enumerate() {
        if node != req.root && p == NONE {
            return Err(format!(
                "buchheim node {node} is not reachable from root {}",
                req.root
            ));
        }
    }

    // Parent uniqueness alone does not exclude a disconnected directed cycle.
    let mut seen = vec![false; n];
    let mut reached = 0;
    let mut stack = vec![req.root];
    while let Some(v) = stack.pop() {
        if seen[v] {
            return Err(format!("buchheim graph contains a cycle through node {v}"));
        }
        seen[v] = true;
        reached += 1;
        stack.extend(children[v].iter().rev().copied());
    }
    if reached != n {
        let node = seen.iter().position(|&x| !x).unwrap();
        return Err(format!("buchheim node {node} is not reachable from root {} (the disconnected component may contain a cycle)", req.root));
    }

    if req
        .node_sizes
        .iter()
        .any(|size| !size.is_finite() || *size <= 0.0)
    {
        return Err("buchheim node sizes must be finite and positive".into());
    }
    let mut size = vec![1.0; n];
    for (dst, &src) in size.iter_mut().zip(&req.node_sizes) {
        *dst = src;
    }
    let mut tree = Tree::new(children, parent, sibling, size);
    tree.first_walk(req.root);

    let mut positions = vec![vec![0.0; 2]; n];
    let mut stack = vec![(req.root, -tree.prelim[req.root], 0.0)];
    while let Some((v, m, depth)) = stack.pop() {
        positions[v] = vec![tree.prelim[v] + m, -depth];
        let max_child = tree.children[v]
            .iter()
            .map(|&w| tree.size[w])
            .fold(0.0, f64::max);
        for &w in tree.children[v].iter().rev() {
            stack.push((w, m + tree.modifier[v], depth + 1.0 + max_child));
        }
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

struct Tree {
    children: Vec<Vec<usize>>,
    parent: Vec<usize>,
    sibling: Vec<usize>,
    size: Vec<f64>,
    modifier: Vec<f64>,
    thread: Vec<usize>,
    ancestor: Vec<usize>,
    prelim: Vec<f64>,
    shift: Vec<f64>,
    change: Vec<f64>,
}

impl Tree {
    fn new(
        children: Vec<Vec<usize>>,
        parent: Vec<usize>,
        sibling: Vec<usize>,
        size: Vec<f64>,
    ) -> Self {
        let n = children.len();
        Self {
            children,
            parent,
            sibling,
            size,
            modifier: vec![0.0; n],
            thread: vec![NONE; n],
            ancestor: (0..n).collect(),
            prelim: vec![0.0; n],
            shift: vec![0.0; n],
            change: vec![0.0; n],
        }
    }

    fn first_walk(&mut self, root: usize) {
        enum Event {
            Enter(usize),
            Apportion(usize, usize),
            Finish(usize),
        }

        let mut default_ancestor = vec![NONE; self.children.len()];
        let mut stack = vec![Event::Enter(root)];
        while let Some(event) = stack.pop() {
            match event {
                Event::Enter(v) if self.children[v].is_empty() => {
                    if self.sibling[v] > 0 {
                        let left = self.children[self.parent[v]][self.sibling[v] - 1];
                        self.prelim[v] = self.prelim[left] + self.size[left];
                    }
                }
                Event::Enter(v) => {
                    default_ancestor[v] = self.children[v][0];
                    stack.push(Event::Finish(v));
                    for &w in self.children[v].iter().rev() {
                        stack.push(Event::Apportion(v, w));
                        stack.push(Event::Enter(w));
                    }
                }
                Event::Apportion(v, w) => {
                    default_ancestor[v] = self.apportion(w, default_ancestor[v]);
                }
                Event::Finish(v) => self.finish_walk(v),
            }
        }
    }

    fn finish_walk(&mut self, v: usize) {
        let first = self.children[v][0];
        let last = *self.children[v].last().unwrap();
        self.execute_shifts(v);
        let midpoint = (self.prelim[first] + self.prelim[last]) / 2.0;
        if self.sibling[v] > 0 {
            let left = self.children[self.parent[v]][self.sibling[v] - 1];
            self.prelim[v] = self.prelim[left] + self.size[left] + 1.0;
            self.modifier[v] = self.prelim[v] - midpoint;
        } else {
            self.prelim[v] = midpoint;
        }
    }

    fn apportion(&mut self, v: usize, mut default_ancestor: usize) -> usize {
        if self.sibling[v] == 0 {
            return default_ancestor;
        }
        let p = self.parent[v];
        let w = self.children[p][self.sibling[v] - 1];
        let (mut vir, mut vor, mut vil, mut vol) = (v, v, w, self.children[p][0]);
        let (mut sir, mut sor, mut sil, mut sol) = (
            self.modifier[vir],
            self.modifier[vor],
            self.modifier[vil],
            self.modifier[vol],
        );
        while self.next_right(vil) != NONE && self.next_left(vir) != NONE {
            vil = self.next_right(vil);
            vir = self.next_left(vir);
            vol = self.next_left(vol);
            vor = self.next_right(vor);
            self.ancestor[vor] = v;
            let shift = self.prelim[vil] + sil - self.prelim[vir] - sir + self.size[vil];
            if shift > 0.0 {
                let ancestor = self.find_ancestor(vil, v, default_ancestor);
                self.move_subtree(ancestor, v, shift);
                sir += shift;
                sor += shift;
            }
            sil += self.modifier[vil];
            sir += self.modifier[vir];
            sol += self.modifier[vol];
            sor += self.modifier[vor];
        }
        if self.next_right(vil) != NONE && self.next_right(vor) == NONE {
            self.thread[vor] = self.next_right(vil);
            self.modifier[vor] += sil - sor;
        } else if self.next_left(vir) != NONE && self.next_left(vol) == NONE {
            self.thread[vol] = self.next_left(vir);
            self.modifier[vol] += sir - sol;
            default_ancestor = v;
        }
        default_ancestor
    }

    fn next_left(&self, v: usize) -> usize {
        self.children[v].first().copied().unwrap_or(self.thread[v])
    }
    fn next_right(&self, v: usize) -> usize {
        self.children[v].last().copied().unwrap_or(self.thread[v])
    }
    fn find_ancestor(&self, w: usize, v: usize, default: usize) -> usize {
        let a = self.ancestor[w];
        if self.parent[a] == self.parent[v] {
            a
        } else {
            default
        }
    }
    fn move_subtree(&mut self, left: usize, right: usize, amount: f64) {
        let count = (self.sibling[right] - self.sibling[left]) as f64;
        self.change[right] -= amount / count;
        self.shift[right] += amount;
        self.change[left] += amount / count;
        self.prelim[right] += amount;
        self.modifier[right] += amount;
    }
    fn execute_shifts(&mut self, v: usize) {
        let (mut shift, mut change) = (0.0, 0.0);
        for &w in self.children[v].iter().rev() {
            self.prelim[w] += shift;
            self.modifier[w] += shift;
            change += self.change[w];
            shift += self.shift[w] + change;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Algorithm;

    fn request(nodes: usize, edges: Vec<[usize; 2]>) -> crate::Request {
        crate::Request::new(nodes, edges, Algorithm::Buchheim)
    }

    #[test]
    fn balanced_seven_node_tree_has_stable_geometry() {
        let result = run(&request(
            7,
            vec![[0, 1], [0, 2], [1, 3], [1, 4], [2, 5], [2, 6]],
        ))
        .unwrap();
        assert_eq!(
            result.positions,
            vec![
                vec![0.0, 0.0],
                vec![-1.0, -2.0],
                vec![1.0, -2.0],
                vec![-1.5, -4.0],
                vec![-0.5, -4.0],
                vec![0.5, -4.0],
                vec![1.5, -4.0],
            ]
        );
    }

    #[test]
    fn centers_parents_and_preserves_sibling_order() {
        let mut req = request(4, vec![[2, 1], [2, 3], [2, 0]]);
        req.root = 2;
        let result = run(&req).unwrap();
        assert_eq!(result.positions[2][0], 0.0);
        assert!(result.positions[1][0] < result.positions[3][0]);
        assert!(result.positions[3][0] < result.positions[0][0]);
        assert_eq!(
            result.positions[2][0],
            (result.positions[1][0] + result.positions[0][0]) / 2.0
        );
    }

    #[test]
    fn handles_deep_and_wide_trees_without_recursion() {
        let deep = 10_000;
        let chain = (0..deep - 1).map(|i| [i, i + 1]).collect();
        let result = run(&request(deep, chain)).unwrap();
        assert_eq!(
            result.positions[deep - 1],
            vec![0.0, -2.0 * (deep - 1) as f64]
        );

        // Large enough that rescanning all parents for each child would be prohibitive.
        let wide = 20_000;
        let edges = (1..wide).map(|i| [0, i]).collect();
        let result = run(&request(wide, edges)).unwrap();
        assert_eq!(result.positions[0][0], 0.0);
        assert!(result.positions[1][0] < result.positions[wide - 1][0]);
    }

    #[test]
    fn rejects_multiple_parents_and_disconnected_cycles() {
        assert!(run(&request(3, vec![[0, 2], [1, 2]]))
            .unwrap_err()
            .contains("multiple parents"));
        assert!(run(&request(4, vec![[0, 1], [2, 3], [3, 2]]))
            .unwrap_err()
            .contains("not reachable"));
    }
}
