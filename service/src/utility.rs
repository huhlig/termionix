//
// Copyright 2025 Hans W. Uhlig. All Rights Reserved.
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

use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;

pub struct RwLockReadReference<'a, T, R> {
    _guard: std::sync::RwLockReadGuard<'a, T>,
    value: &'a R,
}

impl<'a, T, R> RwLockReadReference<'a, T, R> {
    pub(crate) fn new(
        _guard: std::sync::RwLockReadGuard<'a, T>,
        value: &'a R,
    ) -> RwLockReadReference<'a, T, R> {
        RwLockReadReference { _guard, value }
    }
}

impl<'a, T, R> Deref for RwLockReadReference<'_, T, R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

pub struct RwLockWriteReference<'a, T, R> {
    _guard: std::sync::RwLockWriteGuard<'a, T>,
    value: &'a mut R,
}

impl<'a, T, R> RwLockWriteReference<'a, T, R> {
    pub(crate) fn new(
        _guard: std::sync::RwLockWriteGuard<'a, T>,
        value: &'a mut R,
    ) -> RwLockWriteReference<'a, T, R> {
        RwLockWriteReference { _guard, value }
    }
}

impl<'a, T, R> Deref for RwLockWriteReference<'_, T, R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'a, T, R> DerefMut for RwLockWriteReference<'_, T, R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value
    }
}
