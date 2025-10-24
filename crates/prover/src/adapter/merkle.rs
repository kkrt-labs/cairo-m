//! # Merkle Tree Construction for Memory Commitments
//!
//! This module implements partial Merkle tree construction for Cairo-M memory states.
//! The initial memory committed is vm.memory before the first step of a segment of execution.
//! The final memory committed is vm.memory after the last step of a segment of execution.
//! The memory commitments can contain memory values that are not accessed during the execution (ie not part of the
//! memory log).
//! These commitments are used for continuation to attest that the memory is consistent over the overall execution.
//!
//! ## Tree construction
//! - The leaves of the tree are M31 values corresponding to the QM31 values of the memory.
//!   Each QM31 memory value (4 M31 elements) is decomposed into 4 consecutive leaves:
//!     - Address N → Leaves at positions [N*4, N*4+1, N*4+2, N*4+3]
//!     - Maximum memory size: 2^28 QM31 values → 2^30 M31 leaves
//! - Only used memory cells are added as leaves. To keep a 31 layered tree, missing nodes are added using default hash values,
//!   these added nodes are the "intermediate nodes".

use std::collections::HashMap;

use cairo_m_common::PublicAddressRanges;
use num_traits::{One, Zero};
use stwo::core::fields::m31::M31;
use stwo::core::fields::qm31::QM31;

pub use super::HashInput;

/// MerkleValue represents a value in the Merkle tree with its multiplicity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MerkleValue {
    pub value: M31,
    pub multiplicity: M31,
}

impl MerkleValue {
    pub fn new_node(value: M31) -> Self {
        Self {
            value,
            multiplicity: M31::one(),
        }
    }

    pub fn new_public_node(value: M31) -> Self {
        Self {
            value,
            multiplicity: M31::from(2),
        }
    }

    pub fn new_intermediate(value: M31) -> Self {
        Self {
            value,
            multiplicity: M31::zero(),
        }
    }
}

/// Maximum memory size in QM31 values (2^28)
pub const MAX_MEMORY_LOG_SIZE: u32 = 28;
/// Number of M31 elements per QM31 (4 elements = 2^2)
pub const QM31_LOG_SIZE: u32 = 2;
/// Total Merkle tree height: memory size + QM31 decomposition (28 + 2 = 30)
pub const TREE_HEIGHT: u32 = MAX_MEMORY_LOG_SIZE + QM31_LOG_SIZE;

/// Indicates whether we're building an initial or final Merkle tree
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeType {
    Initial,
    Final,
}

/// Represents a node in the Merkle tree.
///
/// Each node captures a single hash operation: parent = hash(left, right).
/// The node is identified by the left child's index and depth.
///
/// # Node Scheme
/// ```text
///         parent_value
///           /      \
///   left_value   right_value
///   (index)      (index+1)
///    [depth]      [depth]
/// ```
///
/// ## Fields
/// - `index`: Index of the left child node
/// - `depth`: Tree depth of this node (0 = root, 30 = M31 leaves)
/// - `left_value`: Hash value of the left child
/// - `right_value`: Hash value of the right child
/// - `parent_value`: Computed hash value of this node
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeData {
    /// Index of the left child node at this tree depth
    pub index: M31,
    /// Tree depth (0 = root, increases toward leaves)
    pub depth: u8,
    /// Value of the left child
    pub left_value: MerkleValue,
    /// Value of the right child
    pub right_value: MerkleValue,
    /// Computed parent hash: hash(left_value, right_value)
    pub parent_value: MerkleValue,
}

impl NodeData {
    /// Converts the node data to an M31 array for constraint evaluation.
    ///
    /// ## Returns
    /// Array containing [index, depth, left_value, right_value, parent_value]
    pub fn to_m31_array(&self) -> [M31; 8] {
        [
            self.index,
            M31::from(self.depth as u32),
            self.left_value.value,
            self.right_value.value,
            self.parent_value.value,
            self.left_value.multiplicity,
            self.right_value.multiplicity,
            self.parent_value.multiplicity,
        ]
    }

    /// Extracts the hash input (left and right values) for hash computation.
    ///
    /// ## Returns
    /// HashInput array with left and right child values
    pub fn to_hash_input(&self) -> HashInput {
        let mut input: HashInput = Default::default();
        input[0] = self.left_value.value;
        input[1] = self.right_value.value;
        input
    }
}

/// Trait for Merkle tree hash functions.
///
/// Implementations must provide both the hash function and default hash values
/// for uninitialized nodes at each tree level.
pub trait MerkleHasher {
    /// Computes the hash of two M31 field elements.
    ///
    /// ## Arguments
    /// * `left` - Left child hash value
    /// * `right` - Right child hash value
    ///
    /// ## Returns
    /// The computed parent hash value
    fn hash(left: M31, right: M31) -> M31;

    /// Returns default hash values for each tree level.
    ///
    /// Used for uninitialized nodes to maintain tree structure.
    /// Array index corresponds to tree depth.
    ///
    /// ## Returns
    /// Static array of default hash values indexed by depth
    fn default_hashes() -> &'static [M31];
}

/// Constructs a partial Merkle tree from current memory state.
///
/// This function builds only the necessary portions of the Merkle tree based on
/// the memory addresses that were actually present in memory. Missing
/// nodes are filled with default hash values to maintain tree integrity.
///
/// ## Process
/// 1. **Leaf Generation**: Each QM31 memory value is split into 4 M31 leaves
/// 2. **Tree Construction**: Build from leaves (depth 30) up to depth 1
/// 3. **Missing Nodes**: Fill gaps with default hashes and add to memory map
/// 4. **Root Computation**: Calculate single root hash at depth 0
///
/// ## Arguments
/// * `memory` - Memory state map: (address, depth) → (value, clock, multiplicity). Will be modified to include intermediate nodes
///
/// ## Returns
/// * `Vec<NodeData>` - Vec of nodes of the merkle tree
/// * `Option<M31>` - Root hash value (None if memory is empty)
///
/// ## Tree Structure
/// - **Depth 0**: Root (excluded from NodeData)
/// - **Depth 1-29**: Intermediate hash computations
/// - **Depth 30**: M31 leaf values from QM31 decomposition
pub fn build_partial_merkle_tree<H: MerkleHasher>(
    memory: &HashMap<M31, (QM31, M31, M31)>,
    tree_type: TreeType,
    public_address_ranges: &PublicAddressRanges,
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
    let mut current_depth_nodes: HashMap<u32, MerkleValue> = HashMap::new();

    for (addr, (value, _, _)) in memory.iter() {
        // no intermediate nodes yet so depth is leaf depth
        let m31_values = value.to_m31_array();
        let base_address = addr.0 << QM31_LOG_SIZE;

        // Check if this address should have increased multiplicity
        let is_public_address = match tree_type {
            TreeType::Initial => {
                public_address_ranges.program.contains(&addr.0)
                    || public_address_ranges.input.contains(&addr.0)
            }
            TreeType::Final => public_address_ranges.output.contains(&addr.0),
        };

        for (i, &m31_value) in m31_values.iter().enumerate() {
            let merkle_value = if is_public_address {
                MerkleValue::new_public_node(m31_value)
            } else {
                MerkleValue::new_node(m31_value)
            };

            current_depth_nodes.insert(base_address + i as u32, merkle_value);
        }
    }

    // Build tree from leaves (depth 30) up to root excluded (depth 1)
    for depth in (1..=TREE_HEIGHT).rev() {
        let mut parent_depth_nodes: HashMap<u32, MerkleValue> = HashMap::new();

        // Process all nodes at this depth
        let mut indices_to_process: Vec<u32> = current_depth_nodes.keys().copied().collect();
        indices_to_process.sort_unstable();

        let mut processed_indices = std::collections::HashSet::new();

        for &index in &indices_to_process {
            if processed_indices.contains(&index) {
                continue;
            }

            // Get sibling and parent indexes
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
                .unwrap_or_else(|| {
                    MerkleValue::new_intermediate(H::default_hashes()[depth as usize])
                });
            let right_value = current_depth_nodes
                .get(&right_index)
                .copied()
                .unwrap_or_else(|| {
                    MerkleValue::new_intermediate(H::default_hashes()[depth as usize])
                });

            // Calculate parent hash
            let parent_hash = H::hash(left_value.value, right_value.value);
            let parent_value = MerkleValue::new_node(parent_hash);

            // Store node data
            nodes.push(NodeData {
                index: M31::from(left_index),
                depth: depth as u8,
                left_value,
                right_value,
                parent_value,
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
    let root_value = current_depth_nodes[&0].value;

    (nodes, Some(root_value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::poseidon2::Poseidon2Hash;

    #[test]
    fn test_empty_tree() {
        let memory = HashMap::new();
        let public_address_ranges = PublicAddressRanges::default();
        let (tree, root) = build_partial_merkle_tree::<Poseidon2Hash>(
            &memory,
            TreeType::Initial,
            &public_address_ranges,
        );
        assert!(tree.is_empty());
        assert!(root.is_none());
    }

    #[test]
    fn test_single_element_tree() {
        let mut memory = HashMap::new();
        memory.insert(M31::from(5), (QM31::from(42), M31::zero(), M31::zero()));

        let public_address_ranges = PublicAddressRanges::default();
        let (tree, root) = build_partial_merkle_tree::<Poseidon2Hash>(
            &memory,
            TreeType::Initial,
            &public_address_ranges,
        );
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

        let public_address_ranges = PublicAddressRanges::default();
        let (tree, root) = build_partial_merkle_tree::<Poseidon2Hash>(
            &memory,
            TreeType::Initial,
            &public_address_ranges,
        );

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
        assert_eq!(node.left_value.value, M31::from(10));
        assert_eq!(node.right_value.value, M31::from(11));

        // Second pair of M31 values from address 0
        let node = find_node(&tree, 2, 30).expect("Should find node at index 2, depth 30");
        assert_eq!(node.left_value.value, M31::from(12));
        assert_eq!(node.right_value.value, M31::from(13));

        // First pair of M31 values from address 1
        let node = find_node(&tree, 4, 30).expect("Should find node at index 4, depth 30");
        assert_eq!(node.left_value.value, M31::from(20));
        assert_eq!(node.right_value.value, M31::from(21));
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

        let public_address_ranges = PublicAddressRanges::default();
        let (tree, root) = build_partial_merkle_tree::<Poseidon2Hash>(
            &memory,
            TreeType::Initial,
            &public_address_ranges,
        );
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
