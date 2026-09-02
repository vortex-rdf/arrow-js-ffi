import { readFileSync } from "fs";
import { describe, it, expect } from "vitest";
import * as arrow from "apache-arrow";
import * as wasm from "rust-arrow-ffi";
import {
  arrowTableToFFI,
  arraysEqual,
  loadIPCTableFromDisk,
  validityEqual,
} from "./utils";
import { parseField, parseVector } from "../src";
import { Type } from "../src/types";

wasm.setPanicHook();

const WASM_MEMORY = wasm.wasmMemory();

const TEST_TABLE = loadIPCTableFromDisk("tests/table.arrow");
const FFI_TABLE = arrowTableToFFI(TEST_TABLE);

/** The field and array pointers of a column, with the field checked. */
function column(name: string, ffiTable: wasm.FFIArrowTable = FFI_TABLE) {
  const columnIndex = TEST_TABLE.schema.fields.findIndex(
    (field) => field.name == name,
  );
  const originalField = TEST_TABLE.schema.fields[columnIndex];
  const originalVector = TEST_TABLE.getChildAt(columnIndex)!;
  const field = parseField(WASM_MEMORY.buffer, ffiTable.schemaAddr(columnIndex));

  expect(field.name).toStrictEqual(originalField.name);
  expect(field.typeId).toStrictEqual(originalField.typeId);
  expect(field.nullable).toStrictEqual(originalField.nullable);

  const arrayPtr = ffiTable.arrayAddr(0, columnIndex);
  return { originalVector, field, arrayPtr };
}

/** Every buffer of a parsed view array: the views, then the variadic data. */
function viewBuffers(vector: arrow.Vector): Uint8Array[] {
  const data = vector.data[0];
  return [data.values as Uint8Array, ...data.variadicBuffers];
}

describe("string_view", (t) => {
  function test(copy: boolean) {
    const { originalVector, field, arrayPtr } = column("string_view");
    expect(field.typeId).toStrictEqual(Type.Utf8View);

    const wasmVector = parseVector(
      WASM_MEMORY.buffer,
      arrayPtr,
      field.type,
      copy,
    );

    expect(wasmVector.length).toStrictEqual(originalVector.length);
    expect(arraysEqual([...originalVector], [...wasmVector])).toBeTruthy();

    // The fixture keeps two data buffers, so views address more than one.
    expect(wasmVector.data[0].variadicBuffers.length).toStrictEqual(2);

    // copy=false views Wasm memory; copy=true owns its bytes.
    for (const buffer of viewBuffers(wasmVector)) {
      expect(buffer.buffer === WASM_MEMORY.buffer).toStrictEqual(!copy);
    }
  }

  it("copy=false", () => test(false));
  it("copy=true", () => test(true));
});

describe("string_view (with nulls)", (t) => {
  function test(copy: boolean) {
    const { originalVector, field, arrayPtr } = column("string_view_null");

    const wasmVector = parseVector(
      WASM_MEMORY.buffer,
      arrayPtr,
      field.type,
      copy,
    );

    expect(wasmVector.nullCount).toStrictEqual(originalVector.nullCount);
    expect(validityEqual(originalVector, wasmVector)).toBeTruthy();
    expect(wasmVector.get(1)).toBeNull();
    expect(arraysEqual([...originalVector], [...wasmVector])).toBeTruthy();
  }

  it("copy=false", () => test(false));
  it("copy=true", () => test(true));
});

describe("string_view with an offset", (t) => {
  // Exported from a sliced record batch, the array's buffers start before
  // its first element: the views are read from the offset, validity is
  // addressed by `offset + index`.
  const tableBuffer = readFileSync("tests/table.arrow");
  const OFFSET = 1;
  const LENGTH = 2;

  function test(copy: boolean) {
    const ffiTable = wasm.arrowIPCToFFISliced(tableBuffer, OFFSET, LENGTH);
    for (const name of ["string_view", "string_view_null"]) {
      const { originalVector, field, arrayPtr } = column(name, ffiTable);
      const wasmVector = parseVector(
        WASM_MEMORY.buffer,
        arrayPtr,
        field.type,
        copy,
      );
      const expected = originalVector.slice(OFFSET, OFFSET + LENGTH);

      expect(wasmVector.length).toStrictEqual(LENGTH);
      expect(wasmVector.nullCount).toStrictEqual(expected.nullCount);
      expect(arraysEqual([...expected], [...wasmVector])).toBeTruthy();
    }
    ffiTable.drop();
  }

  it("copy=false", () => test(false));
  it("copy=true", () => test(true));
});

describe("binary_view", (t) => {
  function test(copy: boolean) {
    const { originalVector, field, arrayPtr } = column("binary_view");
    expect(field.typeId).toStrictEqual(Type.BinaryView);

    const wasmVector = parseVector(
      WASM_MEMORY.buffer,
      arrayPtr,
      field.type,
      copy,
    );

    expect(wasmVector.length).toStrictEqual(originalVector.length);
    for (let i = 0; i < originalVector.length; i++) {
      expect(
        arraysEqual(originalVector.get(i) as Uint8Array, wasmVector.get(i)),
      ).toBeTruthy();
    }
    for (const buffer of viewBuffers(wasmVector)) {
      expect(buffer.buffer === WASM_MEMORY.buffer).toStrictEqual(!copy);
    }
  }

  it("copy=false", () => test(false));
  it("copy=true", () => test(true));
});

describe("dictionary encoded string_view", (t) => {
  function test(copy: boolean) {
    const { originalVector, field, arrayPtr } = column(
      "dictionary_encoded_string_view",
    );
    expect(field.type.dictionary.typeId).toStrictEqual(Type.Utf8View);

    const wasmVector = parseVector(
      WASM_MEMORY.buffer,
      arrayPtr,
      field.type,
      copy,
    );

    for (let i = 0; i < originalVector.length; i++) {
      expect(originalVector.get(i)).toStrictEqual(wasmVector.get(i));
    }
  }

  it("copy=false", () => test(false));
  it("copy=true", () => test(true));
});
