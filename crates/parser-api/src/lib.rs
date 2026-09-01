//! Application-facing composition for reusable parser profiles.
//!
//! This crate owns no detection rules. A profile compiles the caller's field
//! vocabulary once, then passes the resulting plan to the existing format
//! adapters and parser pipeline.

use parser_core::{Failure, ParseLimits, ParseResponse, TableSelectionOptions};
use parser_formats::{
    CsvOptions, InputSource, TextLimits, parse_extracted_table_with_plan_and_limits,
    read_csv_bytes, read_csv_table_bytes, read_input, read_txt_bytes, read_xlsx_bytes,
    read_xlsx_table_bytes,
};
use parser_schema::{
    FieldConstraint, FieldDefinition, FieldType, SchemaOptions, TargetSchema, TextPipelineOptions,
    compile_schema,
};

/// A reusable application-owned vocabulary. `name` and `version` identify the
/// caller's profile; the engine never interprets either as business meaning.
#[derive(Debug, Clone)]
pub struct ApplicationProfile {
    name: String,
    version: String,
    schema: TargetSchema,
    plan: parser_core::ParsePlan,
}

impl ApplicationProfile {
    /// Starts a typed profile definition. `build` validates supported parser
    /// capabilities and compiles the reusable execution plan before input arrives.
    pub fn define(name: impl Into<String>, version: impl Into<String>) -> ProfileBuilder {
        ProfileBuilder::new(name, version)
    }

    /// Turns an already typed schema into a named reusable profile without JSON
    /// decoding. This keeps low-level schema users compatible while giving them
    /// the same early compiler validation.
    pub fn from_schema(
        name: impl Into<String>,
        version: impl Into<String>,
        schema: TargetSchema,
    ) -> Result<Self, ProfileError> {
        let name = name.into();
        let version = version.into();
        validate_identity(&name, &version)?;
        let plan = compile_schema(&schema).map_err(ProfileError::UnsupportedCapability)?;
        Ok(Self {
            name,
            version,
            schema,
            plan,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn version(&self) -> &str {
        &self.version
    }
    pub fn schema(&self) -> &TargetSchema {
        &self.schema
    }

    /// The single application-facing parse entry point for text, TXT, CSV and XLSX.
    pub fn parse(
        &self,
        input: ApplicationInput<'_>,
        options: ApplicationParseOptions,
    ) -> Result<ParseResponse, Failure> {
        if let Some(selection) = options.table_selection {
            let table = match input {
                ApplicationInput::Csv { bytes, file_name } => read_csv_table_bytes(
                    file_name,
                    bytes,
                    "<application input>",
                    CsvOptions::default(),
                ),
                ApplicationInput::Xlsx { bytes, file_name } => {
                    read_xlsx_table_bytes(file_name, bytes)
                }
                ApplicationInput::Text(_) | ApplicationInput::Txt { .. } => {
                    return Err(Failure::new(parser_core::FailureKind::TableSelection {
                        reason: parser_core::TableSelectionReason::UnsupportedSource,
                    }));
                }
            }
            .map_err(|error| Failure::from(&error))?;
            return parse_extracted_table_with_plan_and_limits(
                &table,
                &self.plan,
                &selection,
                options.limits,
            );
        }

        let document = match input {
            ApplicationInput::Text(text) => {
                read_input(InputSource::Text(text), TextLimits::default())
            }
            ApplicationInput::Txt { bytes, file_name } => {
                read_txt_bytes(file_name, bytes, "<application input>")
            }
            ApplicationInput::Csv { bytes, file_name } => read_csv_bytes(
                file_name,
                bytes,
                "<application input>",
                CsvOptions::default(),
            ),
            ApplicationInput::Xlsx { bytes, file_name } => read_xlsx_bytes(file_name, bytes),
        }
        .map_err(|error| Failure::from(&error))?;
        parser_core::parse_document_with_plan_with_limits(&document, &self.plan, options.limits)
    }
}

/// Input bytes remain borrowed by the caller. The parser never opens a path or
/// evaluates workbook formulas, macros, scripts or links through this API.
#[derive(Debug, Clone, Copy)]
pub enum ApplicationInput<'a> {
    Text(&'a str),
    Txt {
        bytes: &'a [u8],
        file_name: Option<&'a str>,
    },
    Csv {
        bytes: &'a [u8],
        file_name: Option<&'a str>,
    },
    Xlsx {
        bytes: &'a [u8],
        file_name: Option<&'a str>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct ApplicationParseOptions {
    pub table_selection: Option<TableSelectionOptions>,
    pub limits: ParseLimits,
}

/// Typed profile construction. It intentionally exposes only caller-owned
/// vocabulary; parsing behavior is compiled by parser-schema, not recreated here.
#[derive(Debug, Clone)]
pub struct ProfileBuilder {
    name: String,
    version: String,
    record_name: Option<String>,
    fields: Vec<FieldDefinition>,
    options: SchemaOptions,
}

impl ProfileBuilder {
    fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            record_name: None,
            fields: Vec::new(),
            options: SchemaOptions::default(),
        }
    }

    pub fn record_name(mut self, value: impl Into<String>) -> Self {
        self.record_name = Some(value.into());
        self
    }

    pub fn field(mut self, field: ProfileField) -> Self {
        self.fields.push(field.into_definition());
        self
    }

    /// Applies the already documented optional text composition settings.
    pub fn text_pipeline(mut self, value: TextPipelineOptions) -> Self {
        self.options.text_pipeline = Some(value);
        self
    }

    /// `true` is the currently supported execution capability. Setting false is
    /// retained so the shared compiler returns its stable typed failure at build time.
    pub fn allow_unknown_fields(mut self, value: bool) -> Self {
        self.options.allow_unknown_fields = value;
        self
    }

    pub fn build(self) -> Result<ApplicationProfile, ProfileError> {
        ApplicationProfile::from_schema(
            self.name,
            self.version,
            TargetSchema {
                schema_version: parser_schema::SCHEMA_VERSION.to_owned(),
                record_name: self.record_name,
                fields: self.fields,
                options: self.options,
            },
        )
    }
}

#[derive(Debug, Clone)]
pub struct ProfileField {
    definition: FieldDefinition,
}

impl ProfileField {
    pub fn required(name: impl Into<String>, field_type: FieldType) -> Self {
        Self::new(name, field_type, true)
    }
    pub fn optional(name: impl Into<String>, field_type: FieldType) -> Self {
        Self::new(name, field_type, false)
    }
    fn new(name: impl Into<String>, field_type: FieldType, required: bool) -> Self {
        Self {
            definition: FieldDefinition {
                name: name.into(),
                field_type,
                required,
                multiple: false,
                aliases: Vec::new(),
                constraints: Vec::new(),
            },
        }
    }
    pub fn aliases(mut self, values: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.definition
            .aliases
            .extend(values.into_iter().map(Into::into));
        self
    }
    pub fn multiple(mut self) -> Self {
        self.definition.multiple = true;
        self
    }
    pub fn constraint(mut self, value: FieldConstraint) -> Self {
        self.definition.constraints.push(value);
        self
    }
    fn into_definition(self) -> FieldDefinition {
        self.definition
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileError {
    EmptyName,
    EmptyVersion,
    UnsupportedCapability(Failure),
}

impl std::fmt::Display for ProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyName => f.write_str("profile name must not be empty"),
            Self::EmptyVersion => f.write_str("profile version must not be empty"),
            Self::UnsupportedCapability(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for ProfileError {}

fn validate_identity(name: &str, version: &str) -> Result<(), ProfileError> {
    if name.trim().is_empty() {
        return Err(ProfileError::EmptyName);
    }
    if version.trim().is_empty() {
        return Err(ProfileError::EmptyVersion);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use parser_core::{CandidateType, ParseContent, SourceType};

    fn profile() -> ApplicationProfile {
        ApplicationProfile::define("generic-contact", "2026-08")
            .record_name("contact")
            .field(ProfileField::required("person", FieldType::PersonName).aliases(["Name"]))
            .field(ProfileField::required("phone", FieldType::PhoneNumber))
            .field(ProfileField::optional("amount", FieldType::Currency))
            .field(ProfileField::optional("notes", FieldType::Text))
            .build()
            .unwrap()
    }

    fn fields(response: &ParseResponse) -> &[parser_core::AssignedField] {
        match &response.content {
            ParseContent::Text { records } => &records[0].parse.assignment.fields,
            ParseContent::Table { sheets } => &sheets[0].records[0].parse.assignment.fields,
        }
    }

    #[test]
    fn one_profile_handles_present_and_absent_optional_fields_across_text_csv_and_xlsx() {
        let profile = profile();
        let text = profile
            .parse(
                ApplicationInput::Text(
                    "Name: Ada; Phone: +255700000000; Amount: $42; Notes: early",
                ),
                Default::default(),
            )
            .unwrap();
        let csv = profile
            .parse(
                ApplicationInput::Csv {
                    bytes: b"Name,Phone\nAda,+255700000000\n",
                    file_name: Some("contacts.csv"),
                },
                Default::default(),
            )
            .unwrap();
        let xlsx = profile
            .parse(
                ApplicationInput::Xlsx {
                    bytes: include_bytes!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../../fixtures/xlsx/sample.xlsx"
                    )),
                    file_name: Some("contacts.xlsx"),
                },
                Default::default(),
            )
            .unwrap();
        assert_eq!(text.source_type, SourceType::Text);
        assert!(fields(&text).iter().any(|field| field.name == "amount"));
        assert!(!fields(&csv).iter().any(|field| field.name == "amount"));
        assert_eq!(xlsx.source_type, SourceType::Xlsx);
        assert!(xlsx.source_evidence.is_some());
        assert!(fields(&xlsx).iter().all(|field| {
            field
                .candidates
                .iter()
                .all(|candidate| candidate.candidate_type != CandidateType::Currency)
        }));
    }

    #[test]
    fn profile_builds_capabilities_early_and_keeps_domain_meaning_with_the_caller() {
        let error = ApplicationProfile::define("inventory", "1")
            .field(ProfileField::required("when", FieldType::Datetime))
            .build()
            .unwrap_err();
        assert!(matches!(error, ProfileError::UnsupportedCapability(_)));
        assert!(
            ProfileField::optional("tags", FieldType::Text)
                .multiple()
                .into_definition()
                .multiple
        );
    }

    #[test]
    fn unrelated_profiles_reuse_the_api_without_sharing_application_semantics() {
        let attendance = ApplicationProfile::define("attendance", "1")
            .record_name("attendance row")
            .field(ProfileField::required("present", FieldType::Boolean))
            .build()
            .unwrap();
        let inventory = ApplicationProfile::define("inventory", "1")
            .record_name("inventory row")
            .field(ProfileField::required("quantity", FieldType::Integer))
            .build()
            .unwrap();

        let attendance_result = attendance
            .parse(ApplicationInput::Text("present: yes"), Default::default())
            .unwrap();
        let inventory_result = inventory
            .parse(ApplicationInput::Text("quantity: 12"), Default::default())
            .unwrap();

        assert_eq!(fields(&attendance_result)[0].name, "present");
        assert_eq!(fields(&inventory_result)[0].name, "quantity");
        assert_eq!(
            attendance_result.record_name.as_deref(),
            Some("attendance row")
        );
        assert_eq!(
            inventory_result.record_name.as_deref(),
            Some("inventory row")
        );
    }
}
