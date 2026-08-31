import { parse } from '@fuzzy-parser/node';

export const runtime = 'nodejs';
export const dynamic = 'force-dynamic';

const schema = {
  schema_version: '0.1',
  record_name: 'inventory_item',
  fields: [
    {
      name: 'count',
      field_type: 'integer',
      required: true,
      multiple: false,
      aliases: [],
      constraints: [],
    },
  ],
  options: { allow_unknown_fields: true },
};

export async function GET() {
  const response = await parse({
    input: { format: 'text', bytes: new TextEncoder().encode('42') },
    schema,
  });
  return Response.json({
    contractVersion: response.contract_version,
    parserVersion: response.parser_version,
    recordName: response.record_name,
    raw: response.source_evidence.document.blocks[0].value.value,
  });
}
