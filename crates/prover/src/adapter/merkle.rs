use std::collections::HashMap;

use num_traits::Zero;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;

pub use super::HashInput;

pub const MAX_MEMORY_LOG_SIZE: u32 = 28;
pub const QM31_LOG_SIZE: u32 = 2; // a QM31 is 4 M31 so 4 leaves
pub const TREE_HEIGHT: u32 = MAX_MEMORY_LOG_SIZE + QM31_LOG_SIZE; // tree height is 30, with depth 0 (root) to depth 30 (leaves)

/// NodeData represents a node in the partial Merkle tree with left node taken as reference.
///
/// - index: the index of the node (left node index)
/// - depth: the depth of this left node
/// - value_left: the value of this same left node
/// - value_right: the value of the node to the right
/// - value_parent: the value of the parent node
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeData {
    pub index: M31,
    pub depth: u8,
    pub value_left: M31,
    pub value_right: M31,
    pub value_parent: M31,
}

impl NodeData {
    pub fn to_m31_array(&self) -> [M31; 5] {
        [
            self.index,
            M31::from(self.depth as u32),
            self.value_left,
            self.value_right,
            self.value_parent,
        ]
    }

    pub fn to_hash_input(&self) -> HashInput {
        let mut input: HashInput = Default::default();
        input[0] = self.value_left;
        input[1] = self.value_right;
        input
    }
}

pub trait MerkleHasher {
    fn hash(left: M31, right: M31) -> M31;
    fn default_hashes() -> &'static [M31];
}

/// Build a partial Merkle tree from a memory state
/// Each QM31 value is split into 4 M31 leaves
/// The tree has depth 0 to 30:
/// - Depth 0: Root with a single hash value
/// - Depth 30: Leaves with up to 2^30 M31 values (from 2^28 QM31 memory cells)
///
/// Returns (node data, root hash) for all hash computations
pub fn build_partial_merkle_tree<H: MerkleHasher>(
    memory: &mut HashMap<(M31, M31), (QM31, M31, M31)>,
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

    for ((addr, _), (value, _, _)) in memory.iter() {
        // no intermediate nodes yet so depth is leaf depth
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

            let mut add_intermediate_node = |node_index: u32| {
                let default_hash = H::default_hashes()[depth as usize];
                memory.insert(
                    (M31::from(node_index), M31::from(depth)),
                    (
                        QM31::from(default_hash),
                        M31::zero(), // clock is irrelevant
                        M31::zero(), // intermediate nodes shouldn't be emitted for the memory relation
                    ),
                );
                default_hash
            };

            let left_value = current_depth_nodes
                .get(&left_index)
                .copied()
                .unwrap_or_else(|| add_intermediate_node(left_index));
            let right_value = current_depth_nodes
                .get(&right_index)
                .copied()
                .unwrap_or_else(|| add_intermediate_node(right_index));

            // Calculate parent hash and collect trace data if using Poseidon2Hash
            let parent_value = H::hash(left_value, right_value);

            // Store node data
            nodes.push(NodeData {
                index: M31::from(left_index),
                depth: depth as u8,
                value_left: left_value,
                value_right: right_value,
                value_parent: parent_value,
            });

            // Store parent value for next depth
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
    use crate::utils::poseidon2::Poseidon2Hash;

    #[test]
    fn test_empty_tree() {
        let mut memory = HashMap::new();
        let (tree, root) = build_partial_merkle_tree::<Poseidon2Hash>(&mut memory);
        assert!(tree.is_empty());
        assert!(root.is_none());
    }

    #[test]
    fn test_single_element_tree() {
        let mut memory = HashMap::new();
        memory.insert(
            (M31::from(5), M31::from(TREE_HEIGHT)),
            (QM31::from(42), M31::zero(), M31::zero()),
        );

        let (tree, root) = build_partial_merkle_tree::<Poseidon2Hash>(&mut memory);
        // Should have nodes up to the root
        assert!(!tree.is_empty());
        assert!(root.is_some());
    }

    #[test]
    fn test_multiple_elements_tree() {
        let mut memory = HashMap::new();
        // Create QM31 values with specific M31 components for testing
        memory.insert(
            (M31::from(0), M31::from(TREE_HEIGHT)),
            (
                QM31::from_m31_array([M31::from(10), M31::from(11), M31::from(12), M31::from(13)]),
                M31::zero(),
                M31::zero(),
            ),
        );
        memory.insert(
            (M31::from(1), M31::from(TREE_HEIGHT)),
            (
                QM31::from_m31_array([M31::from(20), M31::from(21), M31::from(22), M31::from(23)]),
                M31::zero(),
                M31::zero(),
            ),
        );

        let (tree, root) = build_partial_merkle_tree::<Poseidon2Hash>(&mut memory);

        // Verify the tree exists
        assert!(!tree.is_empty());
        assert!(root.is_some());

        // Helper function to find a node
        fn find_node(tree: &[NodeData], index: u32, depth: u32) -> Option<&NodeData> {
            tree.iter()
                .find(|node| node.index == M31::from(index) && node.depth == depth as u8)
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
        memory.insert(
            (M31::from(0), M31::from(TREE_HEIGHT)),
            (QM31::from(1), M31::zero(), M31::zero()),
        );
        // Use a high address within bounds (2^28 - 1)
        memory.insert(
            (
                M31::from((1 << MAX_MEMORY_LOG_SIZE) - 1),
                M31::from(TREE_HEIGHT),
            ),
            (QM31::from(2), M31::zero(), M31::zero()),
        );

        let (tree, root) = build_partial_merkle_tree::<Poseidon2Hash>(&mut memory);
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
            max_depth, TREE_HEIGHT as u8,
            "Tree should have leaves at depth {}",
            TREE_HEIGHT
        );
    }
}
