import { expect, test } from '@playwright/test';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { exportContentFaults } from '../compose/support/export-content';

// Design D6 of test-gates-round-2: the smoke suite checks the content of an export document.
// These tests need no server. They use the golden file of `recipe-export` as the reference.

const GOLDEN_FILE = path.resolve(
  __dirname,
  '../../crates/recipe-export/tests/golden/recipe_R-100_v3.json',
);

/** The own values of the golden file for the parameter `SPEED`. */
const GOLDEN_EXPECTED = {
  machineCode: 'M12',
  referenceNumber: 'R-100',
  versionNumber: 3,
  unitPath: 'U1',
  tag: 'SPEED',
  type: 'INT',
  value: '120',
  min: '10',
  max: '250',
};

type Json = Record<string, any>;

function loadGolden(): Json {
  return JSON.parse(readFileSync(GOLDEN_FILE, 'utf8'));
}

/** The parameter `SPEED` of a loaded copy of the golden file. */
function speedOf(doc: Json): Json {
  return doc.parameters.find((parameter: Json) => parameter.tag === 'SPEED');
}

test.describe('Content check of the export document', () => {
  test('gives no fault for the own values of the golden file', () => {
    expect(exportContentFaults(loadGolden(), GOLDEN_EXPECTED)).toEqual([]);
  });

  const changes: ReadonlyArray<{ field: string; change: (doc: Json) => void }> = [
    { field: 'machine.code', change: (doc) => (doc.machine.code = 'M99') },
    { field: 'recipe.reference_number', change: (doc) => (doc.recipe.reference_number = 'R-999') },
    { field: 'version.number', change: (doc) => (doc.version.number = 4) },
    { field: 'unit_path', change: (doc) => (speedOf(doc).unit_path = 'U2') },
    { field: 'type_id', change: (doc) => (speedOf(doc).type_id = 'REAL') },
    { field: 'value', change: (doc) => (speedOf(doc).value = '121') },
    { field: 'leaf_path', change: (doc) => (speedOf(doc).limits[0].leaf_path = '.x') },
    { field: 'min', change: (doc) => (speedOf(doc).limits[0].min = '11') },
    { field: 'max', change: (doc) => (speedOf(doc).limits[0].max = '251') },
    { field: 'max_length', change: (doc) => (speedOf(doc).limits[0].max_length = '16') },
  ];

  for (const { field, change } of changes) {
    test(`gives a fault that names ${field} when only ${field} changes`, () => {
      const doc = loadGolden();
      change(doc);
      const faults = exportContentFaults(doc, GOLDEN_EXPECTED);
      expect(faults.length).toBeGreaterThan(0);
      // The word boundary keeps `max` apart from `max_length`.
      expect(faults.some((fault) => new RegExp(`\\b${field}\\b`).test(fault))).toBe(true);
    });
  }

  test('gives a fault that names SPEED when the parameter is missing', () => {
    const doc = loadGolden();
    doc.parameters = doc.parameters.filter((parameter: Json) => parameter.tag !== 'SPEED');
    const faults = exportContentFaults(doc, GOLDEN_EXPECTED);
    expect(faults.length).toBeGreaterThan(0);
    expect(faults.some((fault) => fault.includes('SPEED'))).toBe(true);
  });

  test('gives a fault that names SPEED when two parameters have the tag', () => {
    const doc = loadGolden();
    doc.parameters.push({ ...speedOf(doc), unit_path: 'U3' });
    const faults = exportContentFaults(doc, GOLDEN_EXPECTED);
    expect(faults.length).toBeGreaterThan(0);
    expect(faults.some((fault) => fault.includes('SPEED'))).toBe(true);
  });

  test('gives a fault when the parameter has two limits', () => {
    const doc = loadGolden();
    const limit = speedOf(doc).limits[0];
    speedOf(doc).limits.push({ ...limit, leaf_path: '.extra' });
    const faults = exportContentFaults(doc, GOLDEN_EXPECTED);
    expect(faults.some((fault) => fault.includes('limits'))).toBe(true);
  });
});
