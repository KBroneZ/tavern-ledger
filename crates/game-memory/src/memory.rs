//! Reading another process's memory, behind a trait so every walk can be
//! tested against a synthetic memory image.

use std::collections::BTreeMap;

/// Longest C string the walker reads (class, field and assembly names).
pub const MAX_NAME: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReadError {
    /// The address range is not readable (or a pointer that must be set is
    /// null).
    Unreadable(u64),
    /// A name was not NUL-terminated within [`MAX_NAME`] bytes, or not UTF-8.
    BadName(u64),
}

pub trait Memory {
    /// Fills `buf` from `addr`, all or nothing.
    fn read(&self, addr: u64, buf: &mut [u8]) -> Result<(), ReadError>;

    fn read_u8(&self, addr: u64) -> Result<u8, ReadError> {
        let mut b = [0u8; 1];
        self.read(addr, &mut b)?;
        Ok(b[0])
    }

    fn read_u16(&self, addr: u64) -> Result<u16, ReadError> {
        let mut b = [0u8; 2];
        self.read(addr, &mut b)?;
        Ok(u16::from_le_bytes(b))
    }

    fn read_u32(&self, addr: u64) -> Result<u32, ReadError> {
        let mut b = [0u8; 4];
        self.read(addr, &mut b)?;
        Ok(u32::from_le_bytes(b))
    }

    fn read_i32(&self, addr: u64) -> Result<i32, ReadError> {
        Ok(self.read_u32(addr)? as i32)
    }

    fn read_u64(&self, addr: u64) -> Result<u64, ReadError> {
        let mut b = [0u8; 8];
        self.read(addr, &mut b)?;
        Ok(u64::from_le_bytes(b))
    }

    /// A pointer; null is an error, since every pointer the walker follows
    /// must be set.
    fn read_ptr(&self, addr: u64) -> Result<u64, ReadError> {
        match self.read_u64(addr)? {
            0 => Err(ReadError::Unreadable(0)),
            p => Ok(p),
        }
    }

    /// A NUL-terminated UTF-8 name of at most [`MAX_NAME`] bytes. Only used
    /// for type metadata (module exports, assembly, class and field names),
    /// never for game data.
    fn read_name(&self, addr: u64) -> Result<String, ReadError> {
        let mut out = Vec::new();
        let mut chunk = [0u8; 32];
        while out.len() < MAX_NAME {
            let at = addr + out.len() as u64;
            // A chunk can cross into an unreadable page; fall back to a byte.
            let got = match self.read(at, &mut chunk) {
                Ok(()) => &chunk[..],
                Err(_) => {
                    self.read(at, &mut chunk[..1])?;
                    &chunk[..1]
                }
            };
            if let Some(end) = got.iter().position(|&b| b == 0) {
                out.extend_from_slice(&got[..end]);
                return String::from_utf8(out).map_err(|_| ReadError::BadName(addr));
            }
            out.extend_from_slice(got);
        }
        Err(ReadError::BadName(addr))
    }
}

/// A synthetic memory image for tests: disjoint regions at fixed addresses.
#[derive(Default, Debug)]
pub struct FakeMemory {
    regions: BTreeMap<u64, Vec<u8>>,
}

impl FakeMemory {
    pub fn new() -> FakeMemory {
        FakeMemory::default()
    }

    /// Adds (or replaces) a region of bytes at `addr`. Regions must not
    /// overlap.
    pub fn put(&mut self, addr: u64, bytes: &[u8]) {
        self.regions.insert(addr, bytes.to_vec());
    }

    /// A zeroed region of `len` bytes.
    pub fn zeroed(&mut self, addr: u64, len: usize) {
        self.put(addr, &vec![0; len]);
    }

    pub fn put_u8(&mut self, addr: u64, v: u8) {
        self.write_into(addr, &[v]);
    }

    pub fn put_u16(&mut self, addr: u64, v: u16) {
        self.write_into(addr, &v.to_le_bytes());
    }

    pub fn put_u32(&mut self, addr: u64, v: u32) {
        self.write_into(addr, &v.to_le_bytes());
    }

    pub fn put_u64(&mut self, addr: u64, v: u64) {
        self.write_into(addr, &v.to_le_bytes());
    }

    /// A NUL-terminated string in its own region.
    pub fn put_str(&mut self, addr: u64, s: &str) {
        let mut b = s.as_bytes().to_vec();
        b.push(0);
        self.put(addr, &b);
    }

    /// Writes into the region covering `addr`, or adds a region.
    pub fn write_into(&mut self, addr: u64, bytes: &[u8]) {
        if let Some((start, region)) = self.regions.range_mut(..=addr).next_back() {
            let off = (addr - start) as usize;
            if off + bytes.len() <= region.len() {
                region[off..off + bytes.len()].copy_from_slice(bytes);
                return;
            }
        }
        self.put(addr, bytes);
    }
}

impl Memory for FakeMemory {
    fn read(&self, addr: u64, buf: &mut [u8]) -> Result<(), ReadError> {
        let (start, region) = self
            .regions
            .range(..=addr)
            .next_back()
            .ok_or(ReadError::Unreadable(addr))?;
        let off = (addr - start) as usize;
        let end = off
            .checked_add(buf.len())
            .ok_or(ReadError::Unreadable(addr))?;
        if end > region.len() {
            return Err(ReadError::Unreadable(addr));
        }
        buf.copy_from_slice(&region[off..end]);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_values_inside_a_region() {
        let mut m = FakeMemory::new();
        m.zeroed(0x1000, 16);
        m.put_u32(0x1004, 7);
        m.put_u64(0x1008, 0xdead_beef);
        assert_eq!(m.read_u32(0x1004), Ok(7));
        assert_eq!(m.read_u64(0x1008), Ok(0xdead_beef));
    }

    #[test]
    fn refuses_reads_outside_or_across_regions() {
        let mut m = FakeMemory::new();
        m.put(0x1000, &[1, 2, 3, 4]);
        assert_eq!(m.read_u32(0x0ffc), Err(ReadError::Unreadable(0x0ffc)));
        assert_eq!(m.read_u64(0x1000), Err(ReadError::Unreadable(0x1000)));
        assert_eq!(m.read_u32(0x2000), Err(ReadError::Unreadable(0x2000)));
    }

    #[test]
    fn null_pointer_is_an_error() {
        let mut m = FakeMemory::new();
        m.zeroed(0x1000, 8);
        assert!(m.read_ptr(0x1000).is_err());
    }

    #[test]
    fn reads_names_up_to_the_nul_at_a_region_end() {
        let mut m = FakeMemory::new();
        m.put_str(0x1000, "Assembly-CSharp");
        assert_eq!(m.read_name(0x1000).unwrap(), "Assembly-CSharp");
    }

    #[test]
    fn refuses_names_without_a_nul_or_too_long() {
        let mut m = FakeMemory::new();
        m.put(0x1000, b"abc");
        assert!(m.read_name(0x1000).is_err());
        m.put(0x2000, &[b'a'; MAX_NAME + 8]);
        assert_eq!(m.read_name(0x2000), Err(ReadError::BadName(0x2000)));
    }
}
