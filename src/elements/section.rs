use std::io;
use super::{
    Serialize,
    Deserialize,
    Unparsed,
    Error,
    VarUint7,
    VarUint32,
    CountedList,
    ImportEntry,
    MemoryType,
    TableType,
    ExportEntry,
    GlobalEntry,
    Func,
    FuncBody,
    ElementSegment,
    DataSegment,
    CountedWriter,
    CountedListWriter,
    External,
};

use super::types::Type;

/// Section in the WebAssembly module.
#[derive(Clone)]
pub enum Section {
    /// Section is unparsed.
    Unparsed {
        /// id of the unparsed section
        id: u8,
        /// raw bytes of the unparsed section
        payload: Vec<u8>,
    },
    /// Custom section (`id=0`)
    Custom(CustomSection),
    /// Types section
    Type(TypeSection),
    /// Import section
    Import(ImportSection),
    /// Function signatures section
    Function(FunctionSection),
    /// Table definition section
    Table(TableSection),
    /// Memory definition section
    Memory(MemorySection),
    /// Global entries section
    Global(GlobalSection),
    /// Export definitions
    Export(ExportSection),
    /// Entry reference of the module
    Start(u32),
    /// Elements section
    Element(ElementSection),
    /// Function bodies section
    Code(CodeSection),
    /// Data definition section
    Data(DataSection),
}

impl Deserialize for Section {
    type Error = Error;

    fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
        let id = match VarUint7::deserialize(reader) {
            // todo: be more selective detecting no more section
            Err(_) => { return Err(Error::UnexpectedEof); },
            Ok(id) => id,
        };

        Ok(
            match id.into() {
                0 => {
                    Section::Custom(CustomSection::deserialize(reader)?.into())
                },
                1 => {
                    Section::Type(TypeSection::deserialize(reader)?)
                },
                2 => {
                    Section::Import(ImportSection::deserialize(reader)?)
                },
                3 => {
                    Section::Function(FunctionSection::deserialize(reader)?)
                },
                4 => {
                    Section::Table(TableSection::deserialize(reader)?)
                },
                5 => {
                    Section::Memory(MemorySection::deserialize(reader)?)
                },
                6 => {
                    Section::Global(GlobalSection::deserialize(reader)?)
                },
                7 => {
                    Section::Export(ExportSection::deserialize(reader)?)
                },
                8 => {
                    let _section_length = VarUint32::deserialize(reader)?;
                    Section::Start(VarUint32::deserialize(reader)?.into())
                },
                9 => {
                    Section::Element(ElementSection::deserialize(reader)?)
                },
                10 => {
                    Section::Code(CodeSection::deserialize(reader)?)
                },
                11 => {
                    Section::Data(DataSection::deserialize(reader)?)
                },
                _ => {
                    Section::Unparsed { id: id.into(), payload: Unparsed::deserialize(reader)?.into() }
                }
            }
        )
    }
}

impl Serialize for Section {
    type Error = Error;

    fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
        match self {
            Section::Custom(custom_section) => {
                VarUint7::from(0x00).serialize(writer)?;
                custom_section.serialize(writer)?;
            },
            Section::Unparsed { id, payload } => {
                VarUint7::from(id).serialize(writer)?;
                writer.write_all(&payload[..])?;
            },
            Section::Type(type_section) => {
                VarUint7::from(0x01).serialize(writer)?;
                type_section.serialize(writer)?;
            },
            Section::Import(import_section) => {
                VarUint7::from(0x02).serialize(writer)?;
                import_section.serialize(writer)?;
            },
            Section::Function(function_section) => {
                VarUint7::from(0x03).serialize(writer)?;
                function_section.serialize(writer)?;
            },
            Section::Table(table_section) => {
                VarUint7::from(0x04).serialize(writer)?;
                table_section.serialize(writer)?;
            },
            Section::Memory(memory_section) => {
                VarUint7::from(0x05).serialize(writer)?;
                memory_section.serialize(writer)?;
            },
            Section::Global(global_section) => {
                VarUint7::from(0x06).serialize(writer)?;
                global_section.serialize(writer)?;
            },
            Section::Export(export_section) => {
                VarUint7::from(0x07).serialize(writer)?;
                export_section.serialize(writer)?;
            },
            Section::Start(index) => {
                VarUint7::from(0x08).serialize(writer)?;
                let mut counted_writer = CountedWriter::new(writer);
                VarUint32::from(index).serialize(&mut counted_writer)?;
                counted_writer.done()?;
            },
            Section::Element(element_section) => {
                VarUint7::from(0x09).serialize(writer)?;
                element_section.serialize(writer)?;
            },
            Section::Code(code_section) => {
                VarUint7::from(0x0a).serialize(writer)?;
                code_section.serialize(writer)?;
            },
            Section::Data(data_section) => {
                VarUint7::from(0x0b).serialize(writer)?;
                data_section.serialize(writer)?;
            },
        }
        Ok(())
    }
}

/// Custom section
#[derive(Clone)]
pub struct CustomSection {
    name: String,
    payload: Vec<u8>,
}

impl CustomSection {

    /// Name of the custom section
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Payload of the custom secion
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Name of the custom section (mutable)
    pub fn name_mut(&mut self) -> &mut String {
        &mut self.name
    }

    /// Payload of the custom section (mutable)
    pub fn payload_mut(&mut self) -> &mut Vec<u8> {
        &mut self.payload
    }
}

impl Deserialize for CustomSection {
    type Error = Error;

    fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
        // todo: maybe use reader.take(section_length)
        let section_length: u32 = VarUint32::deserialize(reader)?.into();

        let name = String::deserialize(reader)?;
        let payload_left = section_length - (name.len() as u32 + name.len() as u32 / 128 + 1);
        let mut payload = vec![0u8; payload_left as usize];
        reader.read_exact(&mut payload[..])?;

        Ok(CustomSection { name: name, payload: payload })
    }
}

impl Serialize for CustomSection {
    type Error = Error;

    fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
        use std::io::Write;

        let mut counted_writer = CountedWriter::new(writer);
        self.name.serialize(&mut counted_writer)?;
        counted_writer.write_all(&self.payload[..])?;
        counted_writer.done()?;
        Ok(())
    }
}

/// Section with type declarations
#[derive(Default, Clone)]
pub struct TypeSection(Vec<Type>);

impl TypeSection {
    ///  New type section with provided types
    pub fn with_types(types: Vec<Type>) -> Self {
        TypeSection(types)
    }

    /// List of type declarations
    pub fn types(&self) -> &[Type] {
        &self.0
    }

    /// List of type declarations (mutable)
    pub fn types_mut(&mut self) -> &mut Vec<Type> {
        &mut self.0
    }
}

impl Deserialize for TypeSection {
    type Error = Error;

    fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
        // todo: maybe use reader.take(section_length)
        let _section_length = VarUint32::deserialize(reader)?;
        let types: Vec<Type> = CountedList::deserialize(reader)?.into_inner();
        Ok(TypeSection(types))
    }
}

impl Serialize for TypeSection {
    type Error = Error;

    fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
        let mut counted_writer = CountedWriter::new(writer);
        let data = self.0;
        let counted_list = CountedListWriter::<Type, _>(
            data.len(),
            data.into_iter().map(Into::into),
        );
        counted_list.serialize(&mut counted_writer)?;
        counted_writer.done()?;
        Ok(())
    }
}

/// Section of the imports definition.
#[derive(Debug, Default, Clone)]
pub struct ImportSection(Vec<ImportEntry>);

impl ImportSection {
    ///  New import section with provided types
    pub fn with_entries(entries: Vec<ImportEntry>) -> Self {
        ImportSection(entries)
    }

    /// List of import entries.
    pub fn entries(&self) -> &[ImportEntry] {
        &self.0
    }

    /// List of import entries (mutable).
    pub fn entries_mut(&mut self) -> &mut Vec<ImportEntry> {
        &mut self.0
    }

    /// Returns number of functions
    pub fn functions(&self) -> usize {
        self.0.iter()
            .filter(|entry| match entry.external() { &External::Function(_) => true, _ => false })
            .count()
    }

    /// Returns number of globals
    pub fn globals(&self) -> usize {
        self.0.iter()
            .filter(|entry| match entry.external() { &External::Global(_) => true, _ => false })
            .count()
    }
}

impl Deserialize for ImportSection {
    type Error = Error;

    fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
        // todo: maybe use reader.take(section_length)
        let _section_length = VarUint32::deserialize(reader)?;
        let entries: Vec<ImportEntry> = CountedList::deserialize(reader)?.into_inner();
        Ok(ImportSection(entries))
    }
}

impl Serialize for ImportSection {
    type Error = Error;

    fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
        let mut counted_writer = CountedWriter::new(writer);
        let data = self.0;
        let counted_list = CountedListWriter::<ImportEntry, _>(
            data.len(),
            data.into_iter().map(Into::into),
        );
        counted_list.serialize(&mut counted_writer)?;
        counted_writer.done()?;
        Ok(())
    }
}

/// Section with function signatures definition.
#[derive(Default, Clone)]
pub struct FunctionSection(Vec<Func>);

impl FunctionSection {
    ///  New function signatures section with provided entries
    pub fn with_entries(entries: Vec<Func>) -> Self {
        FunctionSection(entries)
    }

    /// List of all functions in the section, mutable
    pub fn entries_mut(&mut self) -> &mut Vec<Func> {
        &mut self.0
    }

    /// List of all functions in the section
    pub fn entries(&self) -> &[Func] {
        &self.0
    }
}

impl Deserialize for FunctionSection {
    type Error = Error;

    fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
        // todo: maybe use reader.take(section_length)
        let _section_length = VarUint32::deserialize(reader)?;
        let funcs: Vec<Func> = CountedList::<VarUint32>::deserialize(reader)?
            .into_inner()
            .into_iter()
            .map(|f| Func::new(f.into()))
            .collect();
        Ok(FunctionSection(funcs))
    }
}

impl Serialize for FunctionSection {
    type Error = Error;

    fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
        let mut counted_writer = CountedWriter::new(writer);
        let data = self.0;
        let counted_list = CountedListWriter::<VarUint32, _>(
            data.len(),
            data.into_iter().map(|func| func.type_ref().into())
        );
        counted_list.serialize(&mut counted_writer)?;
        counted_writer.done()?;
        Ok(())
    }
}

/// Section with table definition (currently only one is allowed).
#[derive(Default, Clone)]
pub struct TableSection(Vec<TableType>);

impl TableSection {
    /// Table entries.
    pub fn entries(&self) -> &[TableType] {
        &self.0
    }

    ///  New table section with provided table entries
    pub fn with_entries(entries: Vec<TableType>) -> Self {
        TableSection(entries)
    }

    /// Mutable table entries.
    pub fn entries_mut(&mut self) -> &mut Vec<TableType> {
        &mut self.0
    }
}

impl Deserialize for TableSection {
    type Error = Error;

    fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
        // todo: maybe use reader.take(section_length)
        let _section_length = VarUint32::deserialize(reader)?;
        let entries: Vec<TableType> = CountedList::deserialize(reader)?.into_inner();
        Ok(TableSection(entries))
    }
}

impl Serialize for TableSection {
    type Error = Error;

    fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
        let mut counted_writer = CountedWriter::new(writer);
        let data = self.0;
        let counted_list = CountedListWriter::<TableType, _>(
            data.len(),
            data.into_iter().map(Into::into),
        );
        counted_list.serialize(&mut counted_writer)?;
        counted_writer.done()?;
        Ok(())
    }
}

/// Section with table definition (currently only one entry is allowed).
#[derive(Default, Clone)]
pub struct MemorySection(Vec<MemoryType>);

impl MemorySection {
    /// List of all memory entries in the section
    pub fn entries(&self) -> &[MemoryType] {
        &self.0
    }

    ///  New memory section with memory types
    pub fn with_entries(entries: Vec<MemoryType>) -> Self {
        MemorySection(entries)
    }

    /// Mutable list of all memory entries in the section
    pub fn entries_mut(&mut self) -> &mut Vec<MemoryType> {
        &mut self.0
    }
}

impl Deserialize for MemorySection {
    type Error = Error;

    fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
        // todo: maybe use reader.take(section_length)
        let _section_length = VarUint32::deserialize(reader)?;
        let entries: Vec<MemoryType> = CountedList::deserialize(reader)?.into_inner();
        Ok(MemorySection(entries))
    }
}

impl Serialize for MemorySection {
    type Error = Error;

    fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
        let mut counted_writer = CountedWriter::new(writer);
        let data = self.0;
        let counted_list = CountedListWriter::<MemoryType, _>(
            data.len(),
            data.into_iter().map(Into::into),
        );
        counted_list.serialize(&mut counted_writer)?;
        counted_writer.done()?;
        Ok(())
    }
}

/// Globals definition section.
#[derive(Default, Clone)]
pub struct GlobalSection(Vec<GlobalEntry>);

impl GlobalSection {
    /// List of all global entries in the section
    pub fn entries(&self) -> &[GlobalEntry] {
        &self.0
    }

    /// New global section from list of global entries
    pub fn with_entries(entries: Vec<GlobalEntry>) -> Self {
        GlobalSection(entries)
    }

    /// List of all global entries in the section (mutable)
    pub fn entries_mut(&mut self) -> &mut Vec<GlobalEntry> {
        &mut self.0
    }
}

impl Deserialize for GlobalSection {
    type Error = Error;

    fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
        // todo: maybe use reader.take(section_length)
        let _section_length = VarUint32::deserialize(reader)?;
        let entries: Vec<GlobalEntry> = CountedList::deserialize(reader)?.into_inner();
        Ok(GlobalSection(entries))
    }
}

impl Serialize for GlobalSection {
    type Error = Error;

    fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
        let mut counted_writer = CountedWriter::new(writer);
        let data = self.0;
        let counted_list = CountedListWriter::<GlobalEntry, _>(
            data.len(),
            data.into_iter().map(Into::into),
        );
        counted_list.serialize(&mut counted_writer)?;
        counted_writer.done()?;
        Ok(())
    }
}

/// List of exports definition.
#[derive(Debug, Default, Clone)]
pub struct ExportSection(Vec<ExportEntry>);

impl ExportSection {
    /// List of all export entries in the section
    pub fn entries(&self) -> &[ExportEntry] {
        &self.0
    }

    /// New export section from list of export entries
    pub fn with_entries(entries: Vec<ExportEntry>) -> Self {
        ExportSection(entries)
    }

    /// List of all export entries in the section (mutable)
    pub fn entries_mut(&mut self) -> &mut Vec<ExportEntry> {
        &mut self.0
    }
}

impl Deserialize for ExportSection {
    type Error = Error;

    fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
        // todo: maybe use reader.take(section_length)
        let _section_length = VarUint32::deserialize(reader)?;
        let entries: Vec<ExportEntry> = CountedList::deserialize(reader)?.into_inner();
        Ok(ExportSection(entries))
    }
}

impl Serialize for ExportSection {
    type Error = Error;

    fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
        let mut counted_writer = CountedWriter::new(writer);
        let data = self.0;
        let counted_list = CountedListWriter::<ExportEntry, _>(
            data.len(),
            data.into_iter().map(Into::into),
        );
        counted_list.serialize(&mut counted_writer)?;
        counted_writer.done()?;
        Ok(())
    }
}

/// Section with function bodies of the module.
#[derive(Default, Clone)]
pub struct CodeSection(Vec<FuncBody>);

impl CodeSection {
    /// New code section with specified function bodies
    pub fn with_bodies(bodies: Vec<FuncBody>) -> Self {
        CodeSection(bodies)
    }

    /// All function bodies in the section.
    pub fn bodies(&self) -> &[FuncBody] {
        &self.0
    }

    /// All function bodies in the section, mutable.
    pub fn bodies_mut(&mut self) -> &mut Vec<FuncBody> {
        &mut self.0
    }
}

impl Deserialize for CodeSection {
    type Error = Error;

    fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
        // todo: maybe use reader.take(section_length)
        let _section_length = VarUint32::deserialize(reader)?;
        let entries: Vec<FuncBody> = CountedList::deserialize(reader)?.into_inner();
        Ok(CodeSection(entries))
    }
}

impl Serialize for CodeSection {
    type Error = Error;

    fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
        let mut counted_writer = CountedWriter::new(writer);
        let data = self.0;
        let counted_list = CountedListWriter::<FuncBody, _>(
            data.len(),
            data.into_iter().map(Into::into),
        );
        counted_list.serialize(&mut counted_writer)?;
        counted_writer.done()?;
        Ok(())
    }
}

/// Element entries section.
#[derive(Default, Clone)]
pub struct ElementSection(Vec<ElementSegment>);

impl ElementSection {
    /// New elements section
    pub fn with_entries(entries: Vec<ElementSegment>) -> Self {
        ElementSection(entries)
    }

    /// New elements entries in the section
    pub fn entries(&self) -> &[ElementSegment] {
        &self.0
    }

    /// List of all data entries in the section (mutable)
    pub fn entries_mut(&mut self) -> &mut Vec<ElementSegment> {
        &mut self.0
    }
}

impl Deserialize for ElementSection {
    type Error = Error;

    fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
        // todo: maybe use reader.take(section_length)
        let _section_length = VarUint32::deserialize(reader)?;
        let entries: Vec<ElementSegment> = CountedList::deserialize(reader)?.into_inner();
        Ok(ElementSection(entries))
    }
}

impl Serialize for ElementSection {
    type Error = Error;

    fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
        let mut counted_writer = CountedWriter::new(writer);
        let data = self.0;
        let counted_list = CountedListWriter::<ElementSegment, _>(
            data.len(),
            data.into_iter().map(Into::into),
        );
        counted_list.serialize(&mut counted_writer)?;
        counted_writer.done()?;
        Ok(())
    }
}

/// Data entries definitions.
#[derive(Default, Clone)]
pub struct DataSection(Vec<DataSegment>);

impl DataSection {
    /// New data section
    pub fn with_entries(entries: Vec<DataSegment>) -> Self {
        DataSection(entries)
    }

    /// List of all data entries in the section
    pub fn entries(&self) -> &[DataSegment] {
        &self.0
    }

    /// List of all data entries in the section (mutable)
    pub fn entries_mut(&mut self) -> &mut Vec<DataSegment> {
        &mut self.0
    }
}

impl Deserialize for DataSection {
    type Error = Error;

    fn deserialize<R: io::Read>(reader: &mut R) -> Result<Self, Self::Error> {
        // todo: maybe use reader.take(section_length)
        let _section_length = VarUint32::deserialize(reader)?;
        let entries: Vec<DataSegment> = CountedList::deserialize(reader)?.into_inner();
        Ok(DataSection(entries))
    }
}

impl Serialize for DataSection {
    type Error = Error;

    fn serialize<W: io::Write>(self, writer: &mut W) -> Result<(), Self::Error> {
        let mut counted_writer = CountedWriter::new(writer);
        let data = self.0;
        let counted_list = CountedListWriter::<DataSegment, _>(
            data.len(),
            data.into_iter().map(Into::into),
        );
        counted_list.serialize(&mut counted_writer)?;
        counted_writer.done()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::super::{
        deserialize_buffer, deserialize_file, ValueType, InitExpr, DataSegment,
        serialize, ElementSegment, Opcodes, BlockType, Local, FuncBody,
    };
    use super::{Section, TypeSection, Type, DataSection, ElementSection, CodeSection};

    #[test]
    fn import_section() {
        let module = deserialize_file("./res/cases/v1/test5.wasm").expect("Should be deserialized");
        let mut found = false;
        for section in module.sections() {
            match section {
                &Section::Import(ref import_section) => {
                    assert_eq!(25, import_section.entries().len());
                    found = true
                },
                _ => { }
            }
        }
        assert!(found, "There should be import section in test5.wasm");
    }

    fn functions_test_payload() -> Vec<u8> {
        vec![
            // functions section id
            0x03u8,
            // functions section length
            0x87, 0x80, 0x80, 0x80, 0x0,
            // number of functions
            0x04,
            // type reference 1
            0x01,
            // type reference 2
            0x86, 0x80, 0x00,
            // type reference 3
            0x09,
            // type reference 4
            0x33
        ]
    }

    #[test]
    fn fn_section_detect() {
        let section: Section =
            deserialize_buffer(functions_test_payload()).expect("section to be deserialized");

        match section {
            Section::Function(_) => {},
            _ => {
                panic!("Payload should be recognized as functions section")
            }
        }
    }

    #[test]
    fn fn_section_number() {
        let section: Section =
            deserialize_buffer(functions_test_payload()).expect("section to be deserialized");

        match section {
            Section::Function(fn_section) => {
                assert_eq!(4, fn_section.entries().len(), "There should be 4 functions total");
            },
            _ => {
                // will be catched by dedicated test
            }
        }
    }

    #[test]
    fn fn_section_ref() {
        let section: Section =
            deserialize_buffer(functions_test_payload()).expect("section to be deserialized");

        match section {
            Section::Function(fn_section) => {
                assert_eq!(6, fn_section.entries()[1].type_ref());
            },
            _ => {
                // will be catched by dedicated test
            }
        }
    }

    fn types_test_payload() -> Vec<u8> {
        vec![
            // section length
            148u8, 0x80, 0x80, 0x80, 0x0,

            // 2 functions
            130u8, 0x80, 0x80, 0x80, 0x0,
            // func 1, form =1
            0x01,
            // param_count=1
            129u8, 0x80, 0x80, 0x80, 0x0,
                // first param
                0x7e, // i64
            // no return params
            0x00,

            // func 2, form=1
            0x01,
            // param_count=1
            130u8, 0x80, 0x80, 0x80, 0x0,
                // first param
                0x7e,
                // second param
                0x7d,
            // return param (is_present, param_type)
            0x01, 0x7e
        ]
    }

    #[test]
    fn type_section_len() {
        let type_section: TypeSection =
            deserialize_buffer(types_test_payload()).expect("type_section be deserialized");

        assert_eq!(type_section.types().len(), 2);
    }

    #[test]
    fn type_section_infer() {
        let type_section: TypeSection =
            deserialize_buffer(types_test_payload()).expect("type_section be deserialized");

        let t1 = match &type_section.types()[1] {
            &Type::Function(ref func_type) => func_type
        };

        assert_eq!(Some(ValueType::I64), t1.return_type());
        assert_eq!(2, t1.params().len());
    }

    fn export_payload() -> Vec<u8> {
        vec![
            // section id
            0x07,
            // section length
            148u8, 0x80, 0x80, 0x80, 0x0,
            // 6 entries
            134u8, 0x80, 0x80, 0x80, 0x0,
            // func "A", index 6
            // [name_len(1-5 bytes), name_bytes(name_len, internal_kind(1byte), internal_index(1-5 bytes)])
            0x01, 0x41,  0x01, 0x86, 0x80, 0x00,
            // func "B", index 8
            0x01, 0x42,  0x01, 0x86, 0x00,
            // func "C", index 7
            0x01, 0x43,  0x01, 0x07,
            // memory "D", index 0
            0x01, 0x44,  0x02, 0x00,
            // func "E", index 1
            0x01, 0x45,  0x01, 0x01,
            // func "F", index 2
            0x01, 0x46,  0x01, 0x02
        ]
    }


    #[test]
    fn export_detect() {
        let section: Section =
            deserialize_buffer(export_payload()).expect("section to be deserialized");

        match section {
            Section::Export(_) => {},
            _ => {
                panic!("Payload should be recognized as export section")
            }
        }
    }

    fn code_payload() -> Vec<u8> {
        vec![
            // sectionid
            0x0Au8,
            // section length, 32
            0x20,
            // body count
            0x01,
            // body 1, length 30
            0x1E,
            0x01, 0x01, 0x7F, // local i32 (one collection of length one of type i32)
            0x02, 0x7F, // block i32
                0x23, 0x00, // get_global 0
                0x21, 0x01, // set_local 1
                0x23, 0x00, // get_global 0
                0x20, 0x00, // get_local 0
                0x6A,       // i32.add
                0x24, 0x00, // set_global 0
                0x23, 0x00, // get_global 0
                0x41, 0x0F, // i32.const 15
                0x6A,       // i32.add
                0x41, 0x70, // i32.const -16
                0x71,       // i32.and
                0x24, 0x00, // set_global 0
                0x20, 0x01, // get_local 1
            0x0B,
            0x0B,
        ]
    }

    #[test]
    fn code_detect() {

        let section: Section =
            deserialize_buffer(code_payload()).expect("section to be deserialized");

        match section {
            Section::Code(_) => {},
            _ => {
                panic!("Payload should be recognized as a code section")
            }
        }
    }

    fn data_payload() -> Vec<u8> {
        vec![
            0x0bu8,  // section id
            19,      // 19 bytes overall
            0x01,    // number of segments
            0x00,    // index
            0x0b,    // just `end` op
            // 16x 0x00
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00
        ]
    }

    #[test]
    fn data_section_ser() {
        let data_section = DataSection::with_entries(
            vec![DataSegment::new(0u32, InitExpr::empty(), vec![0u8; 16])]
        );

        let buf = serialize(data_section).expect("Data section to be serialized");

        assert_eq!(buf, vec![
            20u8, // 19 bytes overall
            0x01, // number of segments
            0x00, // index
            0x0b, // just `end` op
            16,   // value of length 16
            0x00, 0x00, 0x00, 0x00, // 16x 0x00 as in initialization
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00
        ]);
    }

    #[test]
    fn data_section_detect() {
        let section: Section =
            deserialize_buffer(data_payload()).expect("section to be deserialized");

        match section {
            Section::Data(_) => {},
            _ => {
                panic!("Payload should be recognized as a data section")
            }
        }
    }

    #[test]
    fn element_section_ser() {
        let element_section = ElementSection::with_entries(
            vec![ElementSegment::new(0u32, InitExpr::empty(), vec![0u32; 4])]
        );

        let buf = serialize(element_section).expect("Element section to be serialized");

        assert_eq!(buf, vec![
            08u8, // 8 bytes overall
            0x01, // number of segments
            0x00, // index
            0x0b, // just `end` op
            0x04, // 4 elements
            0x00, 0x00, 0x00, 0x00 // 4x 0x00 as in initialization
        ]);
    }

    #[test]
    fn code_section_ser() {
        use super::super::Opcode::*;

        let code_section = CodeSection::with_bodies(
            vec![
                FuncBody::new(
                    vec![Local::new(1, ValueType::I32)],
                    Opcodes::new(vec![
                        Block(BlockType::Value(ValueType::I32)),
                        GetGlobal(0),
                        End,
                        End,
                    ])
                )
            ]);

        let buf = serialize(code_section).expect("Code section to be serialized");

        assert_eq!(buf, vec![
            11u8,            // 11 bytes total section size
            0x01,            // 1 function
              9,             //   function #1 total code size
              1,             //   1 local variable declaration
              1,             //      amount of variables
              0x7f,          //      type of variable (7-bit, -0x01), negative
              0x02,          //   block
                0x7f,        //      block return type (7-bit, -0x01), negative
                0x23, 0x00,  //      get_global(0)
                0x0b,        //   block end
            0x0b,            // function end
        ]);
    }

    #[test]
    fn start_section() {
        let section: Section = deserialize_buffer(vec![08u8, 01u8, 00u8]).expect("Start section to deserialize");
        if let Section::Start(_) = section {
        } else {
            panic!("Payload should be a start section");
        }

        let serialized = serialize(section).expect("Start section to successfully serializen");

        assert_eq!(serialized, vec![08u8, 01u8, 00u8]);
    }
}
