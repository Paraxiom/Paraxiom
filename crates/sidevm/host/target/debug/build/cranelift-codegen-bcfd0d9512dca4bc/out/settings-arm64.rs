#[derive(Clone, Hash)]
/// Flags group `arm64`.
pub struct Flags {
    bytes: [u8; 1],
}
impl Flags {
    /// Create flags arm64 settings group.
    #[allow(unused_variables)]
    pub fn new(shared: &settings::Flags, builder: Builder) -> Self {
        let bvec = builder.state_for("arm64");
        let mut arm64 = Self { bytes: [0; 1] };
        debug_assert_eq!(bvec.len(), 1);
        arm64.bytes[0..1].copy_from_slice(&bvec);
        // Precompute #1.
        if arm64.has_lse() {
            arm64.bytes[0] |= 1 << 1;
        }
        arm64
    }
}
impl Flags {
    /// Iterates the setting values.
    pub fn iter(&self) -> impl Iterator<Item = Value> {
        let mut bytes = [0; 1];
        bytes.copy_from_slice(&self.bytes[0..1]);
        DESCRIPTORS.iter().filter_map(move |d| {
            let values = match &d.detail {
                detail::Detail::Preset => return None,
                detail::Detail::Enum { last, enumerators } => Some(TEMPLATE.enums(*last, *enumerators)),
                _ => None
            };
            Some(Value{ name: d.name, detail: d.detail, values, value: bytes[d.offset as usize] })
        })
    }
}
/// User-defined settings.
#[allow(dead_code)]
impl Flags {
    /// Get a view of the boolean predicates.
    pub fn predicate_view(&self) -> crate::settings::PredicateView {
        crate::settings::PredicateView::new(&self.bytes[0..])
    }
    /// Dynamic numbered predicate getter.
    fn numbered_predicate(&self, p: usize) -> bool {
        self.bytes[0 + p / 8] & (1 << (p % 8)) != 0
    }
    /// Has Large System Extensions support.
    pub fn has_lse(&self) -> bool {
        self.numbered_predicate(0)
    }
    /// Computed predicate `arm64.has_lse()`.
    pub fn use_lse(&self) -> bool {
        self.numbered_predicate(1)
    }
}
static DESCRIPTORS: [detail::Descriptor; 1] = [
    detail::Descriptor {
        name: "has_lse",
        description: "Has Large System Extensions support.",
        offset: 0,
        detail: detail::Detail::Bool { bit: 0 },
    },
];
static ENUMERATORS: [&str; 0] = [
];
static HASH_TABLE: [u16; 2] = [
    0xffff,
    0,
];
static PRESETS: [(u8, u8); 0] = [
];
static TEMPLATE: detail::Template = detail::Template {
    name: "arm64",
    descriptors: &DESCRIPTORS,
    enumerators: &ENUMERATORS,
    hash_table: &HASH_TABLE,
    defaults: &[0x00],
    presets: &PRESETS,
};
/// Create a `settings::Builder` for the arm64 settings group.
pub fn builder() -> Builder {
    Builder::new(&TEMPLATE)
}
impl fmt::Display for Flags {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "[arm64]")?;
        for d in &DESCRIPTORS {
            if !d.detail.is_preset() {
                write!(f, "{} = ", d.name)?;
                TEMPLATE.format_toml_value(d.detail, self.bytes[d.offset as usize], f)?;
                writeln!(f)?;
            }
        }
        Ok(())
    }
}
