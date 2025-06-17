use std::io::Read;
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
pub struct TraceEntry {
    pub pc: u32,
    pub fp: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Pod, Zeroable, Debug)]
pub struct MemEntry {
    pub addr: u32,
    pub val: [u32; 4],
}

pub struct TraceIter<'a, R: Read>(pub &'a mut R);

impl<R: Read> Iterator for TraceIter<'_, R> {
    type Item = TraceEntry;

    fn next(&mut self) -> Option<Self::Item> {
        let mut entry = TraceEntry::default();
        self.0
            .read_exact(bytes_of_mut(&mut entry))
            .ok()
            .map(|_| entry)
    }
}

pub struct MemEntryIter<'a, R: Read>(pub &'a mut R);

impl<R: Read> Iterator for MemEntryIter<'_, R> {
    type Item = MemEntry;

    fn next(&mut self) -> Option<Self::Item> {
        let mut entry = MemEntry::default();
        self.0
            .read_exact(bytes_of_mut(&mut entry))
            .ok()
            .map(|_| entry)
    }
}

pub fn read_memory_and_trace_from_paths(
    trace_path: &Path,
    mem_path: &Path,
) -> Result<(Vec<MemEntry>, Vec<TraceEntry>), VmImportError> {
    let mut trace_file = std::io::BufReader::new(std::fs::File::open(trace_path)?);
    let mut mem_file = std::io::BufReader::new(std::fs::File::open(mem_path)?);

    let memory_entries: Vec<MemEntry> = MemEntryIter(&mut mem_file).collect();
    let trace_entries: Vec<TraceEntry> = TraceIter(&mut trace_file).collect();

    Ok((memory_entries, trace_entries))
}
