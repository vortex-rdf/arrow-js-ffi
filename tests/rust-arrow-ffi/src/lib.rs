mod ffi;

use std::io::Cursor;

use arrow_array::RecordBatch;
use arrow_ipc::reader::FileReader;
use arrow_schema::SchemaRef;
use wasm_bindgen::prelude::*;

pub use crate::ffi::{FFIArrowRecordBatch, FFIArrowTable};

type WasmResult<T> = Result<T, JsError>;

fn read_ipc_file(arrow_file: &[u8]) -> WasmResult<(SchemaRef, Vec<RecordBatch>)> {
    let reader = FileReader::try_new(Cursor::new(arrow_file), None)?;
    let schema = reader.schema();
    let batches = reader.collect::<Result<Vec<_>, _>>()?;
    Ok((schema, batches))
}

#[wasm_bindgen(js_name = arrowIPCToFFI)]
pub fn arrow_ipc_to_ffi(arrow_file: &[u8]) -> WasmResult<FFIArrowTable> {
    let (schema, batches) = read_ipc_file(arrow_file)?;
    Ok(FFIArrowTable::new(&schema, &batches)?)
}

#[wasm_bindgen(js_name = arrowIPCToFFIRecordBatch)]
pub fn arrow_ipc_to_ffi_record_batch(
    arrow_file: &[u8],
    chunk_idx: Option<usize>,
) -> WasmResult<FFIArrowRecordBatch> {
    let (_, mut batches) = read_ipc_file(arrow_file)?;
    let chunk_idx = chunk_idx.unwrap_or(0);
    if chunk_idx >= batches.len() {
        return Err(JsError::new("Index out of range"));
    }
    Ok(FFIArrowRecordBatch::from_batch(batches.swap_remove(chunk_idx))?)
}

/// Every record batch sliced to `length` rows from `offset`, so that the
/// exported arrays carry a non-zero offset into their buffers.
#[wasm_bindgen(js_name = arrowIPCToFFISliced)]
pub fn arrow_ipc_to_ffi_sliced(
    arrow_file: &[u8],
    offset: usize,
    length: usize,
) -> WasmResult<FFIArrowTable> {
    let (schema, batches) = read_ipc_file(arrow_file)?;
    let sliced: Vec<RecordBatch> = batches
        .iter()
        .map(|batch| batch.slice(offset, length))
        .collect();
    Ok(FFIArrowTable::new(&schema, &sliced)?)
}

#[wasm_bindgen(js_name = setPanicHook)]
pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    console_error_panic_hook::set_once();
}

/// Returns a handle to this wasm instance's `WebAssembly.Memory`
#[wasm_bindgen(js_name = wasmMemory)]
pub fn memory() -> JsValue {
    wasm_bindgen::memory()
}

/// Returns a handle to this wasm instance's `WebAssembly.Table` which is the indirect function
/// table used by Rust
#[wasm_bindgen(js_name = _functionTable)]
pub fn function_table() -> JsValue {
    wasm_bindgen::function_table()
}
