use std::collections::HashMap;

use boltffi_ast::{
    ClassDef as SourceClass, EnumDef as SourceEnum, EnumId as SourceEnumId,
    RecordDef as SourceRecord, RecordId as SourceRecordId, SourceContract,
};

/// Borrowed view over a [`SourceContract`] with lookup tables for the
/// declarations the lowering pass dereferences while walking type
/// expressions.
///
/// The pass needs to read the source record or enum behind a
/// `TypeExpr::Record(id)` or `TypeExpr::Enum(id)` to decide whether a
/// nested reference codes as direct memory or encoded bytes. Storing
/// references rather than copying the source keeps construction cheap
/// and ties every lookup to the lifetime of the input.
pub(super) struct Index<'src> {
    source: &'src SourceContract,
    records: HashMap<&'src str, &'src SourceRecord>,
    enums: HashMap<&'src str, &'src SourceEnum>,
}

impl<'src> Index<'src> {
    pub(super) fn new(source: &'src SourceContract) -> Self {
        Self {
            source,
            records: source
                .records
                .iter()
                .map(|record| (record.id.as_str(), record))
                .collect(),
            enums: source
                .enums
                .iter()
                .map(|enumeration| (enumeration.id.as_str(), enumeration))
                .collect(),
        }
    }

    pub(super) fn source(&self) -> &'src SourceContract {
        self.source
    }

    pub(super) fn records(&self) -> &'src [SourceRecord] {
        &self.source.records
    }

    pub(super) fn enums(&self) -> &'src [SourceEnum] {
        &self.source.enums
    }

    pub(super) fn classes(&self) -> &'src [SourceClass] {
        &self.source.classes
    }

    pub(super) fn record(&self, id: &SourceRecordId) -> Option<&'src SourceRecord> {
        self.records.get(id.as_str()).copied()
    }

    pub(super) fn enumeration(&self, id: &SourceEnumId) -> Option<&'src SourceEnum> {
        self.enums.get(id.as_str()).copied()
    }
}
