use std::collections::HashMap;

use num_traits::Zero;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;

/// NodeData represents a node in the partial Merkle tree
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeData {
    pub index: u32,
    pub layer: u32,
    pub value_left: M31,
    pub value_right: M31,
}

/// Trait for hash functions used in the Merkle tree
pub trait MerkleHasher: Clone {
    /// Hash two M31 values into a single M31
    fn hash(left: M31, right: M31) -> M31;

    /// Get precomputed default hashes for each layer
    fn default_hashes() -> &'static [M31];
}

pub const MAX_MEMORY_LOG_SIZE: u32 = 28;
pub const QM31_LOG_SIZE: u32 = 2; // a QM31 is 4 M31 so 4 leaves
pub const TREE_HEIGHT: u32 = MAX_MEMORY_LOG_SIZE + QM31_LOG_SIZE; // layers go from 0 (leaves) to TREE_HEIGHT (root) included

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

            // Layer 0: hash of zero
            defaults[0] = Self::hash(M31::zero(), M31::zero());

            // Compute default hashes for each layer
            for layer in 1..=TREE_HEIGHT {
                let prev_default = defaults[(layer - 1) as usize];
                defaults[layer as usize] = Self::hash(prev_default, prev_default);
            }

            defaults
        })
    }
}

/// Build a partial Merkle tree from a memory state
/// Each QM31 value is split into 4 M31 leaves
/// The tree has 31 layers (0 to 30):
/// - Layer 0: Leaf layer with up to 2^30 M31 values (from 2^28 QM31 memory cells)
/// - Layer 30: Root layer with a single hash value
pub fn build_partial_merkle_tree<H: MerkleHasher>(
    memory: &HashMap<M31, (QM31, M31, M31)>,
) -> Vec<NodeData> {
    if memory.is_empty() {
        return vec![];
    }

    // Assert memory size is within bounds
    assert!(
        memory.len() < (1 << MAX_MEMORY_LOG_SIZE),
        "Memory size must be less than 2^{}",
        MAX_MEMORY_LOG_SIZE
    );

    let mut nodes = Vec::new();

    // Layer 0: leaf nodes - convert each QM31 to 4 M31 leaves
    let mut current_layer_nodes: HashMap<u32, M31> = HashMap::new();

    for (addr, (value, _, _)) in memory {
        let m31_values = value.to_m31_array();
        let base_address = addr.0 << QM31_LOG_SIZE;

        for (i, &m31_value) in m31_values.iter().enumerate() {
            let leaf_value = H::hash(m31_value, M31::zero());
            current_layer_nodes.insert(base_address + i as u32, leaf_value);
        }
    }

    // Build tree layer by layer - 31 layers total (0 to 30)
    for layer in 0..=TREE_HEIGHT {
        if layer == TREE_HEIGHT {
            let root_value = current_layer_nodes[&0];
            nodes.push(NodeData {
                index: 0,
                layer: TREE_HEIGHT,
                value_left: root_value,
                value_right: root_value,
            });
            break;
        }

        let mut next_layer_nodes: HashMap<u32, M31> = HashMap::new();

        // Process all nodes at this layer
        let mut indices_to_process: Vec<u32> = current_layer_nodes.keys().copied().collect();
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

            let left_value = current_layer_nodes
                .get(&left_index)
                .copied()
                .unwrap_or_else(|| H::default_hashes()[layer as usize]);
            let right_value = current_layer_nodes
                .get(&right_index)
                .copied()
                .unwrap_or_else(|| H::default_hashes()[layer as usize]);

            // Store node data
            nodes.push(NodeData {
                index: left_index,
                layer,
                value_left: left_value,
                value_right: right_value,
            });

            // Compute parent value
            let parent_value = H::hash(left_value, right_value);
            next_layer_nodes.insert(parent_index, parent_value);

            // Mark both indices as processed
            processed_indices.insert(left_index);
            processed_indices.insert(right_index);
        }

        current_layer_nodes = next_layer_nodes;
    }
    nodes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_tree() {
        let memory = HashMap::new();
        let tree = build_partial_merkle_tree::<MockHasher>(&memory);
        assert!(tree.is_empty());
    }

    #[test]
    fn test_single_element_tree() {
        let mut memory = HashMap::new();
        memory.insert(M31::from(5), (QM31::from(42), M31::zero(), M31::zero()));

        let tree = build_partial_merkle_tree::<MockHasher>(&memory);
        // Should have nodes up to the root
        assert!(!tree.is_empty());
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

        let tree = build_partial_merkle_tree::<MockHasher>(&memory);

        // Verify the tree exists
        assert!(!tree.is_empty());

        // Helper function to find a node
        let find_node = |index: u32, layer: u32| -> Option<&NodeData> {
            tree.iter()
                .find(|node| node.index == index && node.layer == layer)
        };

        // Check layer 0 nodes - each QM31 value creates 4 consecutive leaves
        // Address 0 -> leaves 0,1,2,3
        // Address 1 -> leaves 4,5,6,7

        // First pair of M31 values from address 0
        let node = find_node(0, 0).expect("Should find node at index 0, layer 0");
        let expected_left_val = MockHasher::hash(M31::from(10), M31::zero());
        let expected_right_val = MockHasher::hash(M31::from(11), M31::zero());
        assert_eq!(node.value_left, expected_left_val);
        assert_eq!(node.value_right, expected_right_val);

        // Second pair of M31 values from address 0
        let node = find_node(2, 0).expect("Should find node at index 2, layer 0");
        let expected_left_val = MockHasher::hash(M31::from(12), M31::zero());
        let expected_right_val = MockHasher::hash(M31::from(13), M31::zero());
        assert_eq!(node.value_left, expected_left_val);
        assert_eq!(node.value_right, expected_right_val);

        // First pair of M31 values from address 1
        let node = find_node(4, 0).expect("Should find node at index 4, layer 0");
        let expected_left_val = MockHasher::hash(M31::from(20), M31::zero());
        let expected_right_val = MockHasher::hash(M31::from(21), M31::zero());
        assert_eq!(node.value_left, expected_left_val);
        assert_eq!(node.value_right, expected_right_val);
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

        let tree = build_partial_merkle_tree::<MockHasher>(&memory);

        let max_layer = tree.iter().map(|node| node.layer).max().unwrap_or(0);

        // We should build to layer 30 (31 layers total: 0 to 30)
        assert_eq!(
            max_layer, TREE_HEIGHT,
            "Tree should build to layer {} (root layer)",
            TREE_HEIGHT
        );
    }
}
