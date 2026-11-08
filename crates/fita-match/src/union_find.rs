/// Disjoint sets over the numbers `0..len`, with path compression and union
/// by size.
#[derive(Debug, Clone)]
pub struct UnionFind {
    parent: Vec<usize>,
    size: Vec<usize>,
}

impl UnionFind {
    /// Creates `len` sets with one element each.
    pub fn new(len: usize) -> Self {
        Self {
            parent: (0..len).collect(),
            size: vec![1; len],
        }
    }

    /// Returns the representative of the set that holds `element`.
    ///
    /// Panics if `element` is out of range.
    pub fn find(&mut self, element: usize) -> usize {
        let mut root = element;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        let mut current = element;
        while self.parent[current] != root {
            let next = self.parent[current];
            self.parent[current] = root;
            current = next;
        }
        root
    }

    /// Joins the sets of `a` and `b`. Returns false if they were already
    /// in the same set.
    pub fn union(&mut self, a: usize, b: usize) -> bool {
        let root_a = self.find(a);
        let root_b = self.find(b);
        if root_a == root_b {
            return false;
        }
        let (big, small) = if self.size[root_a] >= self.size[root_b] {
            (root_a, root_b)
        } else {
            (root_b, root_a)
        };
        self.parent[small] = big;
        self.size[big] += self.size[small];
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starts_with_singletons() {
        let mut sets = UnionFind::new(3);
        assert_eq!((sets.find(0), sets.find(1), sets.find(2)), (0, 1, 2));
    }

    #[test]
    fn joins_and_reports_redundant_unions() {
        let mut sets = UnionFind::new(5);
        assert!(sets.union(0, 1));
        assert!(sets.union(3, 4));
        assert!(sets.union(1, 4));
        assert!(!sets.union(0, 3));
        assert_eq!(sets.find(0), sets.find(4));
        assert_ne!(sets.find(0), sets.find(2));
    }

    #[test]
    fn long_chains_compress() {
        let mut sets = UnionFind::new(1000);
        for i in 1..1000 {
            sets.union(i - 1, i);
        }
        let root = sets.find(999);
        assert!((0..1000).all(|i| sets.find(i) == root));
    }
}
