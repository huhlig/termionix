//
// Copyright 2017-2025 Hans W. Uhlig. All Rights Reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

//! MudServerData Option
//!
//! https://tools.ietf.org/html/rfc8549#section-3.1.1
//!
//! MSDP is a subnegotiation of the MSDP option.
//!
//! The MSDP subnegotiation is used to send information about the Mud to the
//! client. The information is sent in a series of key-value pairs.
//!
//! The key is a string, and the value is a string, an array of strings, or a

use crate::{consts, result::CodecResult};
use bytes::{Buf, BufMut};
use std::collections::HashMap;

///
/// `MudServerData` contains data about the Mud.
///
#[derive(Clone, Debug)]
pub struct MudServerData(MudServerDataTable);

impl MudServerData {
    /// Creates a new instance of `MudServerData`.
    ///
    /// This function initializes an empty `MudServerData` structure, which internally
    /// contains a `HashMap`. The `HashMap` can later be used to store and manage data
    /// related to the server.
    ///
    /// # Returns
    /// A new `MudServerData` instance with an empty `HashMap`.
    pub fn new() -> MudServerData {
        MudServerData(MudServerDataTable::default())
    }
    ///
    pub fn set(&mut self, key: &str, value: MudServerDataValue) {
        self.0.set(key, value);
    }
    ///
    pub fn get(&self, key: &str) -> Option<&MudServerDataValue> {
        self.0.get(key)
    }
    ///
    /// Encode `MudServerDataValue` to `BufMut`
    ///
    pub fn encode<T: BufMut>(&self, dst: &mut T) {
        self.0.encode(dst);
    }
    ///
    /// Get Encoded Length of `MudServerDataValue`
    ///
    pub fn encoded_len(&self) -> usize {
        self.0.encoded_len()
    }
    /// Decodes the data from the provided buffer.
    ///
    /// # Type Parameters
    /// - `T`: A type that implements the `BufMut` trait. This trait allows the buffer
    ///   to provide mutable access to its underlying data.
    ///
    /// # Arguments
    /// - `src`: A mutable reference to a buffer implementing the `BufMut` trait.
    ///   This is the source buffer containing the data to be decoded.
    ///
    /// # Behavior
    /// This function performs a decoding operation on the provided buffer.
    /// The exact behavior and decoding logic should be implemented within the body of this function.
    ///
    pub fn decode<T: Buf>(src: &mut T) -> CodecResult<MudServerData> {
        Ok(MudServerData(MudServerDataTable::decode(src)?))
    }
}

///
/// `MudServerDataValue`
///
#[derive(Clone, Debug)]
pub enum MudServerDataValue {
    /// String Value
    String(String),
    /// Array Value
    Array(MudServerDataArray),
    /// Table Value
    Table(MudServerDataTable),
}

impl MudServerDataValue {
    ///
    /// Create a new String Value
    ///
    pub fn string(string: &str) -> MudServerDataValue {
        MudServerDataValue::String(string.to_string())
    }
    ///
    /// Create a new Array Value
    ///
    pub fn array(array: MudServerDataArray) -> MudServerDataValue {
        MudServerDataValue::Array(array)
    }
    ///
    /// Create a new Table Value
    ///
    pub fn table(table: MudServerDataTable) -> MudServerDataValue {
        MudServerDataValue::Table(table)
    }
    ///
    /// Encode `MudServerDataValue` to `BufMut`
    ///
    pub fn encode<T: BufMut>(&self, dst: &mut T) {
        match self {
            MudServerDataValue::String(string) => {
                dst.put(string.as_bytes());
            }
            MudServerDataValue::Array(array) => {
                array.encode(dst);
            }
            MudServerDataValue::Table(table) => {
                table.encode(dst);
            }
        }
    }
    ///
    /// Get Encoded Length of `MudServerDataValue`
    ///
    pub fn encoded_len(&self) -> usize {
        match self {
            MudServerDataValue::String(string) => string.len(),
            MudServerDataValue::Array(array) => array.encoded_len(),
            MudServerDataValue::Table(table) => table.encoded_len(),
        }
    }
    ///
    /// Decode `MudServerDataValue` from `Buf`
    ///
    pub fn decode<T: Buf>(src: &mut T) -> CodecResult<MudServerDataValue> {
        if !src.has_remaining() {
            return Ok(MudServerDataValue::String(String::new()));
        }

        let first_byte = src.chunk()[0];

        match first_byte {
            consts::option::msdp::ARRAY_OPEN => {
                Ok(MudServerDataValue::Array(MudServerDataArray::decode(src)?))
            }
            consts::option::msdp::TABLE_OPEN => {
                Ok(MudServerDataValue::Table(MudServerDataTable::decode(src)?))
            }
            _ => {
                // Read string until we hit a control byte
                let mut string_bytes = Vec::new();
                while src.has_remaining() {
                    let byte = src.chunk()[0];
                    if byte == consts::option::msdp::VAR
                        || byte == consts::option::msdp::VAL
                        || byte == consts::option::msdp::ARRAY_CLOSE
                        || byte == consts::option::msdp::TABLE_CLOSE
                    {
                        break;
                    }
                    string_bytes.push(src.get_u8());
                }
                Ok(MudServerDataValue::String(
                    String::from_utf8_lossy(&string_bytes).to_string(),
                ))
            }
        }
    }
}

///
/// `MudServerDataArray` is an array of values.
///
#[derive(Clone, Debug, Default)]
pub struct MudServerDataArray(Vec<MudServerDataValue>);

impl MudServerDataArray {
    ///
    /// Creates a new instance of `MudServerDataArray`.
    ///
    pub fn new() -> MudServerDataArray {
        MudServerDataArray(Vec::new())
    }
    ///
    /// Adds a value to the array.
    ///
    pub fn push(&mut self, value: MudServerDataValue) {
        self.0.push(value);
    }
    ///
    /// Returns the number of values in the array.
    ///
    pub fn len(&self) -> usize {
        self.0.len()
    }
    ///
    /// Returns a reference to the value at the specified index.
    ///
    pub fn get(&self, index: usize) -> Option<&MudServerDataValue> {
        self.0.get(index)
    }
    ///
    /// Encodes the array into a buffer.
    ///
    pub fn encode<T: BufMut>(&self, dst: &mut T) {
        dst.put_u8(consts::option::msdp::ARRAY_OPEN);
        for value in &self.0 {
            dst.put_u8(consts::option::msdp::VAL);
            value.encode(dst);
        }
        dst.put_u8(consts::option::msdp::ARRAY_CLOSE);
    }
    ///
    /// Get Encoded Length of `MudServerDataValue`
    ///
    pub fn encoded_len(&self) -> usize {
        let mut length = 0;
        length += 1; // ARRAY_OPEN
        for value in &self.0 {
            length += 1; // VAL
            length += value.encoded_len();
        }
        length += 1; // ARRAY_CLOSE
        length
    }
    ///
    /// Decode `MudServerDataArray` from `Buf`
    ///
    pub fn decode<T: Buf>(src: &mut T) -> CodecResult<MudServerDataArray> {
        let mut array = MudServerDataArray::new();

        // Consume ARRAY_OPEN
        if src.has_remaining() && src.chunk()[0] == consts::option::msdp::ARRAY_OPEN {
            src.advance(1);
        }

        while src.has_remaining() {
            let byte = src.chunk()[0];

            if byte == consts::option::msdp::ARRAY_CLOSE {
                src.advance(1);
                break;
            } else if byte == consts::option::msdp::VAL {
                src.advance(1);
                array.push(MudServerDataValue::decode(src)?);
            } else {
                // Unexpected byte, skip it
                src.advance(1);
            }
        }

        Ok(array)
    }
}

/// `MudServerDataTable` is a table of key-value pairs.
#[derive(Clone, Debug, Default)]
pub struct MudServerDataTable(HashMap<String, MudServerDataValue>);

impl MudServerDataTable {
    ///
    /// Creates a new instance of `MudServerDataTable`.
    ///
    pub fn new() -> MudServerDataTable {
        MudServerDataTable(HashMap::new())
    }
    ///
    /// Sets the value associated with the given key.
    ///
    pub fn set(&mut self, key: &str, value: MudServerDataValue) {
        self.0.insert(key.to_string(), value);
    }
    ///
    /// Returns a reference to the value associated with the given key.
    ///
    pub fn get(&self, key: &str) -> Option<&MudServerDataValue> {
        self.0.get(key)
    }
    ///
    /// Returns the number of key-value pairs in the table.
    ///
    pub fn len(&self) -> usize {
        self.0.len()
    }
    ///
    /// Encodes the array into a buffer.
    ///
    pub fn encode<T: BufMut>(&self, dst: &mut T) {
        dst.put_u8(consts::option::msdp::TABLE_OPEN);
        for (key, value) in &self.0 {
            dst.put_u8(consts::option::msdp::VAR);
            dst.put(key.as_bytes());
            dst.put_u8(consts::option::msdp::VAL);
            value.encode(dst);
        }
        dst.put_u8(consts::option::msdp::TABLE_CLOSE);
    }
    ///
    /// Get Encoded Length of `MudServerDataTable`.
    ///
    pub fn encoded_len(&self) -> usize {
        let mut length = 0;
        length += 1; // TABLE_OPEN
        for (key, value) in &self.0 {
            length += 1; // VAR
            length += key.len();
            length += 1; // VAL
            length += value.encoded_len();
        }
        length += 1; // TABLE_CLOSE
        length
    }
    ///
    /// Decode `MudServerDataTable` from `Buf`
    ///
    pub fn decode<T: Buf>(src: &mut T) -> CodecResult<MudServerDataTable> {
        let mut table = MudServerDataTable::new();

        // Check if this is a nested table (starts with TABLE_OPEN)
        let has_table_markers =
            src.has_remaining() && src.chunk()[0] == consts::option::msdp::TABLE_OPEN;

        if has_table_markers {
            src.advance(1); // Consume TABLE_OPEN
        }

        while src.has_remaining() {
            let byte = src.chunk()[0];

            if byte == consts::option::msdp::TABLE_CLOSE {
                if has_table_markers {
                    src.advance(1);
                }
                break;
            } else if byte == consts::option::msdp::VAR {
                src.advance(1);

                // Read the key
                let mut key_bytes = Vec::new();
                while src.has_remaining() {
                    let byte = src.chunk()[0];
                    if byte == consts::option::msdp::VAL {
                        break;
                    }
                    key_bytes.push(src.get_u8());
                }
                let key = String::from_utf8_lossy(&key_bytes).to_string();

                // Expect VAL marker
                if src.has_remaining() && src.chunk()[0] == consts::option::msdp::VAL {
                    src.advance(1);
                    table.set(&key, MudServerDataValue::decode(src)?);
                }
            } else {
                // Unexpected byte or we've reached the end
                break;
            }
        }

        Ok(table)
    }
}
