// Module declarations
pub mod adapter;
pub mod instructions;
pub mod io;
pub mod memory;

// Re-export public API
pub use adapter::{adapt_from_iter, import_from_vm_output, ProverInput};
pub use instructions::{Instructions, StatesByOpcodes, StateData, VmState};
pub use io::{MemEntry, MemEntryIter, TraceEntry, TraceIter, VmImportError, read_memory_and_trace_from_paths};
pub use memory::{MemoryBoundaries, MemoryCache};
