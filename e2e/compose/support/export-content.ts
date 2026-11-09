import type { Json } from './jcs';

// Design D6 of test-gates-round-2: the smoke suite checks the content of an export document.
// The hash check proves the integrity of the body. This check proves that the body holds the
// values that the test entered. The document shape follows `api.md` section 6.

/** The values that the test entered. Strings hold the canonical decimal text of the export. */
export interface ExpectedExportContent {
  readonly machineCode: string;
  readonly referenceNumber: string;
  readonly versionNumber: number;
  readonly unitPath: string;
  readonly tag: string;
  readonly type: string;
  readonly value: string;
  readonly min: string;
  readonly max: string;
}

/** The leaf path of a limit on a scalar parameter. */
const SCALAR_LEAF_PATH = '';

/** Return the member of an object, or `undefined` when the value is not an object. */
function member(value: Json | undefined, key: string): Json | undefined {
  if (value === null || value === undefined || typeof value !== 'object' || Array.isArray(value)) {
    return undefined;
  }
  return value[key];
}

/** Return the fault text for a field that differs from its expected value, or nothing. */
function compare(field: string, actual: Json | undefined, expected: Json): string[] {
  if (actual === expected) {
    return [];
  }
  return [`${field}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual ?? null)}.`];
}

/** Compare the one limit of the parameter. */
function limitFaults(parameter: Json, expected: ExpectedExportContent): string[] {
  const limits = member(parameter, 'limits');
  if (!Array.isArray(limits) || limits.length !== 1) {
    const count = Array.isArray(limits) ? limits.length : 'no list of';
    return [`limits: expected exactly 1 limit for ${expected.tag}, got ${count}.`];
  }
  const limit = limits[0];
  return [
    ...compare('leaf_path', member(limit, 'leaf_path'), SCALAR_LEAF_PATH),
    ...compare('min', member(limit, 'min'), expected.min),
    ...compare('max', member(limit, 'max'), expected.max),
    ...compare('max_length', member(limit, 'max_length'), null),
  ];
}

/** Compare the one parameter with the expected tag. */
function parameterFaults(parameters: Json | undefined, expected: ExpectedExportContent): string[] {
  const matches = Array.isArray(parameters)
    ? parameters.filter((parameter) => member(parameter, 'tag') === expected.tag)
    : [];
  const [parameter] = matches;
  if (parameter === undefined) {
    return [`parameters: no parameter with the tag ${expected.tag}.`];
  }
  if (matches.length !== 1) {
    return [`parameters: expected 1 parameter with the tag ${expected.tag}, got ${matches.length}.`];
  }
  return [
    ...compare('unit_path', member(parameter, 'unit_path'), expected.unitPath),
    ...compare('type_id', member(parameter, 'type_id'), expected.type),
    ...compare('value', member(parameter, 'value'), expected.value),
    ...limitFaults(parameter, expected),
  ];
}

/**
 * Compare an export document with the values that the test entered.
 * It returns one fault text for each difference. An empty list means a match.
 * Each fault text names the JSON field that differs.
 */
export function exportContentFaults(document: Json, expected: ExpectedExportContent): string[] {
  return [
    ...compare('machine.code', member(member(document, 'machine'), 'code'), expected.machineCode),
    ...compare(
      'recipe.reference_number',
      member(member(document, 'recipe'), 'reference_number'),
      expected.referenceNumber,
    ),
    ...compare('version.number', member(member(document, 'version'), 'number'), expected.versionNumber),
    ...parameterFaults(member(document, 'parameters'), expected),
  ];
}
