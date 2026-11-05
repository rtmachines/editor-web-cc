import { createHash } from 'node:crypto';

// ADR-0128 (ai): the smoke suite rebuilds `content_sha256` from the export document.
// The code follows `api.md` section 6.1 (hashed fields) and section 6.3 (array order).
// It uses the RFC 8785 (JCS) canonical form and the Node module `crypto` only.

/** Any parsed JSON value. */
export type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
/** A parsed JSON object. */
export type JsonObject = { [key: string]: Json };

const TYPE_FIELDS = [
  'id',
  'name',
  'revision',
  'kind',
  'string_length',
  'element_type_id',
  'dimensions',
  'members',
] as const;
const PARAMETER_FIELDS = [
  'unit_path',
  'tag',
  'key',
  'type_id',
  's7_address',
  'limits',
  'value',
] as const;
const LIMIT_FIELDS = ['leaf_path', 'min', 'max', 'max_length'] as const;

/** Return the value as an object. A document that has another shape is a test error. */
function asObject(value: Json | undefined, where: string): JsonObject {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error(`The export document needs an object at ${where}.`);
  }
  return value;
}

/** Return the value as an array. */
function asArray(value: Json | undefined, where: string): Json[] {
  if (!Array.isArray(value)) {
    throw new Error(`The export document needs an array at ${where}.`);
  }
  return value;
}

/** Return the text of a member. */
function textOf(object: JsonObject, key: string): string {
  const value = object[key];
  if (typeof value !== 'string') {
    throw new Error(`The export document needs a string at ${key}.`);
  }
  return value;
}

/** Copy the listed members of an object. A member that is absent stays absent. */
function pick(object: JsonObject, fields: readonly string[]): JsonObject {
  const copy: JsonObject = {};
  for (const field of fields) {
    if (Object.hasOwn(object, field)) {
      copy[field] = object[field] as Json;
    }
  }
  return copy;
}

/** Compare two strings byte by byte in UTF-8, as api.md section 6.3 says. */
function compareBytes(a: string, b: string): number {
  return Buffer.compare(Buffer.from(a, 'utf8'), Buffer.from(b, 'utf8'));
}

/** Return the segments of a unit path as integers, the first one without its `U`. */
function unitPathSegments(path: string): bigint[] {
  const segments = path.replace(/^U/, '').split('.');
  return segments.map((segment) => {
    if (!/^\d+$/.test(segment)) {
      throw new Error(`The unit path ${path} has a segment that is not an integer.`);
    }
    return BigInt(segment);
  });
}

/** Compare two unit paths segment by segment. A path comes before a longer path with the same start. */
function compareUnitPaths(a: string, b: string): number {
  const left = unitPathSegments(a);
  const right = unitPathSegments(b);
  const common = Math.min(left.length, right.length);
  for (let index = 0; index < common; index += 1) {
    const l = left[index] as bigint;
    const r = right[index] as bigint;
    if (l !== r) {
      return l < r ? -1 : 1;
    }
  }
  return left.length - right.length;
}

/** Return a sorted copy of an array. */
function sorted(items: JsonObject[], compare: (a: JsonObject, b: JsonObject) => number): JsonObject[] {
  return [...items].sort(compare);
}

function typeEntry(raw: Json): JsonObject {
  return pick(asObject(raw, 'types[]'), TYPE_FIELDS);
}

function parameterEntry(raw: Json): JsonObject {
  const entry = pick(asObject(raw, 'parameters[]'), PARAMETER_FIELDS);
  const limits = asArray(entry.limits, 'parameters[].limits').map((limit) =>
    pick(asObject(limit, 'parameters[].limits[]'), LIMIT_FIELDS),
  );
  entry.limits = sorted(limits, (a, b) => compareBytes(textOf(a, 'leaf_path'), textOf(b, 'leaf_path')));
  return entry;
}

/**
 * Build the hashed body of an export document (api.md section 6.1).
 *
 * The function copies the hashed fields only. A name, a status, and an export field stay out.
 * It sorts `types[]`, `parameters[]`, and `limits[]` by section 6.3. It keeps the order of `members[]`.
 */
export function hashedBodyOf(doc: JsonObject): JsonObject {
  const machine = asObject(doc.machine, 'machine');
  const recipe = asObject(doc.recipe, 'recipe');
  const version = asObject(doc.version, 'version');
  const types = asArray(doc.types, 'types').map(typeEntry);
  const parameters = asArray(doc.parameters, 'parameters').map(parameterEntry);
  return {
    machine: pick(machine, ['code', 'plc_platform']),
    recipe: pick(recipe, ['reference_number']),
    version: pick(version, ['number']),
    types: sorted(types, (a, b) => compareBytes(textOf(a, 'id'), textOf(b, 'id'))),
    parameters: sorted(
      parameters,
      (a, b) =>
        compareUnitPaths(textOf(a, 'unit_path'), textOf(b, 'unit_path')) ||
        compareBytes(textOf(a, 'tag'), textOf(b, 'tag')),
    ),
  };
}

/**
 * Return the RFC 8785 (JCS) canonical form of a JSON value.
 *
 * Object keys sort by UTF-16 code unit, which is the default order of JavaScript.
 * `JSON.stringify` writes strings and numbers as RFC 8785 requires. A number that is not finite is an error.
 */
export function canonicalText(value: Json): string {
  if (value === null || typeof value === 'boolean' || typeof value === 'string') {
    return JSON.stringify(value);
  }
  if (typeof value === 'number') {
    if (!Number.isFinite(value)) {
      throw new Error('JCS has no form for a number that is not finite.');
    }
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    return `[${value.map(canonicalText).join(',')}]`;
  }
  const members = Object.keys(value)
    .sort()
    .map((key) => `${JSON.stringify(key)}:${canonicalText(value[key] as Json)}`);
  return `{${members.join(',')}}`;
}

/** Return the lower-case hex SHA-256 of the UTF-8 bytes of a text. */
export function sha256Hex(text: string): string {
  return createHash('sha256').update(text, 'utf8').digest('hex');
}

/** Return the `content_sha256` that a consumer computes from an export document. */
export function contentSha256(doc: JsonObject): string {
  return sha256Hex(canonicalText(hashedBodyOf(doc)));
}
