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
    memory_cache: HashMap<u32, u32>,
    initial_memory: HashMap<u32, [u32; 4]>,
    final_memory: HashMap<u32, [u32; 4]>,
}

impl MemoryCache {
    pub fn new() -> Self {
        Self {
            memory_cache: HashMap::new(),
            initial_memory: HashMap::new(),
            final_memory: HashMap::new(),
        }
    }

    pub fn push(&mut self, mem_entry: MemEntry, clock: u32) -> (u32, QM31, u32, u32) {
        let prev_clock = self.memory_cache.get(&mem_entry.addr).copied().unwrap_or_else(|| {
            self.initial_memory.insert(mem_entry.addr, mem_entry.val);
            0
        });
        self.memory_cache.insert(mem_entry.addr, clock);
        self.final_memory.insert(mem_entry.addr, mem_entry.val);
        
        (
            mem_entry.addr,
            QM31::from_u32_unchecked(mem_entry.val[0], mem_entry.val[1], mem_entry.val[2], mem_entry.val[3]),
            prev_clock,
            clock,
        )
    }

    pub fn get_memory_boundaries(&self) -> MemoryBoundaries {
        MemoryBoundaries {
            initial_memory: self.initial_memory
                .iter()
                .map(|(&addr, &val)| MemEntry { addr, val })
                .collect(),
            final_memory: self.final_memory
                .iter()
                .map(|(&addr, &val)| MemEntry { addr, val })
                .collect(),
        }
    }
} 
