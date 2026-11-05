import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { canonicalText, contentSha256, hashedBodyOf } from '../compose/support/jcs';

// ADR-0128 (ai): the smoke suite builds the hashed body of an export document by itself.
// These tests need no server. They use the golden file of `recipe-export` as the reference.

const GOLDEN_FILE = path.resolve(
  __dirname,
  '../../crates/recipe-export/tests/golden/recipe_R-100_v3.json',
);
/** The hash that the golden file and the Rust golden test state. */
const GOLDEN_HASH = '885d9a7d767af484f1c1824b5d493b9ddc19604db05eff1fa4fbded25bad7497';

/** The SHA-256 of the UTF-8 bytes of the canonical text of the Unicode test. Python hashlib gave it. */
const EXPECTED_TEXT_HASH = '75a8ba3a4ed2f4c73c9180083624a7fa10316c75ef24eaa18c7c7be92fed43f6';

type Json = Record<string, any>;

function loadGolden(): Json {
  return JSON.parse(readFileSync(GOLDEN_FILE, 'utf8'));
}

test.describe('JCS hash of the export document', () => {
  test('rebuilds the content hash of the golden file', () => {
    const doc = loadGolden();
    expect(doc.content_sha256).toBe(GOLDEN_HASH);
    expect(contentSha256(doc)).toBe(GOLDEN_HASH);
  });

  test('keeps only the fields that api.md 6.1 marks as hashed', () => {
    const body = hashedBodyOf(loadGolden());
    expect(Object.keys(body).sort()).toEqual(['machine', 'parameters', 'recipe', 'types', 'version']);
    expect(body.machine).toEqual({ code: 'M12', plc_platform: 'S7_300' });
    expect(body.recipe).toEqual({ reference_number: 'R-100' });
    expect(body.version).toEqual({ number: 3 });
    expect(body.parameters[0]).not.toHaveProperty('name_en');
  });

  test('gives the same hash when the arrays arrive in another order', () => {
    const doc = loadGolden();
    doc.types.reverse();
    doc.parameters.reverse();
    for (const parameter of doc.parameters) {
      parameter.limits.reverse();
    }
    expect(contentSha256(doc)).toBe(GOLDEN_HASH);
  });

  test('sorts a unit path by integer segments and not by text', () => {
    const doc = loadGolden();
    const [first] = doc.parameters;
    doc.parameters = ['U10', 'U2', 'U2.1', 'U1.10', 'U1.9'].map((unitPath) => ({
      ...first,
      unit_path: unitPath,
    }));
    const order = hashedBodyOf(doc).parameters.map((parameter: Json) => parameter.unit_path);
    expect(order).toEqual(['U1.9', 'U1.10', 'U2', 'U2.1', 'U10']);
  });

  test('hashes Polish letters, a character outside the BMP, and UTF-16 key order', () => {
    // U+FF5E is one UTF-16 unit above U+D83D. U+1F600 is the pair D83D DE00. JCS sorts by UTF-16 unit.
    const doc = loadGolden();
    doc.types = [];
    doc.parameters = [
      {
        unit_path: 'U1',
        tag: 'TEXT',
        key: '1f3a6b2c-8d9e-4f00-a1b2-c3d4e5f60718',
        name_en: 'Not hashed',
        type_id: 'STRING',
        s7_address: null,
        limits: [],
        value: { '～': 'b', '\u{1F600}': 'a', z: 'Zażółć gęślą jaźń \u{1F600}' },
      },
    ];
    const expectedText =
      '{"machine":{"code":"M12","plc_platform":"S7_300"},' +
      '"parameters":[{"key":"1f3a6b2c-8d9e-4f00-a1b2-c3d4e5f60718","limits":[],"s7_address":null,' +
      '"tag":"TEXT","type_id":"STRING","unit_path":"U1",' +
      '"value":{"z":"Zażółć gęślą jaźń \u{1F600}","\u{1F600}":"a","～":"b"}}],' +
      '"recipe":{"reference_number":"R-100"},"types":[],"version":{"number":3}}';
    expect(canonicalText(hashedBodyOf(doc))).toBe(expectedText);
    expect(contentSha256(doc)).toBe(EXPECTED_TEXT_HASH);
  });

  test('gives another hash when one value changes', () => {
    const doc = loadGolden();
    doc.parameters[0].value = '121';
    const changed = contentSha256(doc);
    expect(changed).toMatch(/^[0-9a-f]{64}$/);
    expect(changed).not.toBe(GOLDEN_HASH);
  });
});
