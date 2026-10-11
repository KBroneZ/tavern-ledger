//! Walks Mono's runtime structures (domain, assembly, image, class, field,
//! vtable) to one named field. Mono is open source (MIT); the structure
//! offsets below were measured on the user's own client through names Mono
//! keeps at run time (see `docs/research/memory-reader.md`), not taken from
//! any other tracker. Every step checks what it finds (a name, a back
//! pointer, a bound), so a different layout fails instead of reading the
//! wrong thing.

use crate::memory::{Memory, ReadError};
use crate::pe;

/// Offsets inside Mono's structures for one Mono runtime build.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    pub domain_assemblies: u64,
    pub assembly_name: u64,
    pub assembly_image: u64,
    pub image_assembly_name: u64,
    pub image_class_cache: u64,
    pub class_parent: u64,
    pub class_image: u64,
    pub class_name: u64,
    pub class_namespace: u64,
    pub class_vtable_size: u64,
    pub class_fields: u64,
    pub class_runtime_info: u64,
    pub class_field_count: u64,
    pub class_next_cached: u64,
    pub field_size: u64,
    pub field_name: u64,
    pub field_parent: u64,
    pub field_offset: u64,
    pub type_attrs: u64,
    pub runtime_info_vtables: u64,
    pub vtable_class: u64,
    pub vtable_slots: u64,
}

/// `mono-2.0-bdwgc.dll` shipped with the game (Unity's Mono, 64-bit),
/// measured on build 253216.
pub const LAYOUT_BDWGC_X64: Layout = Layout {
    domain_assemblies: 0xa0,
    assembly_name: 0x10,
    assembly_image: 0x60,
    image_assembly_name: 0x30,
    image_class_cache: 0x4d0,
    class_parent: 0x30,
    class_image: 0x40,
    class_name: 0x48,
    class_namespace: 0x50,
    class_vtable_size: 0x5c,
    class_fields: 0x98,
    class_runtime_info: 0xd0,
    class_field_count: 0x100,
    class_next_cached: 0x108,
    field_size: 0x20,
    field_name: 0x08,
    field_parent: 0x10,
    field_offset: 0x18,
    type_attrs: 0x08,
    runtime_info_vtables: 0x08,
    vtable_class: 0x00,
    vtable_slots: 0x48,
};

/// Bounds on every walk, far above what the game has (119 assemblies, about
/// 14 000 classes in its main assembly, under 100 fields per class).
const MAX_ASSEMBLIES: usize = 1_000;
const MAX_BUCKETS: u32 = 1 << 20;
const MAX_CLASSES: usize = 200_000;
const MAX_FIELDS: u32 = 4_096;
const MAX_VTABLE: u32 = 65_536;

/// `FIELD_ATTRIBUTE_STATIC` and `FIELD_ATTRIBUTE_LITERAL` (ECMA-335).
const ATTR_STATIC: u32 = 0x10;
const ATTR_LITERAL: u32 = 0x40;
/// `MONO_TYPE_I4`: a field of type `int` (ECMA-335 element type).
pub const TYPE_I4: u32 = 0x08;

/// Why a walk stopped. Reasons name only Mono metadata (assembly, class and
/// field names), never game data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WalkError {
    Read(ReadError),
    /// Something the walk needs is not where it should be: the game version
    /// is not one the reader knows.
    Unsupported(String),
}

impl From<ReadError> for WalkError {
    fn from(e: ReadError) -> WalkError {
        WalkError::Read(e)
    }
}

impl std::fmt::Display for WalkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WalkError::Read(ReadError::Unreadable(_)) => write!(f, "memory not readable"),
            WalkError::Read(ReadError::BadName(_)) => write!(f, "unreadable name"),
            WalkError::Read(ReadError::BadPointer(_)) => write!(f, "not a pointer"),
            WalkError::Unsupported(why) => write!(f, "{why}"),
        }
    }
}

fn unsupported(why: impl Into<String>) -> WalkError {
    WalkError::Unsupported(why.into())
}

/// A field as Mono describes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Field {
    pub offset: u64,
    pub is_static: bool,
    /// The element type (`MONO_TYPE_*`), bits 16 to 23 after the attributes.
    pub type_code: u32,
}

pub struct Mono<'a> {
    mem: &'a dyn Memory,
    layout: Layout,
    domain: u64,
}

impl<'a> Mono<'a> {
    /// Finds the root domain from the runtime's own export
    /// `mono_get_root_domain`, a function that only returns a global:
    /// `mov rax, [rip + disp32]; ret`. Any other code means an unknown
    /// runtime, and the walk stops.
    pub fn attach(
        mem: &'a dyn Memory,
        mono_base: u64,
        layout: Layout,
    ) -> Result<Mono<'a>, WalkError> {
        let f = pe::export(mem, mono_base, "mono_get_root_domain")?
            .ok_or_else(|| unsupported("Mono runtime without mono_get_root_domain"))?;
        let mut code = [0u8; 8];
        mem.read(f, &mut code)?;
        if code[..3] != [0x48, 0x8b, 0x05] || code[7] != 0xc3 {
            return Err(unsupported("unknown Mono runtime code"));
        }
        let disp = i32::from_le_bytes([code[3], code[4], code[5], code[6]]);
        let global = (f + 7).wrapping_add_signed(i64::from(disp));
        let domain = mem.read_ptr(global)?;
        Ok(Mono {
            mem,
            layout,
            domain,
        })
    }

    pub fn mem(&self) -> &dyn Memory {
        self.mem
    }

    /// The image of a loaded assembly, by name.
    pub fn image(&self, assembly: &str) -> Result<u64, WalkError> {
        let l = &self.layout;
        let mut node = self.mem.read_ptr(self.domain + l.domain_assemblies)?;
        for _ in 0..MAX_ASSEMBLIES {
            let asm = self.mem.read_ptr(node)?;
            let name = self
                .mem
                .read_name(self.mem.read_ptr(asm + l.assembly_name)?)?;
            if name == assembly {
                let image = self.mem.read_ptr(asm + l.assembly_image)?;
                let back = self
                    .mem
                    .read_name(self.mem.read_ptr(image + l.image_assembly_name)?)?;
                if back != assembly {
                    return Err(unsupported("assembly image does not name its assembly"));
                }
                return Ok(image);
            }
            match self.mem.read_link(node + 8)? {
                None => break,
                Some(next) => node = next,
            }
        }
        Err(unsupported(format!("assembly {assembly} not loaded")))
    }

    /// A class of `image` by namespace and name, from the image's class
    /// cache (a hash table of classes chained through each class).
    pub fn class(&self, image: u64, namespace: &str, name: &str) -> Result<u64, WalkError> {
        let l = &self.layout;
        let cache = image + l.image_class_cache;
        let buckets = self.mem.read_u32(cache + 24)?;
        if buckets == 0 || buckets > MAX_BUCKETS {
            return Err(unsupported("class cache size out of range"));
        }
        let table = self.mem.read_ptr(cache + 32)?;
        let mut seen = 0usize;
        for i in 0..u64::from(buckets) {
            let mut link = self.mem.read_link(table + i * 8)?;
            while let Some(class) = link {
                seen += 1;
                if seen > MAX_CLASSES {
                    return Err(unsupported("class cache too long"));
                }
                if self.mem.read_u64(class + l.class_image)? != image {
                    return Err(unsupported("class does not point back to its image"));
                }
                if self
                    .mem
                    .read_name(self.mem.read_ptr(class + l.class_name)?)?
                    == name
                    && self.class_namespace(class)? == namespace
                {
                    return Ok(class);
                }
                link = self.mem.read_link(class + l.class_next_cached)?;
            }
        }
        Err(unsupported(format!("class {name} not found")))
    }

    fn class_namespace(&self, class: u64) -> Result<String, WalkError> {
        match self.mem.read_link(class + self.layout.class_namespace)? {
            None => Ok(String::new()),
            Some(p) => Ok(self.mem.read_name(p)?),
        }
    }

    /// The name of the class an object belongs to (object → vtable → class).
    pub fn object_class_name(&self, object: u64) -> Result<String, WalkError> {
        let class = self.object_class(object)?;
        Ok(self
            .mem
            .read_name(self.mem.read_ptr(class + self.layout.class_name)?)?)
    }

    /// The class of `object`, through its vtable.
    pub fn object_class(&self, object: u64) -> Result<u64, WalkError> {
        let vtable = self.mem.read_ptr(object)?;
        Ok(self.mem.read_ptr(vtable + self.layout.vtable_class)?)
    }

    /// The ints of a list object, after checking that it is a `list_class`
    /// (`List`1`) and that its items are an array (a class named `…[]`).
    pub fn typed_int_list(
        &self,
        list: u64,
        list_class: &str,
        max: usize,
    ) -> Result<Vec<i32>, WalkError> {
        if self.object_class_name(list)? != list_class {
            return Err(unsupported("list of another type"));
        }
        let items = self.mem.read_ptr(list + 0x10)?;
        if !self.object_class_name(items)?.ends_with("[]") {
            return Err(unsupported("list items are not an array"));
        }
        int_list(self.mem, list, max)
    }

    /// A field of `class` (not of its parents) by name. Literal (const)
    /// fields have no storage and are refused.
    pub fn field(&self, class: u64, name: &str) -> Result<Field, WalkError> {
        let l = &self.layout;
        let count = self.mem.read_u32(class + l.class_field_count)?;
        if count > MAX_FIELDS {
            return Err(unsupported("field count out of range"));
        }
        let fields = self.mem.read_ptr(class + l.class_fields)?;
        for i in 0..u64::from(count) {
            let entry = fields + i * l.field_size;
            if self
                .mem
                .read_name(self.mem.read_ptr(entry + l.field_name)?)?
                != name
            {
                continue;
            }
            if self.mem.read_u64(entry + l.field_parent)? != class {
                return Err(unsupported("field does not point back to its class"));
            }
            let ty = self.mem.read_ptr(entry)?;
            let word = self.mem.read_u32(ty + l.type_attrs)?;
            let attrs = word & 0xffff;
            if attrs & ATTR_LITERAL != 0 {
                return Err(unsupported(format!("field {name} is a constant")));
            }
            let offset = self.mem.read_i32(entry + l.field_offset)?;
            let offset = u64::try_from(offset).map_err(|_| unsupported("negative field offset"))?;
            return Ok(Field {
                offset,
                is_static: attrs & ATTR_STATIC != 0,
                type_code: (word >> 16) & 0xff,
            });
        }
        Err(unsupported(format!("field {name} not found")))
    }

    /// Where the static fields of `class` live in the root domain.
    pub fn static_data(&self, class: u64) -> Result<u64, WalkError> {
        let l = &self.layout;
        let info = self.mem.read_ptr(class + l.class_runtime_info)?;
        // Domain 0 is the root domain, the only one the game runs.
        let vtable = self.mem.read_ptr(info + l.runtime_info_vtables)?;
        if self.mem.read_u64(vtable + l.vtable_class)? != class {
            return Err(unsupported("vtable does not point back to its class"));
        }
        let slots = self.mem.read_u32(class + l.class_vtable_size)?;
        if slots > MAX_VTABLE {
            return Err(unsupported("vtable size out of range"));
        }
        // The static data pointer sits right after the method slots.
        Ok(self
            .mem
            .read_ptr(vtable + l.vtable_slots + u64::from(slots) * 8)?)
    }

    /// The object in a static reference field: `Class.field`.
    pub fn static_object(&self, class: u64, field: &str) -> Result<u64, WalkError> {
        let f = self.field(class, field)?;
        if !f.is_static {
            return Err(unsupported(format!("field {field} is not static")));
        }
        let data = self.static_data(class)?;
        reference(self.mem.read_ptr(data + f.offset), field)
    }

    /// The object in an instance reference field of `object` (of `class`).
    pub fn object_field(&self, object: u64, class: u64, field: &str) -> Result<u64, WalkError> {
        let f = self.instance_field(class, field)?;
        reference(self.mem.read_ptr(object + f.offset), field)
    }

    /// A 32-bit integer instance field of `object` (of `class`); a field of
    /// any other type is refused.
    pub fn int_field(&self, object: u64, class: u64, field: &str) -> Result<i32, WalkError> {
        let f = self.instance_field(class, field)?;
        if f.type_code != TYPE_I4 {
            return Err(unsupported(format!("field {field} is not an int")));
        }
        Ok(self.mem.read_i32(object + f.offset)?)
    }

    /// The elements of an array object of references, after checking that
    /// its class is `array_class` (for example `Object[]`), that it has at
    /// most `max` elements and that it holds at least `used`. Empty slots are
    /// `None`.
    pub fn ref_array(
        &self,
        array: u64,
        array_class: &str,
        used: usize,
        max: usize,
    ) -> Result<Vec<Option<u64>>, WalkError> {
        self.struct_array_refs(array, array_class, used, max, 8, 0)
    }

    /// The `stride`-byte entries of an array object of structs, read as the
    /// reference at `at` in each entry; same checks as [`Mono::ref_array`].
    pub fn struct_array_refs(
        &self,
        array: u64,
        array_class: &str,
        used: usize,
        max: usize,
        stride: u64,
        at: u64,
    ) -> Result<Vec<Option<u64>>, WalkError> {
        if self.object_class_name(array)? != array_class {
            return Err(unsupported("array of another type"));
        }
        let len = self.mem.read_u64(array + 0x18)?;
        let fits = at.checked_add(8).is_some_and(|end| end <= stride);
        if !fits || used > max || (used as u64) > len {
            return Err(unsupported("array shorter than its count"));
        }
        (0..used as u64)
            .map(|i| {
                let addr = i
                    .checked_mul(stride)
                    .and_then(|o| o.checked_add(0x20 + at))
                    .and_then(|o| array.checked_add(o))
                    .ok_or_else(|| unsupported("array out of range"))?;
                Ok(self.mem.read_link(addr)?)
            })
            .collect()
    }

    fn instance_field(&self, class: u64, field: &str) -> Result<Field, WalkError> {
        let f = self.field(class, field)?;
        if f.is_static {
            return Err(unsupported(format!("field {field} is static")));
        }
        Ok(f)
    }
}

/// A reference field's value: null is "not set" (the game has not made the
/// object yet); a value that is not a pointer, or memory that cannot be read,
/// is passed on as it is.
fn reference(read: Result<u64, ReadError>, field: &str) -> Result<u64, WalkError> {
    match read {
        Ok(p) => Ok(p),
        Err(ReadError::Unreadable(0)) => Err(unsupported(format!("{field} is not set"))),
        Err(e) => Err(WalkError::Read(e)),
    }
}

/// `System.Collections.Generic.List<int>` (or of an int enum): `_items` at
/// 0x10, `_size` at 0x18; the array keeps its length at 0x18 and its
/// elements from 0x20.
pub fn int_list(mem: &dyn Memory, list: u64, max: usize) -> Result<Vec<i32>, WalkError> {
    let items = mem.read_ptr(list + 0x10)?;
    let size = mem.read_i32(list + 0x18)?;
    let size = usize::try_from(size).map_err(|_| unsupported("negative list size"))?;
    if size > max {
        return Err(unsupported("list longer than expected"));
    }
    let capacity = mem.read_u64(items + 0x18)?;
    if (size as u64) > capacity {
        return Err(unsupported("list larger than its array"));
    }
    (0..size as u64)
        .map(|i| Ok(mem.read_i32(items + 0x20 + i * 4)?))
        .collect()
}

#[cfg(test)]
pub(crate) mod fake {
    //! A synthetic Mono runtime in a [`FakeMemory`], laid out with
    //! [`LAYOUT_BDWGC_X64`], for walker and reader tests.
    use super::*;
    use crate::memory::FakeMemory;
    use crate::pe::tests::fake_module;

    pub const MONO_BASE: u64 = 0x7ff0_0000_0000;
    const L: Layout = LAYOUT_BDWGC_X64;

    pub struct FakeMono {
        pub mem: FakeMemory,
        next: u64,
        assemblies: Vec<u64>,
    }

    pub struct FakeField<'a> {
        pub name: &'a str,
        pub offset: u64,
        pub is_static: bool,
    }

    /// Fields the fake types as `int`; every other field is a class.
    pub const INT_FIELDS: [&str; 6] = [
        "m_count",
        "<Rating>k__BackingField",
        "<DuosRating>k__BackingField",
        "_count",
        "touchedSlots",
        "m_int",
    ];

    impl FakeMono {
        pub fn new() -> FakeMono {
            let mut mem = FakeMemory::new();
            fake_module(&mut mem, MONO_BASE, &[("mono_get_root_domain", 0x5000)]);
            // mov rax, [rip + disp]; ret  → global at MONO_BASE + 0x6000.
            let disp = (0x6000i64 - (0x5000 + 7)) as i32;
            let mut code = vec![0x48, 0x8b, 0x05];
            code.extend_from_slice(&disp.to_le_bytes());
            code.push(0xc3);
            mem.put(MONO_BASE + 0x5000, &code);
            mem.zeroed(MONO_BASE + 0x6000, 8);
            let mut fake = FakeMono {
                mem,
                next: 0x1000_0000,
                assemblies: Vec::new(),
            };
            let domain = fake.alloc(0x200);
            fake.mem.put_u64(MONO_BASE + 0x6000, domain);
            fake
        }

        pub fn alloc(&mut self, len: usize) -> u64 {
            let at = self.next;
            self.mem.zeroed(at, len);
            self.next += (len as u64 + 0xfff) & !0xfff;
            at
        }

        pub fn string(&mut self, s: &str) -> u64 {
            let at = self.alloc(s.len() + 1);
            self.mem.put_str(at, s);
            at
        }

        pub fn domain(&self) -> u64 {
            use crate::memory::Memory;
            self.mem.read_u64(MONO_BASE + 0x6000).unwrap()
        }

        /// Adds an assembly and its image (with a one-bucket class cache);
        /// returns the image.
        pub fn assembly(&mut self, name: &str) -> u64 {
            let asm = self.alloc(0x100);
            let image = self.alloc(0x600);
            let name_at = self.string(name);
            self.mem.put_u64(asm + L.assembly_name, name_at);
            self.mem.put_u64(asm + L.assembly_image, image);
            self.mem.put_u64(image + L.image_assembly_name, name_at);
            let table = self.alloc(8);
            self.mem.put_u32(image + L.image_class_cache + 24, 1);
            self.mem.put_u64(image + L.image_class_cache + 32, table);
            self.assemblies.push(asm);
            self.relink_assemblies();
            image
        }

        fn relink_assemblies(&mut self) {
            let mut next = 0u64;
            for asm in self.assemblies.clone().into_iter().rev() {
                let node = self.alloc(16);
                self.mem.put_u64(node, asm);
                self.mem.put_u64(node + 8, next);
                next = node;
            }
            let domain = self.domain();
            self.mem.put_u64(domain + L.domain_assemblies, next);
        }

        /// Adds a class with fields to `image`; returns the class and its
        /// static data block (0x100 bytes).
        pub fn class(
            &mut self,
            image: u64,
            ns: &str,
            name: &str,
            fields: &[FakeField],
        ) -> (u64, u64) {
            use crate::memory::Memory;
            let class = self.alloc(0x120);
            let name_at = self.string(name);
            let ns_at = self.string(ns);
            self.mem.put_u64(class + L.class_image, image);
            self.mem.put_u64(class + L.class_name, name_at);
            self.mem.put_u64(class + L.class_namespace, ns_at);
            let entries = self.alloc(fields.len().max(1) * L.field_size as usize);
            for (i, f) in fields.iter().enumerate() {
                let e = entries + i as u64 * L.field_size;
                let ty = self.alloc(16);
                let attrs = if f.is_static { ATTR_STATIC } else { 0 };
                let code = if INT_FIELDS.contains(&f.name) {
                    TYPE_I4
                } else {
                    0x12
                };
                self.mem.put_u32(ty + L.type_attrs, attrs | (code << 16));
                let fname = self.string(f.name);
                self.mem.put_u64(e, ty);
                self.mem.put_u64(e + L.field_name, fname);
                self.mem.put_u64(e + L.field_parent, class);
                self.mem.put_u32(e + L.field_offset, f.offset as u32);
            }
            self.mem.put_u64(class + L.class_fields, entries);
            self.mem
                .put_u32(class + L.class_field_count, fields.len() as u32);
            // Statics: runtime info → vtable (3 method slots) → data.
            let data = self.alloc(0x100);
            let vtable = self.alloc(0x100);
            let info = self.alloc(0x20);
            self.mem.put_u64(vtable + L.vtable_class, class);
            self.mem.put_u32(class + L.class_vtable_size, 3);
            self.mem.put_u64(vtable + L.vtable_slots + 3 * 8, data);
            self.mem.put_u64(info + L.runtime_info_vtables, vtable);
            self.mem.put_u64(class + L.class_runtime_info, info);
            // Chain at the head of the single bucket.
            let table = self.mem.read_u64(image + L.image_class_cache + 32).unwrap();
            let head = self.mem.read_u64(table).unwrap();
            self.mem.put_u64(class + L.class_next_cached, head);
            self.mem.put_u64(table, class);
            (class, data)
        }

        /// An object of `class` with `len` bytes of fields after the header.
        pub fn object(&mut self, class: u64, len: usize) -> u64 {
            let obj = self.alloc(0x10 + len);
            let vtable = self.alloc(0x50);
            self.mem.put_u64(vtable + L.vtable_class, class);
            self.mem.put_u64(obj, vtable);
            obj
        }

        /// A class with only a name (for the type of an object).
        pub fn bare_class(&mut self, name: &str) -> u64 {
            let class = self.alloc(0x120);
            let name_at = self.string(name);
            self.mem.put_u64(class + L.class_name, name_at);
            class
        }

        /// Makes `object` one of a class named `name`.
        pub fn set_class_name(&mut self, object: u64, name: &str) {
            let class = self.bare_class(name);
            let vtable = self.alloc(0x50);
            self.mem.put_u64(vtable + L.vtable_class, class);
            self.mem.put_u64(object, vtable);
        }

        /// A `List<int>` object (`List`1`, items an `Int32[]`) holding `values`.
        pub fn int_list(&mut self, values: &[i32]) -> u64 {
            let list = self.alloc(0x20);
            let array = self.alloc(0x20 + 4 * values.len().max(8));
            self.set_class_name(list, "List`1");
            self.set_class_name(array, "Int32[]");
            self.mem.put_u64(array + 0x18, values.len().max(8) as u64);
            for (i, v) in values.iter().enumerate() {
                self.mem.put_u32(array + 0x20 + 4 * i as u64, *v as u32);
            }
            self.mem.put_u64(list + 0x10, array);
            self.mem.put_u32(list + 0x18, values.len() as u32);
            list
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::*;
    use super::*;

    fn game() -> (FakeMono, u64, u64, u64) {
        let mut f = FakeMono::new();
        f.assembly("mscorlib");
        let image = f.assembly("Assembly-CSharp");
        f.class(image, "", "Other", &[]);
        let (class, data) = f.class(
            image,
            "",
            "Thing",
            &[
                FakeField {
                    name: "s_instance",
                    offset: 8,
                    is_static: true,
                },
                FakeField {
                    name: "m_count",
                    offset: 0x14,
                    is_static: false,
                },
                FakeField {
                    name: "m_list",
                    offset: 0x18,
                    is_static: false,
                },
            ],
        );
        (f, image, class, data)
    }

    #[test]
    fn walks_from_the_root_domain_to_a_static_object() {
        let (mut f, image, class, data) = game();
        let obj = f.object(class, 0x20);
        f.mem.put_u64(data + 8, obj);
        f.mem.put_u32(obj + 0x14, 42);
        let mono = Mono::attach(&f.mem, MONO_BASE, LAYOUT_BDWGC_X64).unwrap();
        assert_eq!(mono.image("Assembly-CSharp").unwrap(), image);
        let found = mono.class(image, "", "Thing").unwrap();
        assert_eq!(found, class);
        let instance = mono.static_object(found, "s_instance").unwrap();
        assert_eq!(instance, obj);
        assert_eq!(mono.int_field(instance, found, "m_count").unwrap(), 42);
        assert_eq!(mono.object_class_name(instance).unwrap(), "Thing");
    }

    #[test]
    fn reads_an_int_list() {
        let (mut f, _, class, data) = game();
        let obj = f.object(class, 0x20);
        f.mem.put_u64(data + 8, obj);
        let list = f.int_list(&[20, 24, 14]);
        f.mem.put_u64(obj + 0x18, list);
        let mono = Mono::attach(&f.mem, MONO_BASE, LAYOUT_BDWGC_X64).unwrap();
        let l = mono.object_field(obj, class, "m_list").unwrap();
        assert_eq!(int_list(&f.mem, l, 8).unwrap(), vec![20, 24, 14]);
        assert!(int_list(&f.mem, l, 2).is_err());
    }

    #[test]
    fn unknown_runtime_code_stops_the_walk() {
        let (mut f, ..) = game();
        f.mem.put_u8(MONO_BASE + 0x5000, 0x90);
        assert!(matches!(
            Mono::attach(&f.mem, MONO_BASE, LAYOUT_BDWGC_X64),
            Err(WalkError::Unsupported(_))
        ));
    }

    #[test]
    fn missing_names_are_unsupported_not_guessed() {
        let (f, image, class, _) = game();
        let mono = Mono::attach(&f.mem, MONO_BASE, LAYOUT_BDWGC_X64).unwrap();
        assert!(matches!(mono.image("Nope"), Err(WalkError::Unsupported(_))));
        assert!(matches!(
            mono.class(image, "", "Nope"),
            Err(WalkError::Unsupported(_))
        ));
        assert!(matches!(
            mono.class(image, "Ns", "Thing"),
            Err(WalkError::Unsupported(_))
        ));
        assert!(matches!(
            mono.field(class, "nope"),
            Err(WalkError::Unsupported(_))
        ));
    }

    #[test]
    fn static_and_instance_fields_are_not_mixed_up() {
        let (f, _, class, _) = game();
        let mono = Mono::attach(&f.mem, MONO_BASE, LAYOUT_BDWGC_X64).unwrap();
        assert!(mono.static_object(class, "m_count").is_err());
        assert!(mono.int_field(0x1234, class, "s_instance").is_err());
    }

    #[test]
    fn unset_static_is_unsupported() {
        let (f, _, class, _) = game();
        let mono = Mono::attach(&f.mem, MONO_BASE, LAYOUT_BDWGC_X64).unwrap();
        assert_eq!(
            mono.static_object(class, "s_instance"),
            Err(WalkError::Unsupported("s_instance is not set".into()))
        );
    }

    #[test]
    fn a_wrong_layout_fails_a_back_pointer_check() {
        let (f, image, ..) = game();
        let mut wrong = LAYOUT_BDWGC_X64;
        wrong.class_image += 8;
        let mono = Mono::attach(&f.mem, MONO_BASE, wrong).unwrap();
        assert!(mono.class(image, "", "Thing").is_err());
    }

    #[test]
    fn a_vtable_of_another_class_is_refused() {
        let (mut f, _, class, _) = game();
        let info = f
            .mem
            .read_u64(class + LAYOUT_BDWGC_X64.class_runtime_info)
            .unwrap();
        let vtable = f.mem.read_u64(info + 8).unwrap();
        f.mem.put_u64(vtable, 0x999);
        let mono = Mono::attach(&f.mem, MONO_BASE, LAYOUT_BDWGC_X64).unwrap();
        assert!(matches!(
            mono.static_data(class),
            Err(WalkError::Unsupported(_))
        ));
    }

    use crate::memory::Memory;
}
