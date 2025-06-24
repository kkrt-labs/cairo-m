use std::io::{BufReader, Read};
use std::path::Path;

use bytemuck::{bytes_of_mut, Pod, Zeroable};
use cairo_m_common::instruction::InstructionError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VmImportError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] sonic_rs::Error),
    #[error("Instruction error: {0}")]
    Instruction(#[from] InstructionError),
    #[error("No memory segments")]
    NoMemorySegments,
    #[error("Empty trace: trace file contains no entries")]
    EmptyTrace,
    #[error("Failed to read metadata header")]
    MetadataError,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Pod, Zeroable, Debug, PartialEq, Eq)]
pub struct IoTraceEntry {
    pub fp: u32,
    pub pc: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Pod, Zeroable, Debug)]
pub struct IoMemoryEntry {
    pub address: u32,
    pub value: [u32; 4],
}

/// Metadata header for memory traces containing program length
#[repr(C)]
#[derive(Copy, Clone, Default, Pod, Zeroable, Debug)]
pub struct MemoryTraceMetadata {
    pub program_length: u32,
}

pub struct TraceIter<R: Read>(pub R);

// Type alias for the concrete iterator from a file path
pub type TraceFileIter = TraceIter<BufReader<std::fs::File>>;

impl<R: Read> Iterator for TraceIter<R> {
    type Item = IoTraceEntry;

    fn next(&mut self) -> Option<Self::Item> {
        let mut entry = IoTraceEntry::default();
        self.0
            .read_exact(bytes_of_mut(&mut entry))
            .ok()
            .map(|_| entry)
    }
}

impl TryFrom<&Path> for TraceFileIter {
    type Error = VmImportError;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let file = std::fs::File::open(path)?;
        let reader = BufReader::new(file);
        Ok(Self(reader))
    }
}

pub struct MemoryEntryIter<R: Read> {
    reader: R,
    metadata: MemoryTraceMetadata,
}

// Type alias for the concrete iterator from a file path
pub type MemoryEntryFileIter = MemoryEntryIter<BufReader<std::fs::File>>;

impl<R: Read> MemoryEntryIter<R> {
    /// Create a new MemoryEntryIter and read the metadata header
    pub fn new(mut reader: R) -> Result<Self, VmImportError> {
        let mut metadata = MemoryTraceMetadata::default();
        reader
            .read_exact(bytes_of_mut(&mut metadata))
            .map_err(|_| VmImportError::MetadataError)?;

        Ok(Self { reader, metadata })
    }

    /// Get the program length from the metadata header
    pub const fn program_length(&self) -> usize {
        self.metadata.program_length as usize
    }
}

impl<R: Read> Iterator for MemoryEntryIter<R> {
    type Item = IoMemoryEntry;

    fn next(&mut self) -> Option<Self::Item> {
        let mut entry = IoMemoryEntry::default();
        self.reader
            .read_exact(bytes_of_mut(&mut entry))
            .ok()
            .map(|_| entry)
    }
}

impl TryFrom<&Path> for MemoryEntryFileIter {
    type Error = VmImportError;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let file = std::fs::File::open(path)?;
        let reader = BufReader::new(file);
        Self::new(reader)
    }
}
