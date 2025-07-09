use std::collections::HashMap;

use num_traits::Zero;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;

/// NodeData represents a node in the partial Merkle tree
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeData {
    pub index: u32,
    pub depth: u32,
    pub value_left: M31,
    pub value_right: M31,
}

/// Trait for hash functions used in the Merkle tree
pub trait MerkleHasher: Clone {
    /// Hash two M31 values into a single M31
    fn hash(left: M31, right: M31) -> M31;

    /// Get precomputed default hashes for each depth
    fn default_hashes() -> &'static [M31];
}

pub const MAX_MEMORY_LOG_SIZE: u32 = 28;
pub const QM31_LOG_SIZE: u32 = 2; // a QM31 is 4 M31 so 4 leaves
pub const TREE_HEIGHT: u32 = MAX_MEMORY_LOG_SIZE + QM31_LOG_SIZE; // tree height is 30, with depth 0 (root) to depth 30 (leaves)

/// Mock hash implementation for testing
#[derive(Clone)]
pub struct MockHasher;

impl MerkleHasher for MockHasher {
    fn hash(left: M31, right: M31) -> M31 {
        M31::from(left.0 ^ right.0)
    }

    fn default_hashes() -> &'static [M31] {
        use std::sync::OnceLock;
        static DEFAULT_HASHES: OnceLock<Vec<M31>> = OnceLock::new();

        DEFAULT_HASHES.get_or_init(|| {
            let mut defaults = vec![M31::zero(); (TREE_HEIGHT + 1) as usize];

            // Depth 30 (leaves): zero values
            defaults[TREE_HEIGHT as usize] = M31::zero();

            // Compute default hashes for each depth from leaves to root
            for depth in (0..TREE_HEIGHT).rev() {
                let child_default = defaults[(depth + 1) as usize];
                defaults[depth as usize] = Self::hash(child_default, child_default);
            }

            defaults
        })
    }
}

/// Build a partial Merkle tree from a memory state
/// Each QM31 value is split into 4 M31 leaves
/// The tree has depth 0 to 30:
/// - Depth 0: Root with a single hash value
/// - Depth 30: Leaves with up to 2^30 M31 values (from 2^28 QM31 memory cells)
pub fn build_partial_merkle_tree<H: MerkleHasher>(
    memory: &HashMap<M31, (QM31, M31, M31)>,
) -> (Vec<NodeData>, Option<M31>) {
    if memory.is_empty() {
        return (vec![], None);
    }

    // Assert memory size is within bounds
    assert!(
        memory.len() < (1 << MAX_MEMORY_LOG_SIZE),
        "Memory size must be less than 2^{}",
        MAX_MEMORY_LOG_SIZE
    );

    let mut nodes = Vec::new();

    // Depth 30 (leaves): convert each QM31 to 4 M31 leaves
    let mut current_depth_nodes: HashMap<u32, M31> = HashMap::new();

    for (addr, (value, _, _)) in memory {
        let m31_values = value.to_m31_array();
        let base_address = addr.0 << QM31_LOG_SIZE;

        for (i, &m31_value) in m31_values.iter().enumerate() {
            current_depth_nodes.insert(base_address + i as u32, m31_value);
        }
    }

    // Build tree from leaves (depth 30) up to root excluded (depth 1)
    for depth in (1..=TREE_HEIGHT).rev() {
        let mut parent_depth_nodes: HashMap<u32, M31> = HashMap::new();

        // Process all nodes at this depth
        let mut indices_to_process: Vec<u32> = current_depth_nodes.keys().copied().collect();
        indices_to_process.sort_unstable();

        let mut processed_indices = std::collections::HashSet::new();

        for &index in &indices_to_process {
            if processed_indices.contains(&index) {
                continue;
            }

            // Get sibling index
            let sibling_index = index ^ 1;
            let parent_index = index >> 1;

            // Ensure we process both siblings together
            let (left_index, right_index) = if index % 2 == 0 {
                (index, sibling_index)
            } else {
                (sibling_index, index)
            };

            let left_value = current_depth_nodes
                .get(&left_index)
                .copied()
                .unwrap_or_else(|| H::default_hashes()[depth as usize]);
            let right_value = current_depth_nodes
                .get(&right_index)
                .copied()
                .unwrap_or_else(|| H::default_hashes()[depth as usize]);

            // Store node data
            nodes.push(NodeData {
                index: left_index,
                depth,
                value_left: left_value,
                value_right: right_value,
            });

            // Compute parent value
            let parent_value = H::hash(left_value, right_value);
            parent_depth_nodes.insert(parent_index, parent_value);

            // Mark both indices as processed
            processed_indices.insert(left_index);
            processed_indices.insert(right_index);
        }

        current_depth_nodes = parent_depth_nodes;
    }

    assert_eq!(current_depth_nodes.len(), 1);
    let root_value = current_depth_nodes[&0];

    (nodes, Some(root_value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_tree() {
        let memory = HashMap::new();
        let (tree, root) = build_partial_merkle_tree::<MockHasher>(&memory);
        assert!(tree.is_empty());
        assert!(root.is_none());
    }

    #[test]
    fn test_single_element_tree() {
        let mut memory = HashMap::new();
        memory.insert(M31::from(5), (QM31::from(42), M31::zero(), M31::zero()));

        let (tree, root) = build_partial_merkle_tree::<MockHasher>(&memory);
        // Should have nodes up to the root
        assert!(!tree.is_empty());
        assert!(root.is_some());
    }

    #[test]
    fn test_multiple_elements_tree() {
        let mut memory = HashMap::new();
        // Create QM31 values with specific M31 components for testing
        memory.insert(
            M31::from(0),
            (
                QM31::from_m31_array([M31::from(10), M31::from(11), M31::from(12), M31::from(13)]),
                M31::zero(),
                M31::zero(),
            ),
        );
        memory.insert(
            M31::from(1),
            (
                QM31::from_m31_array([M31::from(20), M31::from(21), M31::from(22), M31::from(23)]),
                M31::zero(),
                M31::zero(),
            ),
        );

        let (tree, root) = build_partial_merkle_tree::<MockHasher>(&memory);

        // Verify the tree exists
        assert!(!tree.is_empty());
        assert!(root.is_some());

        // Helper function to find a node
        fn find_node(tree: &[NodeData], index: u32, depth: u32) -> Option<&NodeData> {
            tree.iter()
                .find(|node| node.index == index && node.depth == depth)
        }

        // Check depth 30 nodes (leaves) - each QM31 value creates 4 consecutive leaves
        // Address 0 -> leaves 0,1,2,3
        // Address 1 -> leaves 4,5,6,7

        // First pair of M31 values from address 0
        let node = find_node(&tree, 0, 30).expect("Should find node at index 0, depth 30");
        assert_eq!(node.value_left, M31::from(10));
        assert_eq!(node.value_right, M31::from(11));

        // Second pair of M31 values from address 0
        let node = find_node(&tree, 2, 30).expect("Should find node at index 2, depth 30");
        assert_eq!(node.value_left, M31::from(12));
        assert_eq!(node.value_right, M31::from(13));

        // First pair of M31 values from address 1
        let node = find_node(&tree, 4, 30).expect("Should find node at index 4, depth 30");
        assert_eq!(node.value_left, M31::from(20));
        assert_eq!(node.value_right, M31::from(21));
    }

    #[test]
    fn test_tree_builds_to_root() {
        // Test with addresses at extremes to force full tree height
        let mut memory = HashMap::new();
        memory.insert(M31::from(0), (QM31::from(1), M31::zero(), M31::zero()));
        // Use a high address within bounds (2^28 - 1)
        memory.insert(
            M31::from((1 << MAX_MEMORY_LOG_SIZE) - 1),
            (QM31::from(2), M31::zero(), M31::zero()),
        );

        let (tree, root) = build_partial_merkle_tree::<MockHasher>(&memory);
        assert!(root.is_some());

        let min_depth = tree.iter().map(|node| node.depth).min().unwrap_or(1);

        // We should build down to depth 1 (not 0, since root is excluded from nodes)
        assert_eq!(
            min_depth, 1,
            "Tree should build down to depth 1 (parent of root)"
        );

        // Also check we have leaves at depth 30
        let max_depth = tree.iter().map(|node| node.depth).max().unwrap_or(0);
        assert_eq!(
            max_depth, TREE_HEIGHT,
            "Tree should have leaves at depth {}",
            TREE_HEIGHT
        );
    }
}
