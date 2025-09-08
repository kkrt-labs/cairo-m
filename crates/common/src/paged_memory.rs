use std::collections::HashMap;
use std::ops::{Index, IndexMut};
use stwo_prover::core::fields::qm31::QM31;

// TODO sync this with VM crate
pub const DEFAULT_MEMORY_SIZE: usize = 1 << 28;
/// The default page size is 64K elements.
pub const DEFAULT_PAGE_SIZE: usize = 1 << 16;

/// Sparse, paged memory optimized for large logical address spaces.
///
/// - The logical memory is split into fixed-size pages. Each page is a contiguous
///   native heap allocation (`Box<[QM31]>`).
/// - Pages are allocated lazily on first write. Reads never allocate and return
///   `None` for missing pages/cells; callers can treat that as zero.
/// - `len` tracks the logical length (highest written index + 1, or the latest
///   resize). It does not imply physical allocation for all covered pages.
/// - This design keeps zero-filled regions sparse while providing fast, cache-friendly
///   access within allocated pages.
#[derive(Debug, Clone)]
pub struct PagedMemory {
    page_size: usize,
    num_pages: usize,
    len: usize,
    pages: Vec<Option<Page>>, // index = page_number; lazy-allocated pages
}

#[derive(Debug, Clone)]
struct Page {
    data: Box<[QM31]>,
    /// Bitmap tracking which cells in the page have been initialized.
    init_bits: Box<[u64]>,
}

impl PagedMemory {
    /// Create a new paged memory with a total capacity of `mem_size` elements and
    /// runtime-configurable `page_size`. Capacity must be a multiple of `page_size`.
    pub fn new(mem_size: usize, page_size: usize) -> Self {
        assert!(
            mem_size % page_size == 0,
            "mem_size must be multiple of page_size, mem_size={}, page_size={}",
            mem_size,
            page_size
        );
        let num_pages = mem_size / page_size;
        Self {
            page_size,
            num_pages,
            len: 0,
            pages: vec![None; num_pages],
        }
    }

    /// Translate a linear address into `(page_number, offset_within_page)`.
    const fn page_index(&self, addr: usize) -> (usize, usize) {
        let page_num = addr / self.page_size;
        let offset = addr % self.page_size;
        (page_num, offset)
    }

    /// Get a mutable view of a page, allocating it if it doesn't exist yet.
    fn get_page_mut(&mut self, page_num: usize) -> &mut Page {
        self.pages[page_num].get_or_insert_with(|| Page {
            data: vec![QM31::from(0); self.page_size].into_boxed_slice(),
            init_bits: vec![0u64; self.page_size.div_ceil(64)].into_boxed_slice(),
        })
    }

    /// Get an immutable view of a page if it exists. Does not allocate.
    fn get_page(&self, page_num: usize) -> Option<&Page> {
        self.pages.get(page_num).and_then(|opt| opt.as_ref())
    }

    /// Write a single cell. Allocates the corresponding page on demand and updates `len`.
    pub fn set(&mut self, addr: usize, value: QM31) {
        let (page_num, offset) = self.page_index(addr);
        assert!(page_num < self.num_pages, "address out of range");
        let page = self.get_page_mut(page_num);
        page.data[offset] = value;
        set_bit(&mut page.init_bits, offset);
        if addr >= self.len {
            self.len = addr + 1;
        }
    }

    /// Read a single cell. Returns `None` if the page/cell has never been written.
    pub fn get(&self, addr: usize) -> Option<&QM31> {
        let (page_num, offset) = self.page_index(addr);
        if page_num >= self.num_pages {
            return None;
        }
        self.get_page(page_num).and_then(|page| {
            if get_bit(&page.init_bits, offset) {
                Some(&page.data[offset])
            } else {
                None
            }
        })
    }

    /// Read a single cell mutably. Returns `None` if the address is out of range.
    /// This allocates the page on demand.
    pub fn get_mut(&mut self, addr: usize) -> Option<&mut QM31> {
        let (page_num, offset) = self.page_index(addr);
        if page_num >= self.num_pages {
            return None;
        }
        let page = self.get_page_mut(page_num);
        set_bit(&mut page.init_bits, offset);
        Some(&mut page.data[offset])
    }

    /// Logical length (highest initialized index + 1, or latest resize).
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Whether there are no initialized elements (i.e., `len == 0`).
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Extend by pushing values one-by-one, allocating pages only as needed.
    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = QM31>,
    {
        for value in iter {
            let idx = self.len;
            assert!(idx < self.num_pages * self.page_size, "extend out of range");
            let (page_num, offset) = self.page_index(idx);
            let page = self.get_page_mut(page_num);
            page.data[offset] = value;
            set_bit(&mut page.init_bits, offset);
            self.len += 1;
        }
    }

    /// Returns a sparse map of all initialized cells
    /// Keys are absolute addresses; values are the stored QM31.
    pub fn to_initialized_map(&self) -> HashMap<u32, QM31> {
        let mut map = HashMap::new();
        for (page_idx, page_opt) in self.pages.iter().enumerate() {
            if let Some(page) = page_opt {
                let base_addr = (page_idx * self.page_size) as u32;
                for word_index in 0..page.init_bits.len() {
                    let mut bits = page.init_bits[word_index];
                    while bits != 0 {
                        let tz = bits.trailing_zeros() as usize;
                        let off = word_index * 64 + tz;
                        if off >= self.page_size {
                            break;
                        }
                        let addr = base_addr + off as u32;
                        map.insert(addr, page.data[off]);
                        bits &= bits - 1;
                    }
                }
            }
        }
        map
    }
}

impl Default for PagedMemory {
    /// Default configuration used by the VM: total capacity of 2^MAX_MEMORY_SIZE_BITS
    /// and a page size of 64Ki elements.
    fn default() -> Self {
        // 4kB page size, total capacity 2^MAX_MEMORY_SIZE_BITS
        let mem_size: usize = DEFAULT_MEMORY_SIZE;
        let page_size: usize = DEFAULT_PAGE_SIZE;
        Self::new(mem_size, page_size)
    }
}

impl FromIterator<QM31> for PagedMemory {
    /// Build memory from a sequence by appending each value; pages are allocated lazily.
    fn from_iter<I: IntoIterator<Item = QM31>>(iter: I) -> Self {
        let mut pm = Self::default();
        pm.extend(iter);
        pm
    }
}

impl Index<usize> for PagedMemory {
    type Output = QM31;

    /// Random-access read with panic-on-missing semantics, similar to `Vec`.
    /// Prefer `get()` for non-panicking reads that can treat missing cells as zero.
    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.len, "index out of bounds");
        let (page_num, offset) = self.page_index(index);
        let page = self.get_page(page_num).expect("page not allocated");
        if get_bit(&page.init_bits, offset) {
            &page.data[offset]
        } else {
            panic!("uninitialized memory cell");
        }
    }
}

impl IndexMut<usize> for PagedMemory {
    /// Random-access write with on-demand page allocation, like `Vec` growth semantics
    /// for `len` but without clearing intermediate pages.
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        assert!(
            index < self.num_pages * self.page_size,
            "index out of range"
        );
        if index >= self.len {
            self.len = index + 1;
        }
        let (page_num, offset) = self.page_index(index);
        let page = self.get_page_mut(page_num);
        set_bit(&mut page.init_bits, offset);
        &mut page.data[offset]
    }
}

fn set_bit(bits: &mut [u64], idx: usize) {
    let word = idx >> 6;
    let bit = idx & 63;
    bits[word] |= 1u64 << bit;
}

fn get_bit(bits: &[u64], idx: usize) -> bool {
    let word = idx >> 6;
    let bit = idx & 63;
    ((bits[word] >> bit) & 1) == 1
}

impl PartialEq for PagedMemory {
    fn eq(&self, other: &Self) -> bool {
        if self.len != other.len {
            return false;
        }
        for i in 0..self.len {
            // Compare logical values, treating missing cells as zero.
            let a = self.get(i).copied().unwrap_or_default();
            let b = other.get(i).copied().unwrap_or_default();
            if a != b {
                return false;
            }
        }
        true
    }
}

impl Eq for PagedMemory {}

impl PartialEq<Vec<QM31>> for PagedMemory {
    fn eq(&self, other: &Vec<QM31>) -> bool {
        if self.len != other.len() {
            return false;
        }
        for (i, b) in other.iter().enumerate() {
            let a = self.get(i).copied().unwrap_or_default();
            if a != *b {
                return false;
            }
        }
        true
    }
}

impl PartialEq<PagedMemory> for Vec<QM31> {
    fn eq(&self, other: &PagedMemory) -> bool {
        other == self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count_allocated_pages(pm: &PagedMemory) -> usize {
        pm.pages.iter().filter(|p| p.is_some()).count()
    }

    #[test]
    fn default_is_sparse_and_empty() {
        let pm = PagedMemory::new(32, 4);
        assert_eq!(pm.len(), 0);
        assert!(pm.is_empty());
        assert_eq!(count_allocated_pages(&pm), 0);
    }

    #[test]
    fn read_does_not_allocate_pages() {
        let pm = PagedMemory::new(32, 4);
        let before = count_allocated_pages(&pm);
        assert!(pm.get(7).is_none());
        let after = count_allocated_pages(&pm);
        assert_eq!(before, after);
    }

    #[test]
    fn write_allocates_one_page_and_updates_len() {
        let mut pm = PagedMemory::new(32, 4); // 8 pages of 4
        assert_eq!(count_allocated_pages(&pm), 0);
        // Address 7 is on page 1 (0..3 -> page 0, 4..7 -> page 1)
        pm.set(7, QM31::from(123));
        assert_eq!(pm.len(), 8);
        assert_eq!(pm.get(7).copied(), Some(QM31::from(123)));
        assert_eq!(count_allocated_pages(&pm), 1);
    }

    #[test]
    fn index_mut_allocates_and_updates_len() {
        let mut pm = PagedMemory::new(32, 4);
        pm[3] = QM31::from(11);
        assert_eq!(pm.len(), 4);
        assert_eq!(pm.get(3).copied(), Some(QM31::from(11)));
        assert_eq!(count_allocated_pages(&pm), 1);
    }

    #[test]
    fn extend_allocates_pages_as_needed() {
        let mut pm = PagedMemory::new(32, 4);
        // Push 3 values -> stays in page 0
        pm.extend([QM31::from(1), QM31::from(2), QM31::from(3)]);
        assert_eq!(pm.len(), 3);
        assert_eq!(count_allocated_pages(&pm), 1);

        // Push 2 more -> crosses into page 1
        pm.extend([QM31::from(4), QM31::from(5)]);
        assert_eq!(pm.len(), 5);
        assert_eq!(count_allocated_pages(&pm), 2);

        // Verify contents
        assert_eq!(pm.get(0).copied(), Some(QM31::from(1)));
        assert_eq!(pm.get(4).copied(), Some(QM31::from(5)));
    }

    #[test]
    fn get_out_of_range_returns_none() {
        let pm = PagedMemory::new(32, 4);
        // Address beyond capacity (32) -> page 8 which doesn't exist
        assert!(pm.get(1000).is_none());
    }
}
