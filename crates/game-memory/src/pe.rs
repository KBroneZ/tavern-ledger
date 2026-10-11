//! Finds an exported function in a loaded module, from the module's own
//! headers in memory (the public PE format). Used only for the Mono
//! runtime's exports.

use crate::memory::{Memory, ReadError};

/// Most exports the lookup walks; Mono has about 1 500.
const MAX_EXPORTS: u32 = 20_000;

/// The address of `name` in the module loaded at `base`, or `None` if the
/// module does not export it.
pub fn export(mem: &dyn Memory, base: u64, name: &str) -> Result<Option<u64>, ReadError> {
    if mem.read_u16(base)? != 0x5a4d {
        return Err(ReadError::Unreadable(base)); // "MZ"
    }
    let nt = base + u64::from(mem.read_u32(base + 0x3c)?);
    if mem.read_u32(nt)? != 0x4550 {
        return Err(ReadError::Unreadable(nt)); // "PE\0\0"
    }
    let optional = nt + 24;
    if mem.read_u16(optional)? != 0x20b {
        return Err(ReadError::Unreadable(optional)); // PE32+ only
    }
    // Data directory 0 (exports) sits at offset 112 of the PE32+ optional header.
    let dir_rva = u64::from(mem.read_u32(optional + 112)?);
    if dir_rva == 0 {
        return Ok(None);
    }
    let dir = base + dir_rva;
    let functions_count = u64::from(mem.read_u32(dir + 20)?);
    let count = mem.read_u32(dir + 24)?.min(MAX_EXPORTS);
    let functions = base + u64::from(mem.read_u32(dir + 28)?);
    let names = base + u64::from(mem.read_u32(dir + 32)?);
    let ordinals = base + u64::from(mem.read_u32(dir + 36)?);
    for i in 0..u64::from(count) {
        let name_rva = u64::from(mem.read_u32(names + i * 4)?);
        if mem.read_name(base + name_rva)? != name {
            continue;
        }
        let ordinal = u64::from(mem.read_u16(ordinals + i * 2)?);
        if ordinal >= functions_count {
            return Err(ReadError::Unreadable(ordinals + i * 2));
        }
        let rva = u64::from(mem.read_u32(functions + ordinal * 4)?);
        return Ok(Some(base + rva));
    }
    Ok(None)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::memory::FakeMemory;

    /// Builds a minimal PE32+ image at `base` exporting `exports` (name, rva).
    pub(crate) fn fake_module(mem: &mut FakeMemory, base: u64, exports: &[(&str, u32)]) {
        mem.zeroed(base, 0x400);
        mem.put_u16(base, 0x5a4d);
        mem.put_u32(base + 0x3c, 0x80);
        mem.put_u32(base + 0x80, 0x4550);
        mem.put_u16(base + 0x80 + 24, 0x20b);
        let dir = 0x1000u32;
        mem.put_u32(base + 0x80 + 24 + 112, dir);
        let n = exports.len() as u32;
        let (functions, names, ordinals, strings) = (0x1100u32, 0x1200u32, 0x1300u32, 0x1400u32);
        mem.zeroed(base + u64::from(dir), 0x40);
        mem.put_u32(base + u64::from(dir) + 20, n);
        mem.put_u32(base + u64::from(dir) + 24, n);
        mem.put_u32(base + u64::from(dir) + 28, functions);
        mem.put_u32(base + u64::from(dir) + 32, names);
        mem.put_u32(base + u64::from(dir) + 36, ordinals);
        mem.zeroed(base + u64::from(functions), 4 * exports.len());
        mem.zeroed(base + u64::from(names), 4 * exports.len());
        mem.zeroed(base + u64::from(ordinals), 2 * exports.len());
        let mut at = strings;
        for (i, (name, rva)) in exports.iter().enumerate() {
            let i = i as u64;
            mem.put_u32(base + u64::from(functions) + i * 4, *rva);
            mem.put_u32(base + u64::from(names) + i * 4, at);
            mem.put_u16(base + u64::from(ordinals) + i * 2, i as u16);
            mem.put_str(base + u64::from(at), name);
            at += name.len() as u32 + 1;
        }
    }

    #[test]
    fn finds_an_export_by_name() {
        let mut mem = FakeMemory::new();
        fake_module(
            &mut mem,
            0x10_0000,
            &[("mono_a", 0x2000), ("mono_get_root_domain", 0x3000)],
        );
        assert_eq!(
            export(&mem, 0x10_0000, "mono_get_root_domain"),
            Ok(Some(0x10_3000))
        );
        assert_eq!(export(&mem, 0x10_0000, "missing"), Ok(None));
    }

    #[test]
    fn an_ordinal_past_the_function_table_is_an_error() {
        let mut mem = FakeMemory::new();
        fake_module(&mut mem, 0x10_0000, &[("mono_get_root_domain", 0x3000)]);
        mem.zeroed(0x10_1100, 0x40); // readable, but past the one function
        mem.put_u16(0x10_1300, 7);
        assert!(export(&mem, 0x10_0000, "mono_get_root_domain").is_err());
    }

    #[test]
    fn refuses_something_that_is_not_a_module() {
        let mut mem = FakeMemory::new();
        mem.zeroed(0x10_0000, 0x400);
        assert!(export(&mem, 0x10_0000, "x").is_err());
    }
}
