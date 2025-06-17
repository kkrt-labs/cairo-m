use std::collections::HashMap;

use stwo_prover::core::fields::qm31::QM31;

use super::io::MemEntry;

#[derive(Debug, Default, Clone)]
pub struct MemoryBoundaries {
    pub initial_memory: Vec<MemEntry>,
    pub final_memory: Vec<MemEntry>,
}

#[derive(Debug, Default)]
pub struct MemoryCache {
    clock_cache: HashMap<u32, u32>,
    initial_memory: HashMap<u32, [u32; 4]>,
    current_memory: HashMap<u32, [u32; 4]>,
}

impl MemoryCache {
    pub fn new() -> Self {
        Self {
            clock_cache: HashMap::new(),
            initial_memory: HashMap::new(),
            current_memory: HashMap::new(),
        }
    }

    pub fn push(&mut self, mem_entry: MemEntry, clock: u32) -> (u32, QM31, QM31, u32, u32) {
        let prev_clock = self
            .clock_cache
            .get(&mem_entry.addr)
            .copied()
            .unwrap_or_else(|| {
                self.initial_memory.insert(mem_entry.addr, mem_entry.val);
                0
            });
        self.clock_cache.insert(mem_entry.addr, clock);
        let old_value = self
            .current_memory
            .insert(mem_entry.addr, mem_entry.val)
            .unwrap_or([0, 0, 0, 0]);

        (
            mem_entry.addr,
            QM31::from_u32_unchecked(old_value[0], old_value[1], old_value[2], old_value[3]),
            QM31::from_u32_unchecked(
                mem_entry.val[0],
                mem_entry.val[1],
                mem_entry.val[2],
                mem_entry.val[3],
            ),
            prev_clock,
            clock,
        )
    }

    pub fn get_memory_boundaries(&self) -> MemoryBoundaries {
        MemoryBoundaries {
            initial_memory: self
                .initial_memory
                .iter()
                .map(|(&addr, &val)| MemEntry { addr, val })
                .collect(),
            final_memory: self
                .current_memory
                .iter()
                .map(|(&addr, &val)| MemEntry { addr, val })
                .collect(),
        }
    }
}
