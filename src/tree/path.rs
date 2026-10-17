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

//! The grammar of a path: the segments of a JSON pointer, with a position, `*` and `-`.

use super::{Error, ErrorKind};
use alloc::borrow::Cow;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

/// The most octets that a path has.
pub const MAX_OCTETS: usize = 4096;
/// The most segments that a path has, the root among them. A tree that is nested deeper
/// is one value from there on.
pub const MAX_SEGMENTS: usize = 64;
/// The most values that a path selects: as many as the longest list of the application
/// protocols has, the 65,536 connections of an NG RESET (maxnoofNGConnectionsToReset of
/// TS 38.413), so that `*` takes each entry of any list that a message has.
pub const MAX_SELECTED: usize = 65_536;

/// The segments of a path: the text between its `/`, with `~1` for a `/` and `~0` for a
/// `~` in a segment, as a JSON pointer (RFC 6901) writes them.
///
/// The empty path has no segment: it is the path of the tree itself.
///
/// # Errors
///
/// The kind `Path` for other text that does not start with `/` and for a `~` that is
/// followed by neither `0` nor `1`; the kind `Limit` for more than 4096 octets or 64
/// segments.
///
/// ```
/// use oxirush_utils::tree;
///
/// assert_eq!(tree::segments("/nas/5gmm-cause/value")?, ["nas", "5gmm-cause", "value"]);
/// assert_eq!(tree::segments("/a~1b/~0")?, ["a/b", "~"]);
/// assert!(tree::segments("nas/5gmm-cause").is_err());
/// # Ok::<(), oxirush_utils::tree::Error>(())
/// ```
pub fn segments(path: &str) -> Result<Vec<String>, Error> {
    if path.len() > MAX_OCTETS {
        let reason = format!("a path has {MAX_OCTETS} octets at most");
        // The start of the path says which one it is.
        let start = path
            .char_indices()
            .nth(64)
            .map_or(path, |(end, _)| &path[..end]);
        return Err(Error::new(ErrorKind::Limit, start, reason));
    }
    if path.is_empty() {
        return Ok(Vec::new());
    }
    let Some(rest) = path.strip_prefix('/') else {
        let reason = format!("{path:?} is no path: a path starts with \"/\"");
        return Err(Error::new(ErrorKind::Path, path, reason));
    };
    let mut parts = Vec::new();
    for part in rest.split('/') {
        if parts.len() == MAX_SEGMENTS {
            let reason = format!("a path has {MAX_SEGMENTS} segments at most");
            return Err(Error::new(ErrorKind::Limit, path, reason));
        }
        let mut decoded = String::with_capacity(part.len());
        let mut characters = part.chars();
        while let Some(character) = characters.next() {
            decoded.push(match (character, character == '~') {
                (plain, false) => plain,
                (_, true) => match characters.next() {
                    Some('0') => '~',
                    Some('1') => '/',
                    _ => {
                        let reason = "in a path, \"~\" is followed by 0 or 1, which stand for \
                                      \"~\" and \"/\"";
                        return Err(Error::new(ErrorKind::Path, path, reason));
                    }
                },
            });
        }
        parts.push(decoded);
    }
    Ok(parts)
}

/// The position that a segment is: a number as decimal writes it, without a sign and
/// without a zero before it.
///
/// ```
/// use oxirush_utils::tree::position;
///
/// assert_eq!((position("0"), position("12")), (Some(0), Some(12)));
/// assert_eq!((position("01"), position("+1"), position("-")), (None, None, None));
/// ```
pub fn position(segment: &str) -> Option<usize> {
    let plain = !segment.is_empty()
        && segment.bytes().all(|digit| digit.is_ascii_digit())
        && (segment == "0" || !segment.starts_with('0'));
    segment.parse().ok().filter(|_| plain)
}

/// A segment as a path writes it: `~0` for its `~`, and `~1` for its `/`.
///
/// ```
/// use oxirush_utils::tree::escape;
///
/// assert_eq!(escape("a/b~c"), "a~1b~0c");
/// assert_eq!(escape("5gmm-cause"), "5gmm-cause");
/// ```
pub fn escape(segment: &str) -> Cow<'_, str> {
    match segment.contains(['~', '/']) {
        true => Cow::Owned(segment.replace('~', "~0").replace('/', "~1")),
        false => Cow::Borrowed(segment),
    }
}
