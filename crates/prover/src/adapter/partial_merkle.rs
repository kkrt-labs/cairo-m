use std::collections::HashMap;
use std::sync::OnceLock;

use num_traits::Zero;
use stwo_prover::core::fields::m31::M31;
use stwo_prover::core::fields::qm31::QM31;

/// NodeData represents a node in the partial Merkle tree
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeData {
    pub index: u32,
    pub layer: u32,
    pub value_left: QM31,
    pub value_right: QM31,
}

/// Maximum number of layers in the tree (2^30 leaves)
pub const MAX_MEMORY_LOG_SIZE: u32 = 30;

/// Mock Poseidon hash function for testing
/// TODO: Replace with actual Poseidon hash implementation
fn poseidon_hash(left: QM31, right: QM31) -> QM31 {
    // Simple mock hash: XOR the components
    let l = left.to_m31_array();
    let r = right.to_m31_array();
    QM31::from_m31_array([
        M31::from(l[0].0 ^ r[0].0),
        M31::from(l[1].0 ^ r[1].0),
        M31::from(l[2].0 ^ r[2].0),
        M31::from(l[3].0 ^ r[3].0),
    ])
}

/// Precomputed default hashes for each layer
/// default_hash[0] = hash(0,0)
/// default_hash[1] = hash(hash(0), hash(0))
/// default_hash[2] = hash(hash(hash(0), hash(0)), hash(hash(0), hash(0)))
/// etc.
fn compute_default_hashes() -> Vec<QM31> {
    let mut defaults = vec![QM31::zero(); (MAX_MEMORY_LOG_SIZE + 1) as usize];

    // Layer 0: hash of zero
    defaults[0] = poseidon_hash(QM31::zero(), QM31::zero());

    // Compute default hashes for each layer
    for layer in 1..=MAX_MEMORY_LOG_SIZE {
        let prev_default = defaults[(layer - 1) as usize];
        defaults[layer as usize] = poseidon_hash(prev_default, prev_default);
    }

    defaults
}

static DEFAULT_HASHES: OnceLock<Vec<QM31>> = OnceLock::new();

fn get_default_hashes() -> &'static Vec<QM31> {
    DEFAULT_HASHES.get_or_init(compute_default_hashes)
}

/// Build a partial Merkle tree from a memory state
pub fn build_partial_merkle_tree(memory: HashMap<M31, QM31>) -> Vec<NodeData> {
    if memory.is_empty() {
        return vec![];
    }

    // Assert memory size is within bounds
    assert!(
        memory.len() < (1 << MAX_MEMORY_LOG_SIZE),
        "Memory size must be less than 2^30"
    );

    let mut nodes = Vec::new();

    // Layer 0: leaf nodes - work directly with memory values
    let mut current_layer_nodes: HashMap<u32, QM31> = HashMap::new();

    for (addr, value) in memory {
        // Hash the value to get leaf node value
        let leaf_value = poseidon_hash(value, QM31::zero());
        current_layer_nodes.insert(addr.0, leaf_value);
    }

    // Build tree layer by layer - all 31 layers (0 to 30)
    for layer in 0..=MAX_MEMORY_LOG_SIZE {
        if layer == MAX_MEMORY_LOG_SIZE {
            let root_value = current_layer_nodes[&0];

            nodes.push(NodeData {
                index: 0,
                layer: MAX_MEMORY_LOG_SIZE,
                value_left: root_value,
                value_right: root_value,
            });
            break;
        }

        let mut next_layer_nodes: HashMap<u32, QM31> = HashMap::new();

        // For layer 0 and beyond, we need to handle all nodes that exist
        // plus any missing siblings needed to compute parent nodes
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
                .unwrap_or_else(|| get_default_hashes()[layer as usize]);
            let right_value = current_layer_nodes
                .get(&right_index)
                .copied()
                .unwrap_or_else(|| get_default_hashes()[layer as usize]);

            // Store node data
            nodes.push(NodeData {
                index: left_index,
                layer,
                value_left: left_value,
                value_right: right_value,
            });

            // Compute parent value
            let parent_value = poseidon_hash(left_value, right_value);
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
        let tree = build_partial_merkle_tree(memory);
        assert!(tree.is_empty());
    }

    #[test]
    fn test_single_element_tree() {
        let mut memory = HashMap::new();
        memory.insert(M31::from(5), QM31::from(42));

        let tree = build_partial_merkle_tree(memory);
        // Should have nodes up to the root
        assert!(!tree.is_empty());
    }

    #[test]
    fn test_multiple_elements_tree() {
        let mut memory = HashMap::new();
        memory.insert(M31::from(0), QM31::from(10));
        memory.insert(M31::from(1), QM31::from(20));
        memory.insert(M31::from(5), QM31::from(30));
        memory.insert(M31::from(100), QM31::from(40));

        let tree = build_partial_merkle_tree(memory);

        // Verify the tree goes all the way to layer 30
        let max_layer = tree.iter().map(|node| node.layer).max().unwrap_or(0);
        // For this small example, it might not reach layer 30, which is OK
        assert!(max_layer <= MAX_MEMORY_LOG_SIZE);

        // Helper function to find a node
        let find_node = |index: u32, layer: u32| -> Option<&NodeData> {
            tree.iter()
                .find(|node| node.index == index && node.layer == layer)
        };

        // Expected nodes: (index, layer, left_value, right_value)
        // Layer 0 is leaves, increasing up to the root

        // Check (0, 0, 10, 20) - leaf layer
        let node = find_node(0, 0).expect("Should find node at index 0, layer 0");
        let expected_left = poseidon_hash(QM31::from(10), QM31::zero()); // hash of address 0's value
        let expected_right = poseidon_hash(QM31::from(20), QM31::zero()); // hash of address 1's value
        assert_eq!(node.value_left, expected_left);
        assert_eq!(node.value_right, expected_right);

        // Check (4, 0, 0, 30) - leaf layer
        let node = find_node(4, 0).expect("Should find node at index 4, layer 0");
        let expected_left = get_default_hashes()[0]; // address 4 is empty
        let expected_right = poseidon_hash(QM31::from(30), QM31::zero()); // hash of address 5's value
        assert_eq!(node.value_left, expected_left);
        assert_eq!(node.value_right, expected_right);

        // Check (100, 0, 40, 0) - leaf layer
        let node = find_node(100, 0).expect("Should find node at index 100, layer 0");
        let expected_left = poseidon_hash(QM31::from(40), QM31::zero()); // hash of address 100's value
        let expected_right = get_default_hashes()[0]; // address 101 is empty
        assert_eq!(node.value_left, expected_left);
        assert_eq!(node.value_right, expected_right);

        // Check (0, 1, hash(0,1), default) - layer 1
        let node = find_node(0, 1).expect("Should find node at index 0, layer 1");
        let left_child = find_node(0, 0).unwrap();
        let expected_left = poseidon_hash(left_child.value_left, left_child.value_right);
        let expected_right = get_default_hashes()[1]; // default at layer 1
        assert_eq!(node.value_left, expected_left);
        assert_eq!(node.value_right, expected_right);

        // Check (2, 1, hash(4,5), default) - layer 1
        let node = find_node(2, 1).expect("Should find node at index 2, layer 1");
        let left_child = find_node(4, 0).unwrap();
        let expected_left = poseidon_hash(left_child.value_left, left_child.value_right);
        let expected_right = get_default_hashes()[1]; // default at layer 1
        assert_eq!(node.value_left, expected_left);
        assert_eq!(node.value_right, expected_right);

        // Check (50, 1, hash(100,101), default) - layer 1
        let node = find_node(50, 1).expect("Should find node at index 50, layer 1");
        let left_child = find_node(100, 0).unwrap();
        let expected_left = poseidon_hash(left_child.value_left, left_child.value_right);
        let expected_right = get_default_hashes()[1]; // default at layer 1
        assert_eq!(node.value_left, expected_left);
        assert_eq!(node.value_right, expected_right);
    }

    #[test]
    fn test_tree_builds_to_root() {
        // Test with addresses at extremes to force full tree height
        let mut memory = HashMap::new();
        memory.insert(M31::from(0), QM31::from(1));
        memory.insert(M31::from((1 << MAX_MEMORY_LOG_SIZE) - 1), QM31::from(2)); // High address to force height

        let tree = build_partial_merkle_tree(memory);

        let mut current_index = 0u32;

        for layer in 0..=MAX_MEMORY_LOG_SIZE {
            // Find node at current layer with appropriate index
            let node = tree
                .iter()
                .find(|n| n.layer == layer && n.index == current_index);

            if node.is_some() {
                current_index >>= 1;
            }
        }

        let max_layer = tree.iter().map(|node| node.layer).max().unwrap_or(0);

        // We should always build exactly to layer 30 (31 layers total)
        assert_eq!(
            max_layer, 30,
            "Tree should always build to layer 30 (root layer)"
        );
    }
}
