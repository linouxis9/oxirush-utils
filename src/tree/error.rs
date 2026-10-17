/*
   OxiRush
   Copyright 2025-2026 Valentin D'Emmanuele

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

   http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

//! The one error of the tree module.

use alloc::string::String;
use core::fmt;

/// What kind of error it is: what a caller matches on.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// Text that is no path: it does not start with `/`, a `~` in it is no escape, or it
    /// starts with another root than that of the tree.
    Path,
    /// A limit: a path of more than 4096 octets or 64 segments, or one that selects more
    /// than 65,536 values.
    Limit,
    /// A name that the value at that place does not have and cannot have.
    NoSuchMember,
    /// A segment that is no position, where the value is a list; or an insertion into a
    /// value that is no list.
    NotAList,
    /// A path that goes on into a value without members: a number, a string.
    Scalar,
    /// An edit whose path selects nothing: the tree is as it was.
    NothingSelected,
    /// A value that is not written as its type is.
    Value,
    /// What the family of the tree refuses at that place, for a reason of its own.
    Refused,
}

/// Why a tree is not read, or a path not taken: the kind of error, where it is, and why.
///
/// ```
/// use oxirush_utils::{tree, tree::{ErrorKind}};
///
/// let tree = tree!({"cause": {"value": "congestion"}});
/// let error = tree::select(&tree, "/cause/valeu").unwrap_err();
/// assert_eq!(error.kind(), ErrorKind::NoSuchMember);
/// assert_eq!(error.at(), "/cause/valeu");
/// assert_eq!(
///     error.to_string(),
///     "\"valeu\" is not a member of this value, which has value, at /cause/valeu"
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    kind: ErrorKind,
    at: String,
    reason: String,
    code: Option<u16>,
}

impl Error {
    /// An error of a kind, at a path or at the names that lead to a member, for a
    /// reason. A family makes the errors of its own rules with it.
    pub fn new(kind: ErrorKind, at: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            kind,
            at: at.into(),
            reason: reason.into(),
            code: None,
        }
    }

    /// The same error with a code of the family's own, which says more than the kind:
    /// the kind of the family's own error, which the engine carries as it is.
    pub fn with_code(mut self, code: u16) -> Self {
        self.code = Some(code);
        self
    }

    /// The code that the family gave the error, when it gave one.
    pub fn code(&self) -> Option<u16> {
        self.code
    }

    /// The kind of error.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Where it is: the path that was given, or the names and the positions that lead to
    /// the member of a value that is read, with `/` between them. Empty for the value
    /// itself.
    pub fn at(&self) -> &str {
        &self.at
    }

    /// The same error at `path`, when it does not say where it is: an error that a
    /// family made without the path.
    pub(crate) fn located(mut self, path: &str) -> Self {
        if self.at.is_empty() {
            self.at = String::from(path);
        }
        self
    }

    /// Why, without the place.
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.at.is_empty() {
            true => f.write_str(&self.reason),
            false => write!(f, "{}, at {}", self.reason, self.at),
        }
    }
}

impl core::error::Error for Error {}
