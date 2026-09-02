//! Arrow arrays exported as C Data Interface structs in Wasm memory.
use arrow_array::ffi::FFI_ArrowArray;
use arrow_array::{Array, RecordBatch, StructArray};
use arrow_schema::ffi::FFI_ArrowSchema;
use arrow_schema::{ArrowError, DataType, Field, Schema};
use wasm_bindgen::prelude::*;

/// An Arrow table in Wasm memory: one `ArrowSchema` struct per field and one
/// `ArrowArray` struct per column of every record batch.
#[wasm_bindgen]
pub struct FFIArrowTable {
    fields: Vec<Box<FFI_ArrowSchema>>,
    chunks: Vec<Vec<Box<FFI_ArrowArray>>>,
}

impl FFIArrowTable {
    pub fn new(schema: &Schema, batches: &[RecordBatch]) -> Result<Self, ArrowError> {
        let fields = schema
            .fields()
            .iter()
            .map(|field| FFI_ArrowSchema::try_from(field.as_ref()).map(Box::new))
            .collect::<Result<Vec<_>, _>>()?;
        let chunks = batches
            .iter()
            .map(|batch| {
                batch
                    .columns()
                    .iter()
                    .map(|column| Box::new(FFI_ArrowArray::new(&column.to_data())))
                    .collect()
            })
            .collect();
        Ok(Self { fields, chunks })
    }
}

#[wasm_bindgen]
impl FFIArrowTable {
    /// Get the number of Fields in the table schema
    #[wasm_bindgen(js_name = schemaLength)]
    pub fn schema_length(&self) -> usize {
        self.fields.len()
    }

    /// Get the pointer to one ArrowSchema FFI struct
    /// @param i number the index of the field in the schema to use
    #[wasm_bindgen(js_name = schemaAddr)]
    pub fn schema_addr(&self, i: usize) -> *const FFI_ArrowSchema {
        &*self.fields[i] as *const FFI_ArrowSchema
    }

    /// Get the total number of chunks in the table
    #[wasm_bindgen(js_name = chunksLength)]
    pub fn chunks_length(&self) -> usize {
        self.chunks.len()
    }

    /// Get the number of columns in a given chunk
    #[wasm_bindgen(js_name = chunkLength)]
    pub fn chunk_length(&self, i: usize) -> usize {
        self.chunks[i].len()
    }

    /// Get the pointer to one ArrowArray FFI struct for a given chunk index and column index
    /// @param chunk number The chunk index to use
    /// @param column number The column index to use
    /// @returns number pointer to an ArrowArray FFI struct in Wasm memory
    #[wasm_bindgen(js_name = arrayAddr)]
    pub fn array_addr(&self, chunk: usize, column: usize) -> *const FFI_ArrowArray {
        &*self.chunks[chunk][column] as *const FFI_ArrowArray
    }

    #[wasm_bindgen]
    pub fn drop(self) {
        drop(self.fields);
        drop(self.chunks);
    }
}

/// An Arrow record batch in Wasm memory, as one struct array: an
/// `ArrowSchema` for the struct field (carrying the schema metadata) and one
/// `ArrowArray`.
#[wasm_bindgen]
pub struct FFIArrowRecordBatch {
    field: Box<FFI_ArrowSchema>,
    array: Box<FFI_ArrowArray>,
}

impl FFIArrowRecordBatch {
    pub fn from_batch(batch: RecordBatch) -> Result<Self, ArrowError> {
        let schema = batch.schema();
        let field = Field::new("", DataType::Struct(schema.fields().clone()), false)
            .with_metadata(schema.metadata().clone());
        let array = StructArray::from(batch);
        Ok(Self {
            field: Box::new(FFI_ArrowSchema::try_from(&field)?),
            array: Box::new(FFI_ArrowArray::new(&array.to_data())),
        })
    }
}

#[wasm_bindgen]
impl FFIArrowRecordBatch {
    #[wasm_bindgen]
    pub fn array_addr(&self) -> *const FFI_ArrowArray {
        &*self.array as *const FFI_ArrowArray
    }

    #[wasm_bindgen]
    pub fn field_addr(&self) -> *const FFI_ArrowSchema {
        &*self.field as *const FFI_ArrowSchema
    }
}
