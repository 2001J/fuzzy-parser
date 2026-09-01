# Application Profiles

An application profile is the reusable vocabulary an application gives Fuzzy
Parser. It describes what fields the application understands; it does not
contain customer records or business-side effects.

## Who defines the profile?

The application developer or administrator defines it once. People importing
files or pasting lists should not write schemas during each import.

For example, one contact profile may accept a name and phone while optionally
recognizing an amount or notes. Inputs without an amount remain valid because
the field is optional.

## Node.js profile

```js
import { defineProfile, parseProfile } from "@fuzzy-parser/node";

const contacts = await defineProfile({
  name: "contacts",
  version: "1",
  recordName: "contact",
  fields: [
    {
      name: "name",
      fieldType: "person_name",
      required: true,
      aliases: ["Name", "Full name"],
    },
    {
      name: "phone",
      fieldType: "phone_number",
      aliases: ["Phone", "Mobile", "Telephone"],
    },
    {
      name: "amount",
      fieldType: "currency",
      aliases: ["Amount", "Pledge", "Total"],
    },
    { name: "notes", fieldType: "text" },
  ],
});

const result = await parseProfile(contacts, {
  format: "csv",
  bytes: uploadedBytes,
  filename: "contacts.csv",
});
```

`defineProfile` validates supported capabilities before an upload is accepted.
`parseProfile` supports `text`, `txt`, `csv`, and `xlsx` input.

## Rust profile

```rust
use parser_api::{ApplicationInput, ApplicationProfile, ProfileField};
use parser_schema::FieldType;

let contacts = ApplicationProfile::define("contacts", "1")
    .record_name("contact")
    .field(
        ProfileField::required("name", FieldType::PersonName)
            .aliases(["Name", "Full name"]),
    )
    .field(
        ProfileField::optional("phone", FieldType::PhoneNumber)
            .aliases(["Phone", "Mobile", "Telephone"]),
    )
    .field(
        ProfileField::optional("amount", FieldType::Currency)
            .aliases(["Amount", "Pledge", "Total"]),
    )
    .field(ProfileField::optional("notes", FieldType::Text))
    .build()?;

let result = contacts.parse(
    ApplicationInput::Csv {
        bytes: uploaded_csv,
        file_name: Some("contacts.csv"),
    },
    Default::default(),
)?;
```

## Field configuration

Each field has:

- `name`: the stable application-facing identifier.
- `fieldType`: the generic parser capability.
- `required`: whether absence produces a review warning.
- `multiple`: whether more than one value may be assigned.
- `aliases`: labels or headers the caller recognizes.
- `constraints`: supported integer or string-length bounds.

Enum fields additionally provide their canonical values and aliases. Aliases
remain scoped to their field; ambiguous ownership stays unresolved.

The current field support matrix is maintained in [Current state](current-state.md#field-capabilities).

## Versioning a profile

The profile version belongs to the application. It is separate from the parser
package version and JSON contract versions.

Create a new profile version when changing:

- field meaning;
- requiredness or multiplicity;
- aliases;
- constraints;
- enum values;
- text-composition options.

Retain old versions when historical imports may need to be replayed. A filename,
profile name, or version never changes detector behavior by itself.

## What stays in the application

Profiles do not define authorization, database uniqueness, qualification,
approval, persistence, messaging, or application-specific duplicate rules.
Those rules run after review and explicit confirmation in the consuming
application.
