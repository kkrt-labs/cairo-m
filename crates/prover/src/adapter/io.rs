use std::io::{BufReader, Read};
use std::path::Path;

use bytemuck::{bytes_of_mut, Pod, Zeroable};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VmImportError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] sonic_rs::Error),
    #[error("No memory segments")]
    NoMemorySegments,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Pod, Zeroable)]
pub struct IoTraceEntry {
    pub pc: u32,
    pub fp: u32,
}

#[derive(Copy, Clone, Default, Debug)]
pub struct TraceEntry {
    pub pc: u32,
    pub fp: u32,
}

impl From<IoTraceEntry> for TraceEntry {
    fn from(io_entry: IoTraceEntry) -> Self {
        Self {
            pc: io_entry.pc,
            fp: io_entry.fp,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default, Pod, Zeroable, Debug)]
pub struct IoMemoryEntry {
    pub address: u32,
    pub value: [u32; 4],
}

#[derive(Copy, Clone, Default, Debug)]
pub struct MemoryEntry {
    pub address: u32,
    pub value: [u32; 4],
}

impl From<IoMemoryEntry> for MemoryEntry {
    fn from(io_entry: IoMemoryEntry) -> Self {
        Self {
            address: io_entry.address,
            value: io_entry.value,
        }
    }
}

pub struct TraceIter<R: Read>(pub R);

// Type alias for the concrete iterator from a file path
pub type TraceFileIter = TraceIter<BufReader<std::fs::File>>;

impl<R: Read> Iterator for TraceIter<R> {
    type Item = TraceEntry;

    fn next(&mut self) -> Option<Self::Item> {
        let mut entry = IoTraceEntry::default();
        self.0
            .read_exact(bytes_of_mut(&mut entry))
            .ok()
            .map(|_| entry.into())
    }
}

// Standalone function to create a TraceIter from a file path
pub fn trace_iter_from_path(path: &Path) -> Result<TraceFileIter, VmImportError> {
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);
    Ok(TraceIter(reader))
}

pub struct MemoryEntryIter<R: Read>(pub R);

// Type alias for the concrete iterator from a file path
pub type MemoryEntryFileIter = MemoryEntryIter<BufReader<std::fs::File>>;

impl<R: Read> Iterator for MemoryEntryIter<R> {
    type Item = MemoryEntry;

    fn next(&mut self) -> Option<Self::Item> {
        let mut entry = IoMemoryEntry::default();
        self.0
            .read_exact(bytes_of_mut(&mut entry))
            .ok()
            .map(|_| entry.into())
    }
}

// Standalone function to create a MemoryEntryIter from a file path
pub fn memory_entry_iter_from_path(path: &Path) -> Result<MemoryEntryFileIter, VmImportError> {
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);
    Ok(MemoryEntryIter(reader))
}

pub fn read_memory_and_trace_from_paths(
    trace_path: &Path,
    mem_path: &Path,
) -> Result<(Vec<MemoryEntry>, Vec<TraceEntry>), VmImportError> {
    let memory_entries: Vec<MemoryEntry> = memory_entry_iter_from_path(mem_path)?.collect();
    let trace_entries: Vec<TraceEntry> = trace_iter_from_path(trace_path)?.collect();

    Ok((memory_entries, trace_entries))
}
