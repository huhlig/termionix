//
// Copyright 2017-2026 Hans W. Uhlig. All Rights Reserved.
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
use byteorder::WriteBytesExt;
use bytes::{Buf, BufMut};
use std::collections::HashMap;

/// `MudServerData` is the main container for MUD server information.
///
/// It manages a collection of key-value pairs containing metadata about the MUD server.
/// The data is stored internally as a `MudServerDataTable` and can be encoded/decoded
/// according to the MSDP protocol (RFC 8549 Section 3.1.1).
///
/// # Examples
///
/// ```ignore
/// let mut msd = MudServerData::new();
/// msd.set("name", MudServerDataValue::string("My MUD"));
/// ```
#[derive(Clone, Debug)]
pub struct MudServerData(MudServerDataTable);

impl MudServerData {
    /// Creates a new instance of `MudServerData`.
    ///
    /// This function initializes an empty `MudServerData` structure with an empty
    /// internal `HashMap`. Data can be added using the `set` method.
    ///
    /// # Returns
    /// A new `MudServerData` instance with no entries.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let msd = MudServerData::new();
    /// ```
    pub fn new() -> MudServerData {
        MudServerData(MudServerDataTable::default())
    }
    /// Sets a value associated with the given key.
    ///
    /// If the key already exists, its value is replaced.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to associate with the value (as a string slice)
    /// * `value` - The `MudServerDataValue` to store
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut msd = MudServerData::new();
    /// msd.set("version", MudServerDataValue::string("1.0"));
    /// ```
    pub fn set(&mut self, key: &str, value: MudServerDataValue) {
        self.0.set(key, value);
    }
    /// Retrieves the value associated with the given key.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up (as a string slice)
    ///
    /// # Returns
    /// `Some(&MudServerDataValue)` if the key exists, otherwise `None`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let msd = MudServerData::new();
    /// if let Some(value) = msd.get("version") {
    ///     println!("{}", value);
    /// }
    /// ```
    pub fn get(&self, key: &str) -> Option<&MudServerDataValue> {
        self.0.get(key)
    }
    /// Retrieves the value associated with the given key.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up (as a string slice)
    ///
    /// # Returns
    /// `Some(&MudServerDataValue)` if the key exists, otherwise `None`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let msd = MudServerData::new();
    /// if let Some(value) = msd.get("version") {
    ///     println!("{}", value);
    /// }
    /// ```
    pub fn get_mut(&mut self, key: &str) -> Option<&mut MudServerDataValue> {
        self.0.get_mut(key)
    }
    /// Gets the encoded length of this `MudServerData` structure.
    ///
    /// Returns the number of bytes that would be written when encoding this
    /// structure according to the MSDP protocol.
    ///
    /// # Returns
    /// The total encoded byte length.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let msd = MudServerData::new();
    /// let encoded_len = msd.len();
    /// ```
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Encodes `MudServerData` into the provided mutable buffer.
    ///
    /// Serializes the structure according to the MSDP protocol and writes it
    /// to the destination buffer.
    ///
    /// # Type Parameters
    ///
    /// * `T` - A type implementing `BufMut` for buffer operations
    ///
    /// # Arguments
    ///
    /// * `dst` - A mutable reference to the destination buffer
    ///
    /// # Returns
    ///
    /// * `Ok(usize)` - The number of bytes written
    /// * `Err(CodecResult)` - If encoding fails
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut buffer = Vec::new();
    /// let msd = MudServerData::new();
    /// match msd.encode(&mut buffer) {
    ///     Ok(bytes_written) => println!("Encoded {} bytes", bytes_written),
    ///     Err(e) => eprintln!("Encoding error: {}", e),
    /// }
    /// ```
    pub fn encode<T: BufMut>(&self, dst: &mut T) -> CodecResult<usize> {
        Ok(self.write(&mut dst.writer())?)
    }

    /// Writes `MudServerData` to the provided writer.
    ///
    /// Low-level method for writing the encoded data to a generic writer.
    ///
    /// # Arguments
    ///
    /// * `writer` - A mutable reference to a type implementing `std::io::Write`
    ///
    /// # Returns
    ///
    /// * `Ok(usize)` - The number of bytes written
    /// * `Err(std::io::Error)` - If writing fails
    pub fn write<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<usize> {
        self.0.write(writer)
    }

    /// Decodes `MudServerData` from the provided buffer.
    ///
    /// Deserializes data according to the MSDP protocol from the source buffer.
    ///
    /// # Type Parameters
    ///
    /// * `T` - A type implementing `Buf` for buffer operations
    ///
    /// # Arguments
    ///
    /// * `src` - A mutable reference to the source buffer
    ///
    /// # Returns
    ///
    /// * `Ok(MudServerData)` - The decoded structure
    /// * `Err(CodecResult)` - If decoding fails
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut buffer = vec![/* MSDP encoded data */];
    /// match MudServerData::decode(&mut buffer) {
    ///     Ok(msd) => println!("Decoded: {}", msd),
    ///     Err(e) => eprintln!("Decoding error: {}", e),
    /// }
    /// ```
    pub fn decode<T: Buf>(src: &mut T) -> CodecResult<MudServerData> {
        Ok(MudServerData(MudServerDataTable::decode(src)?))
    }
}

impl std::fmt::Display for MudServerData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MudServerData({})", self.0)
    }
}

/// `MudServerDataValue` represents a value in the MSDP protocol.
///
/// Values can be one of three types: simple strings, arrays of values,
/// or nested tables of key-value pairs. This enum provides a flexible
/// structure for representing various data types in the MSDP format.
///
/// # Variants
///
/// * `String(String)` - A simple string value
/// * `Array(MudServerDataArray)` - An array of MSDP values
/// * `Table(MudServerDataTable)` - A nested table of key-value pairs
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
    /// Creates a new string value.
    ///
    /// # Arguments
    ///
    /// * `string` - The string content (as a string slice)
    ///
    /// # Returns
    /// A new `MudServerDataValue::String` variant.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let value = MudServerDataValue::string("Hello, MUD!");
    /// ```
    pub fn string(string: &str) -> MudServerDataValue {
        MudServerDataValue::String(string.to_string())
    }
    /// Creates a new array value.
    ///
    /// # Arguments
    ///
    /// * `array` - A `MudServerDataArray` containing the array elements
    ///
    /// # Returns
    /// A new `MudServerDataValue::Array` variant.
    pub fn array(array: MudServerDataArray) -> MudServerDataValue {
        MudServerDataValue::Array(array)
    }
    /// Creates a new table value.
    ///
    /// # Arguments
    ///
    /// * `table` - A `MudServerDataTable` containing the key-value pairs
    ///
    /// # Returns
    /// A new `MudServerDataValue::Table` variant.
    pub fn table(table: MudServerDataTable) -> MudServerDataValue {
        MudServerDataValue::Table(table)
    }

    /// Gets the encoded length of this `MudServerDataValue`.
    ///
    /// Returns the number of bytes that would be written when encoding this
    /// value according to the MSDP protocol. This includes any control bytes
    /// and structural markers for arrays and tables.
    ///
    /// # Returns
    /// The total encoded byte length.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let value = MudServerDataValue::string("test");
    /// println!("Length: {}", value.len());
    /// ```
    pub fn len(&self) -> usize {
        match self {
            MudServerDataValue::String(string) => string.len(),
            MudServerDataValue::Array(array) => array.len(),
            MudServerDataValue::Table(table) => table.len(),
        }
    }

    /// Encodes this `MudServerDataValue` into the provided mutable buffer.
    ///
    /// Serializes the value according to the MSDP protocol and writes it
    /// to the destination buffer.
    ///
    /// # Type Parameters
    ///
    /// * `T` - A type implementing `BufMut` for buffer operations
    ///
    /// # Arguments
    ///
    /// * `dst` - A mutable reference to the destination buffer
    ///
    /// # Returns
    ///
    /// * `Ok(usize)` - The number of bytes written
    /// * `Err(CodecResult)` - If encoding fails
    pub fn encode<T: BufMut>(&self, dst: &mut T) -> CodecResult<usize> {
        Ok(self.write(&mut dst.writer())?)
    }

    /// Writes this `MudServerDataValue` to the provided writer.
    ///
    /// Low-level method for writing the encoded value to a generic writer.
    ///
    /// # Arguments
    ///
    /// * `writer` - A mutable reference to a type implementing `std::io::Write`
    ///
    /// # Returns
    ///
    /// * `Ok(usize)` - The number of bytes written
    /// * `Err(std::io::Error)` - If writing fails
    pub fn write<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<usize> {
        match self {
            MudServerDataValue::String(string) => {
                writer.write(string.as_bytes())?;
                Ok(string.len())
            }
            MudServerDataValue::Array(array) => array.write(writer),
            MudServerDataValue::Table(table) => table.write(writer),
        }
    }

    /// Decodes a `MudServerDataValue` from the provided buffer.
    ///
    /// Deserializes a value according to the MSDP protocol from the source buffer.
    /// The method automatically detects the value type based on control bytes:
    /// - `ARRAY_OPEN` indicates an array
    /// - `TABLE_OPEN` indicates a table
    /// - Other bytes are treated as string data
    ///
    /// # Type Parameters
    ///
    /// * `T` - A type implementing `Buf` for buffer operations
    ///
    /// # Arguments
    ///
    /// * `src` - A mutable reference to the source buffer
    ///
    /// # Returns
    ///
    /// * `Ok(MudServerDataValue)` - The decoded value
    /// * `Err(CodecResult)` - If decoding fails
    ///
    /// # Notes
    ///
    /// Returns an empty string if the buffer is empty.
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

impl std::fmt::Display for MudServerDataValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MudServerDataValue::String(value) => std::fmt::Display::fmt(value, f),
            MudServerDataValue::Array(array) => std::fmt::Display::fmt(array, f),
            MudServerDataValue::Table(table) => std::fmt::Display::fmt(table, f),
        }
    }
}

/// `MudServerDataArray` is an ordered collection of `MudServerDataValue` elements.
///
/// Arrays are used in MSDP to represent lists of values. Each array is
/// delimited by `ARRAY_OPEN` and `ARRAY_CLOSE` control bytes, with individual
/// elements preceded by `VAL` markers.
///
/// # Examples
///
/// ```ignore
/// let mut array = MudServerDataArray::new();
/// array.push(MudServerDataValue::string("item1"));
/// array.push(MudServerDataValue::string("item2"));
/// ```
#[derive(Clone, Debug, Default)]
pub struct MudServerDataArray(Vec<MudServerDataValue>);

impl MudServerDataArray {
    /// Creates a new empty array.
    ///
    /// # Returns
    /// A new `MudServerDataArray` with no elements.
    pub fn new() -> MudServerDataArray {
        MudServerDataArray(Vec::new())
    }
    /// Adds a value to the end of the array.
    ///
    /// # Arguments
    ///
    /// * `value` - The `MudServerDataValue` to add
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut array = MudServerDataArray::new();
    /// array.push(MudServerDataValue::string("element"));
    /// ```
    pub fn push(&mut self, value: MudServerDataValue) {
        self.0.push(value);
    }
    /// Retrieves a reference to the value at the specified index.
    ///
    /// # Arguments
    ///
    /// * `index` - The zero-based index of the element
    ///
    /// # Returns
    /// `Some(&MudServerDataValue)` if the index is valid, otherwise `None`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let array = MudServerDataArray::new();
    /// if let Some(value) = array.get(0) {
    ///     println!("{}", value);
    /// }
    /// ```
    pub fn get(&self, index: usize) -> Option<&MudServerDataValue> {
        self.0.get(index)
    }
    /// Retrieves a mutable reference to the value at the specified index.
    ///
    /// # Arguments
    ///
    /// * `index` - The zero-based index of the element
    ///
    /// # Returns
    /// `Some(&MudServerDataValue)` if the index is valid, otherwise `None`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut array = MudServerDataArray::new();
    /// if let Some(value) = array.get_mut(0) {
    ///     println!("{}", value);
    /// }
    /// ```
    pub fn get_mut(&mut self, index: usize) -> Option<&mut MudServerDataValue> {
        self.0.get_mut(index)
    }

    /// Gets the encoded length of this array.
    ///
    /// Returns the total number of bytes that would be written when encoding
    /// this array according to the MSDP protocol. This includes the `ARRAY_OPEN`,
    /// `ARRAY_CLOSE` delimiters and `VAL` markers for each element.
    ///
    /// # Returns
    /// The total encoded byte length.
    pub fn len(&self) -> usize {
        let mut length = 0;
        length += 1; // ARRAY_OPEN
        for value in &self.0 {
            length += 1; // VAL
            length += value.len();
        }
        length += 1; // ARRAY_CLOSE
        length
    }

    /// Encodes this array into the provided mutable buffer.
    ///
    /// Serializes the array according to the MSDP protocol and writes it
    /// to the destination buffer.
    ///
    /// # Type Parameters
    ///
    /// * `T` - A type implementing `BufMut` for buffer operations
    ///
    /// # Arguments
    ///
    /// * `dst` - A mutable reference to the destination buffer
    ///
    /// # Returns
    ///
    /// * `Ok(usize)` - The number of bytes written
    /// * `Err(CodecResult)` - If encoding fails
    pub fn encode<T: BufMut>(&self, dst: &mut T) -> CodecResult<usize> {
        Ok(self.write(&mut dst.writer())?)
    }

    /// Writes this array to the provided writer.
    ///
    /// Low-level method for writing the encoded array to a generic writer.
    ///
    /// # Arguments
    ///
    /// * `writer` - A mutable reference to a type implementing `std::io::Write`
    ///
    /// # Returns
    ///
    /// * `Ok(usize)` - The number of bytes written
    /// * `Err(std::io::Error)` - If writing fails
    pub fn write<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<usize> {
        let mut len = 2;
        writer.write_u8(consts::option::msdp::ARRAY_OPEN)?;
        for value in &self.0 {
            writer.write_u8(consts::option::msdp::VAL)?;
            len += 1 + value.write(writer)?;
        }
        writer.write_u8(consts::option::msdp::ARRAY_CLOSE)?;
        Ok(len)
    }

    /// Decodes an array from the provided buffer.
    ///
    /// Deserializes an array according to the MSDP protocol from the source buffer.
    /// Expects the buffer to start with an `ARRAY_OPEN` byte and end with an
    /// `ARRAY_CLOSE` byte, with elements preceded by `VAL` markers.
    ///
    /// # Type Parameters
    ///
    /// * `T` - A type implementing `Buf` for buffer operations
    ///
    /// # Arguments
    ///
    /// * `src` - A mutable reference to the source buffer
    ///
    /// # Returns
    ///
    /// * `Ok(MudServerDataArray)` - The decoded array
    /// * `Err(CodecResult)` - If decoding fails
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

impl std::fmt::Display for MudServerDataArray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for v in &self.0 {
            std::fmt::Display::fmt(v, f)?;
            write!(f, ", ")?;
        }
        Ok(())
    }
}

/// `MudServerDataTable` is a collection of key-value pairs.
///
/// Tables are used in MSDP to represent structured data with named fields.
/// Each table is delimited by `TABLE_OPEN` and `TABLE_CLOSE` control bytes,
/// with each key-value pair preceded by `VAR` and `VAL` markers respectively.
///
/// # Examples
///
/// ```ignore
/// let mut table = MudServerDataTable::new();
/// table.set("name", MudServerDataValue::string("MUD Name"));
/// table.set("version", MudServerDataValue::string("1.0"));
/// ```
#[derive(Clone, Debug, Default)]
pub struct MudServerDataTable(HashMap<String, MudServerDataValue>);

impl MudServerDataTable {
    /// Creates a new empty table.
    ///
    /// # Returns
    /// A new `MudServerDataTable` with no entries.
    pub fn new() -> MudServerDataTable {
        MudServerDataTable(HashMap::new())
    }
    /// Sets the value associated with the given key.
    ///
    /// If the key already exists, its value is replaced with the new value.
    ///
    /// # Arguments
    ///
    /// * `key` - The key (as a string slice)
    /// * `value` - The `MudServerDataValue` to associate with the key
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut table = MudServerDataTable::new();
    /// table.set("name", MudServerDataValue::string("My MUD"));
    /// ```
    pub fn set(&mut self, key: &str, value: MudServerDataValue) {
        self.0.insert(key.to_string(), value);
    }

    /// Retrieves the value associated with the given key.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up (as a string slice)
    ///
    /// # Returns
    /// `Some(&MudServerDataValue)` if the key exists, otherwise `None`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let table = MudServerDataTable::new();
    /// if let Some(value) = table.get("name") {
    ///     println!("MUD Name: {}", value);
    /// }
    /// ```
    pub fn get(&self, key: &str) -> Option<&MudServerDataValue> {
        self.0.get(key)
    }

    /// Retrieves the mutable value associated with the given key.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up (as a string slice)
    ///
    /// # Returns
    /// `Some(&MudServerDataValue)` if the key exists, otherwise `None`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut table = MudServerDataTable::new();
    /// if let Some(value) = table.get_mut("name") {
    ///     println!("MUD Name: {}", value);
    /// }
    /// ```
    pub fn get_mut(&mut self, key: &str) -> Option<&mut MudServerDataValue> {
        self.0.get_mut(key)
    }

    /// Gets the encoded length of this table.
    ///
    /// Returns the total number of bytes that would be written when encoding
    /// this table according to the MSDP protocol. This includes the `TABLE_OPEN`,
    /// `TABLE_CLOSE` delimiters, `VAR` and `VAL` markers for each key-value pair,
    /// and the encoded lengths of all keys and values.
    ///
    /// # Returns
    /// The total encoded byte length.
    pub fn len(&self) -> usize {
        let mut length = 0;
        length += 1; // TABLE_OPEN
        for (key, value) in &self.0 {
            length += 1; // VAR
            length += key.len();
            length += 1; // VAL
            length += value.len();
        }
        length += 1; // TABLE_CLOSE
        length
    }

    /// Encodes this table into the provided mutable buffer.
    ///
    /// Serializes the table according to the MSDP protocol and writes it
    /// to the destination buffer.
    ///
    /// # Type Parameters
    ///
    /// * `T` - A type implementing `BufMut` for buffer operations
    ///
    /// # Arguments
    ///
    /// * `dst` - A mutable reference to the destination buffer
    ///
    /// # Returns
    ///
    /// * `Ok(usize)` - The number of bytes written
    /// * `Err(CodecResult)` - If encoding fails
    pub fn encode<T: BufMut>(&self, dst: &mut T) -> CodecResult<usize> {
        Ok(self.write(&mut dst.writer())?)
    }

    /// Writes this table to the provided writer.
    ///
    /// Low-level method for writing the encoded table to a generic writer.
    ///
    /// # Arguments
    ///
    /// * `writer` - A mutable reference to a type implementing `std::io::Write`
    ///
    /// # Returns
    ///
    /// * `Ok(usize)` - The number of bytes written
    /// * `Err(std::io::Error)` - If writing fails
    pub fn write<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<usize> {
        let mut len = 2;
        writer.write_u8(consts::option::msdp::TABLE_OPEN)?;
        for (key, value) in &self.0 {
            writer.write_u8(consts::option::msdp::VAR)?;
            writer.write(key.as_bytes())?;
            writer.write_u8(consts::option::msdp::VAL)?;
            len += 2 + key.len() + value.write(writer)?;
        }
        writer.write_u8(consts::option::msdp::TABLE_CLOSE)?;
        Ok(len)
    }

    /// Decodes a table from the provided buffer.
    ///
    /// Deserializes a table according to the MSDP protocol from the source buffer.
    /// Handles both standalone tables (with `TABLE_OPEN`/`TABLE_CLOSE` markers)
    /// and nested tables within other structures.
    ///
    /// # Type Parameters
    ///
    /// * `T` - A type implementing `Buf` for buffer operations
    ///
    /// # Arguments
    ///
    /// * `src` - A mutable reference to the source buffer
    ///
    /// # Returns
    ///
    /// * `Ok(MudServerDataTable)` - The decoded table
    /// * `Err(CodecResult)` - If decoding fails
    ///
    /// # Notes
    ///
    /// This method automatically detects whether the table has explicit
    /// `TABLE_OPEN`/`TABLE_CLOSE` markers and handles both cases appropriately.
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

impl std::fmt::Display for MudServerDataTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (key, value) in &self.0 {
            write!(f, "{key}: {value}, ")?;
        }
        Ok(())
    }
}
