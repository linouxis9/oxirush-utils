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

//! Paths on a tree: `select`, `paths`, `set`, `remove` and `insert`, and what a family of
//! messages adds to them where its trees are more than objects and lists.

use super::forms::{Form, key, same_name};
use super::path::{MAX_SEGMENTS, MAX_SELECTED, escape, position, segments};
use super::{Error, ErrorKind};
use alloc::format;
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use serde_json::{Map, Value};

// ── What a family says ──────────────────────────────────────────────────────

/// The first segment of the paths of a family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Root<'a> {
    /// The name of the root, as `nas`: a path starts with `/nas`. Empty for a family
    /// whose paths start at the tree itself.
    pub name: &'a str,
    /// The segments that the root is short for, from the tree: none when the root is the
    /// tree itself.
    pub stands_for: &'a [&'a str],
    /// Whether a path has to start with the root. A path that does not is one from the
    /// tree itself, where it need not.
    pub required: bool,
}

impl Root<'_> {
    /// No root: a path starts at the tree itself.
    pub const NONE: Self = Self {
        name: "",
        stands_for: &[],
        required: false,
    };
}

/// What an edit does at its path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edit {
    /// Give a member or an entry a value.
    Set,
    /// Take a member or an entry out.
    Remove,
    /// Add an entry to a list.
    Insert,
}

/// One step from the tree to a place of it: the name of a member as the tree has it, the
/// position of an entry of a list, or the end of a list, where `insert` adds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    /// A member, by its name.
    Name(String),
    /// An entry, by its position.
    Position(usize),
    /// After the last entry of a list.
    End,
}

/// The keys from the tree to a place, last first: each place holds its own and shares
/// those before it.
#[derive(Debug)]
struct Link {
    key: Key,
    before: Option<Rc<Link>>,
}

/// A place on the way of a path: a value of the tree, or none where the tree has no
/// value and could have one, with what the family knows of it.
#[derive(Debug, Clone)]
pub struct Place<'v, S> {
    value: Option<&'v Value>,
    shape: S,
    /// The keys to the place, when the walk is that of an edit.
    at: Option<Rc<Link>>,
}

impl<'v, S> Place<'v, S> {
    /// The value at the place: `None` for a member that is not there.
    pub fn value(&self) -> Option<&'v Value> {
        self.value
    }

    /// What the family knows of the place.
    pub fn shape(&self) -> &S {
        &self.shape
    }
}

/// What a segment selects, from one place: the family adds to it in its
/// [`step`](Family::step), and the engine in the others.
#[derive(Debug)]
pub struct Next<'v, S> {
    found: Vec<Place<'v, S>>,
    /// The keys to the place that is being stepped from, and whether keys are kept.
    from: Option<Rc<Link>>,
    tracking: bool,
    edit: Option<Edit>,
    last: bool,
}

impl<'v, S> Next<'v, S> {
    fn link(&self, key: impl FnOnce() -> Key) -> Option<Rc<Link>> {
        self.tracking.then(|| {
            Rc::new(Link {
                key: key(),
                before: self.from.clone(),
            })
        })
    }

    /// The member `name` of the place, which has the value `value`: `name` is the name
    /// as the tree has it.
    pub fn member(&mut self, name: &str, value: &'v Value, shape: S) {
        let at = self.link(|| Key::Name(name.to_string()));
        let value = Some(value);
        self.found.push(Place { value, shape, at });
    }

    /// The entry at `position` of the place, which is a list.
    pub fn entry(&mut self, position: usize, value: &'v Value, shape: S) {
        let at = self.link(|| Key::Position(position));
        let value = Some(value);
        self.found.push(Place { value, shape, at });
    }

    /// A member `name` that the place does not have and could have. `select` finds
    /// nothing there, and `set` gives the place the member.
    pub fn absent(&mut self, name: &str, shape: S) {
        let at = self.link(|| Key::Name(name.to_string()));
        let value = None;
        self.found.push(Place { value, shape, at });
    }

    /// The same place again, as `place`, with what the family now knows of it: for a
    /// segment that says what the place is and does not go into it.
    pub fn stay(&mut self, place: &Place<'v, S>, shape: S) {
        let (value, at) = (place.value, place.at.clone());
        self.found.push(Place { value, shape, at });
    }

    /// What the path is for: `None` for `select`.
    pub fn edit(&self) -> Option<Edit> {
        self.edit
    }

    /// Whether the segment is the last of its path.
    pub fn is_last(&self) -> bool {
        self.last
    }
}

/// What a family does with a segment at a place.
#[derive(Debug)]
pub enum Step<S> {
    /// The family added to [`Next`] what the segment selects there, which may be nothing.
    Taken,
    /// The plain rule applies: a member by its name, an entry by its position, `*`.
    Plain,
    /// The segment is taken in the member of this name, as if the name stood before it:
    /// the value of an information element, the decoded content of a container.
    Through(String, S),
}

/// What a family says of a name that an object does not have.
#[derive(Debug)]
pub enum Absent<S> {
    /// The family knows nothing of the name. `select` refuses it with the kind
    /// `NoSuchMember`, so that a misspelt name is not a member that is absent; `set`
    /// adds the member.
    Unknown,
    /// A member that the value could have: `select` finds nothing, and `set` adds it.
    Optional(S),
    /// A name that the value cannot have, with the error that says why.
    Refused(Error),
}

/// How [`paths`] lists a member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Listed {
    /// Under its name.
    Member,
    /// Under another name.
    As(&'static str),
    /// Not at all: what a view keeps of what was received.
    Hidden,
    /// In the place of the value that has it, without a segment of its own.
    Through,
    /// Under the root of the family: the member that the root is short for.
    Root,
}

/// How a member is taken out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Removal {
    /// The member goes.
    Take,
    /// The member stays, as `null`: a view that keeps the name of each information
    /// element that its message can have.
    Null,
}

/// What a family of messages adds to the paths of its trees, where they are more than
/// objects and lists: an entry of a list that is selected by a name, a member whose
/// content is selected as if its name stood in the path, a member that is absent and
/// could be there.
///
/// Every method has the plain rule as its default, and [`Plain`] has nothing else. A
/// family keeps what it knows of a place in its [`Shape`](Self::Shape), which the engine
/// carries along a path without looking at it: the family reads its own tables with it,
/// and the engine knows no codec.
///
/// ```
/// use oxirush_utils::tree::{self, Family, Next, Place, Root, Step};
/// use serde_json::json;
///
/// /// A list of `{"id": .., "value": ..}` whose entries are selected by their `id`.
/// struct ById;
///
/// impl Family for ById {
///     type Shape = ();
///
///     fn root(&self) -> Root<'_> {
///         Root { name: "list", stands_for: &[], required: true }
///     }
///
///     fn step<'v>(
///         &self,
///         place: &Place<'v, ()>,
///         segment: &str,
///         next: &mut Next<'v, ()>,
///     ) -> Result<Step<()>, tree::Error> {
///         let Some(entries) = place.value().and_then(|value| value.as_array()) else {
///             return Ok(Step::Plain);
///         };
///         let named = |entry: &serde_json::Value| entry["id"].as_str() == Some(segment);
///         if !entries.iter().any(named) {
///             return Ok(Step::Plain);
///         }
///         for (position, entry) in entries.iter().enumerate().filter(|(_, entry)| named(entry)) {
///             next.entry(position, entry, ());
///         }
///         Ok(Step::Taken)
///     }
/// }
///
/// let mut list = json!([{"id": "cause", "value": 3}, {"id": "timer", "value": 60}]);
/// assert_eq!(tree::select_in(&ById, &list, "/list/timer/value")?, [&json!(60)]);
/// tree::set_in(&ById, &mut list, "/list/cause/value", json!(7))?;
/// assert_eq!(list[0]["value"], 7);
/// # Ok::<(), tree::Error>(())
/// ```
pub trait Family {
    /// What the family knows of a place of a tree: `()` for one that knows nothing.
    type Shape: Clone + Default;

    /// The root of the paths of the family.
    fn root(&self) -> Root<'_> {
        Root::NONE
    }

    /// What the family knows of a tree, where a path starts. `rooted` says whether the
    /// path starts with the root.
    fn start(&self, tree: &Value, rooted: bool) -> Self::Shape {
        let _ = (tree, rooted);
        Self::Shape::default()
    }

    /// What `segment` selects at `place`, where the family has a rule of its own.
    ///
    /// # Errors
    ///
    /// Why the segment selects nothing at the place. An error of the kind `Refused`
    /// ends the path. One of another kind ends it only when the segment selects nothing
    /// at any place: after `*`, a name that one entry does not have is no error when
    /// another has it.
    fn step<'v>(
        &self,
        place: &Place<'v, Self::Shape>,
        segment: &str,
        next: &mut Next<'v, Self::Shape>,
    ) -> Result<Step<Self::Shape>, Error> {
        let _ = (place, segment, next);
        Ok(Step::Plain)
    }

    /// What the family knows of the member `name` of a value of `shape`, which the plain
    /// rule reached. `value` is the member, or `None` for one that `set` adds.
    fn member(&self, shape: &Self::Shape, name: &str, value: Option<&Value>) -> Self::Shape {
        let _ = (shape, name, value);
        Self::Shape::default()
    }

    /// What the family knows of an entry of a list of `shape`.
    fn entry(&self, shape: &Self::Shape, value: Option<&Value>) -> Self::Shape {
        let _ = (shape, value);
        Self::Shape::default()
    }

    /// What a name is that an object of `shape` does not have.
    fn absent(&self, shape: &Self::Shape, name: &str) -> Absent<Self::Shape> {
        let _ = (shape, name);
        Absent::Unknown
    }

    /// How [`paths`] lists the member `name` of a value of `shape`. `*` leaves out the
    /// members that are hidden.
    fn listed(&self, shape: &Self::Shape, name: &str) -> Listed {
        let _ = (shape, name);
        Listed::Member
    }

    /// The segment that selects each entry of a list of `shape` alone, where an entry is
    /// selected by a name: `None` for a list whose entries go by their position, and for
    /// an entry that does.
    ///
    /// It is asked once for a list, so that the paths of a tree take a time that grows
    /// with the size of the tree and no faster: a family counts the names that are there
    /// twice in one pass.
    fn entry_names(&self, shape: &Self::Shape, list: &[Value]) -> Option<Vec<Option<String>>> {
        let _ = (shape, list);
        None
    }

    /// How the member at a place of `shape` is taken out.
    fn removal(&self, shape: &Self::Shape) -> Removal {
        let _ = shape;
        Removal::Take
    }

    /// How the value at a place of `shape` is written, for a comparison with a value
    /// that someone wrote: see [`written_as`](super::written_as).
    fn form(&self, shape: &Self::Shape, value: &Value) -> Form {
        let _ = (shape, value);
        Form::Plain
    }

    /// What else an edit changes: it is called for each place that was edited, with the
    /// tree as the edit left it and the keys to the place.
    ///
    /// # Errors
    ///
    /// Why the family refuses the edit: the tree that was given stays as it was.
    fn edited(&self, tree: &mut Value, at: &[Key], edit: Edit) -> Result<(), Error> {
        let _ = (tree, at, edit);
        Ok(())
    }
}

/// The family of the trees that are objects and lists and nothing more: every rule is
/// the plain one, and a path starts at the tree itself.
#[derive(Debug, Clone, Copy, Default)]
pub struct Plain;

impl Family for Plain {
    type Shape = ();
}

// ── The walk ────────────────────────────────────────────────────────────────

/// The segments of `path` from the tree, with what the root is short for in its place,
/// and whether the path starts with the root.
fn resolve(root: Root<'_>, path: &str) -> Result<(Vec<String>, bool), Error> {
    let mut parts = segments(path)?;
    if root.name.is_empty() {
        return Ok((parts, false));
    }
    let rooted = parts
        .first()
        .is_some_and(|first| same_name(first, root.name));
    if rooted {
        let stands_for = root.stands_for.iter().map(|part| part.to_string());
        parts.splice(..1, stands_for);
    } else if root.required {
        let reason = format!(
            "{path} is not a path of this tree: it starts with /{}",
            root.name
        );
        return Err(Error::new(ErrorKind::Path, path, reason));
    }
    Ok((parts, rooted))
}

/// Why a name selects nothing in an object: the names that it has.
fn no_member(members: &Map<String, Value>, part: &str, path: &str) -> Error {
    const SHOWN: usize = 12;
    let mut names: Vec<&str> = members.keys().take(SHOWN).map(String::as_str).collect();
    if members.len() > SHOWN {
        names.push("…");
    }
    let reason = match names.is_empty() {
        true => format!("{part:?} is not a member of this value, which has none"),
        false => format!(
            "{part:?} is not a member of this value, which has {}",
            names.join(", ")
        ),
    };
    Error::new(ErrorKind::NoSuchMember, path, reason)
}

/// Add to `next` what `part` selects at `place`, by the plain rule.
fn plain<'v, F: Family>(
    family: &F,
    place: &Place<'v, F::Shape>,
    part: &str,
    path: &str,
    next: &mut Next<'v, F::Shape>,
) -> Result<(), Error> {
    let shape = &place.shape;
    match place.value {
        // Nothing is there, and nothing is selected in it.
        None | Some(Value::Null) => Ok(()),
        Some(Value::Object(members)) => {
            if part == "*" {
                for (name, value) in members {
                    if family.listed(shape, name) != Listed::Hidden {
                        next.member(name, value, family.member(shape, name, Some(value)));
                    }
                }
                return Ok(());
            }
            if let Some(name) = key(members, part) {
                let value = &members[name];
                next.member(name, value, family.member(shape, name, Some(value)));
                return Ok(());
            }
            match family.absent(shape, part) {
                Absent::Optional(shape) => next.absent(part, shape),
                Absent::Refused(error) => return Err(error),
                // An object that the family knows nothing of takes a new member.
                Absent::Unknown if next.last && next.edit == Some(Edit::Set) => {
                    next.absent(part, family.member(shape, part, None));
                }
                Absent::Unknown => return Err(no_member(members, part, path)),
            }
            Ok(())
        }
        Some(Value::Array(entries)) => {
            if part == "*" {
                for (position, entry) in entries.iter().enumerate() {
                    next.entry(position, entry, family.entry(shape, Some(entry)));
                }
            } else if part == "-" && next.last && next.edit == Some(Edit::Insert) {
                let at = next.link(|| Key::End);
                let (value, shape) = (None, family.entry(shape, None));
                next.found.push(Place { value, shape, at });
            } else if let Some(position) = position(part) {
                // An entry past the end is one that is not there.
                if let Some(entry) = entries.get(position) {
                    next.entry(position, entry, family.entry(shape, Some(entry)));
                }
            } else {
                let reason = format!(
                    "{part:?} is no position in a list: a position is a number as decimal \
                     writes it, or \"*\" for each entry"
                );
                return Err(Error::new(ErrorKind::NotAList, path, reason));
            }
            Ok(())
        }
        Some(_) => {
            let reason = format!("{part:?} is not a member of this value, which has none");
            Err(Error::new(ErrorKind::Scalar, path, reason))
        }
    }
}

/// Add to `next` what `part` selects at `place`: by the rule of the family where it has
/// one, and by the plain rule otherwise.
fn advance<'v, F: Family>(
    family: &F,
    place: &Place<'v, F::Shape>,
    part: &str,
    path: &str,
    next: &mut Next<'v, F::Shape>,
    through: usize,
) -> Result<(), Error> {
    next.from = place.at.clone();
    match family.step(place, part, next)? {
        Step::Taken => Ok(()),
        Step::Plain => plain(family, place, part, path, next),
        Step::Through(name, shape) => {
            // A family that goes through member after member without an end has a fault.
            if through == MAX_SEGMENTS {
                let reason = "the segment is taken through more members than a path has segments";
                return Err(Error::new(ErrorKind::Refused, path, reason));
            }
            let members = place.value.and_then(Value::as_object);
            let held = members.and_then(|members| {
                let name = key(members, &name)?;
                Some((name, &members[name]))
            });
            let at =
                next.link(|| Key::Name(held.map_or(name.clone(), |(name, _)| name.to_string())));
            let inside = Place {
                value: held.map(|(_, value)| value),
                shape,
                at,
            };
            advance(family, &inside, part, path, next, through + 1)
        }
    }
}

/// The places that `path` selects in `tree`, for `select` or for an edit.
fn walk<'v, F: Family>(
    family: &F,
    tree: &'v Value,
    path: &str,
    edit: Option<Edit>,
) -> Result<Vec<Place<'v, F::Shape>>, Error> {
    let (parts, rooted) = resolve(family.root(), path)?;
    let mut places = vec![Place {
        value: Some(tree),
        shape: family.start(tree, rooted),
        at: None,
    }];
    for (index, part) in parts.iter().enumerate() {
        let mut next = Next {
            found: Vec::new(),
            from: None,
            tracking: edit.is_some(),
            edit,
            last: index + 1 == parts.len(),
        };
        let mut refusal = None;
        for place in &places {
            match advance(family, place, part, path, &mut next, 0) {
                Ok(()) => {}
                Err(error) if error.kind() == ErrorKind::Refused => {
                    return Err(error.located(path));
                }
                Err(error) => refusal = refusal.or(Some(error.located(path))),
            }
            if next.found.len() > MAX_SELECTED {
                let reason = format!("a path selects {MAX_SELECTED} values at most");
                return Err(Error::new(ErrorKind::Limit, path, reason));
            }
        }
        // What one place refuses, another may have: an error when none has it.
        if let (true, Some(refusal)) = (next.found.is_empty(), refusal) {
            return Err(refusal);
        }
        places = next.found;
    }
    Ok(places)
}

// ── Select ──────────────────────────────────────────────────────────────────

/// The values at `path` in a tree of the family `family`, in the order of the tree.
///
/// A segment is the name of a member, written [however](super::same_name), the position
/// of an entry of a list, or `*` for each member or each entry. What is not there
/// selects nothing: a member that is `null`, an entry past the end of a list, and what a
/// path goes on to under them.
///
/// # Errors
///
/// The kinds `Path` and `Limit` of [`segments`], and `Limit` for more than 4096 values;
/// `NoSuchMember` for a name that the value at that place does not have, so that a
/// misspelt name is not a member that is absent; `NotAList` for a segment that is no
/// position in a list; `Scalar` for a path that goes on into a number or a string; and
/// what the family refuses.
pub fn select_in<'v, F: Family>(
    family: &F,
    tree: &'v Value,
    path: &str,
) -> Result<Vec<&'v Value>, Error> {
    let places = walk(family, tree, path, None)?;
    let values = places.into_iter().filter_map(|place| place.value);
    Ok(values.filter(|value| !value.is_null()).collect())
}

/// [`select_in`], with how each value is written at its place: what a comparison with a
/// value that someone wrote brings that value to, with [`written_as`](super::written_as).
///
/// # Errors
///
/// Those of [`select_in`].
pub fn select_forms_in<'v, F: Family>(
    family: &F,
    tree: &'v Value,
    path: &str,
) -> Result<Vec<(&'v Value, Form)>, Error> {
    let places = walk(family, tree, path, None)?;
    let selected = places.into_iter().filter_map(|place| {
        let value = place.value.filter(|value| !value.is_null())?;
        Some((value, family.form(&place.shape, value)))
    });
    Ok(selected.collect())
}

/// The values at `path` in a tree of objects and lists: [`select_in`] for [`Plain`].
///
/// ```
/// use oxirush_utils::tree;
/// use serde_json::json;
///
/// let tree = json!({"slices": [{"sst": 1}, {"sst": 2, "sd": "0000ff"}], "t3512": null});
/// assert_eq!(tree::select(&tree, "/slices/*/sst")?, [&json!(1), &json!(2)]);
/// assert_eq!(tree::select(&tree, "/Slices/1/SD")?, [&json!("0000ff")]);
///
/// // What is not there selects nothing, and a name that the value has not is an error
/// assert!(tree::select(&tree, "/slices/5/sst")?.is_empty());
/// assert!(tree::select(&tree, "/t3512/value")?.is_empty());
/// assert!(tree::select(&tree, "/slices/0/sts").is_err());
/// # Ok::<(), tree::Error>(())
/// ```
///
/// # Errors
///
/// Those of [`select_in`].
pub fn select<'v>(tree: &'v Value, path: &str) -> Result<Vec<&'v Value>, Error> {
    select_in(&Plain, tree, path)
}

// ── Paths ───────────────────────────────────────────────────────────────────

/// Each value of a tree of the family `family` with the path that selects it, in the
/// order of the tree.
///
/// A value is a number, a string, a boolean, an empty object, or a list that has neither
/// an object nor a list: such a list is one value. A member or an entry that is `null`
/// is not listed, as it is not selected. What is nested deeper than a path has segments, 64, is one value at the
/// deepest path, so every path that is listed is one that [`select_in`] takes.
///
/// The time grows with the size of the tree and no faster: a family names the entries
/// of a list once for the list.
pub fn paths_in<F: Family>(family: &F, tree: &Value) -> Vec<(String, Value)> {
    let root = family.root();
    // The root is the tree itself, or short for a place in it that the family marks.
    let (mut path, depth) = match root.name.is_empty() || !root.stands_for.is_empty() {
        true => (String::new(), 0),
        false => (format!("/{}", root.name), 1),
    };
    let mut found = Vec::new();
    let shape = family.start(tree, true);
    // What is null is not there.
    if !tree.is_null() {
        list(family, tree, &shape, &mut path, depth, &mut found);
    }
    found
}

/// Each value of a tree of objects and lists with the path that selects it:
/// [`paths_in`] for [`Plain`].
///
/// ```
/// use oxirush_utils::tree;
/// use serde_json::json;
///
/// let tree = json!({"cause": 3, "slices": [{"sst": 1}], "tac": [0, 0, 1], "t3512": null});
/// let paths = tree::paths(&tree);
/// assert_eq!(paths, [
///     ("/cause".to_string(), json!(3)),
///     ("/slices/0/sst".to_string(), json!(1)),
///     ("/tac".to_string(), json!([0, 0, 1])),
/// ]);
/// ```
pub fn paths(tree: &Value) -> Vec<(String, Value)> {
    paths_in(&Plain, tree)
}

/// List `child`, which is under `segment` in the value at `path`.
fn inside<F: Family>(
    family: &F,
    segment: &str,
    child: &Value,
    shape: &F::Shape,
    path: &mut String,
    depth: usize,
    found: &mut Vec<(String, Value)>,
) {
    let length = path.len();
    path.push('/');
    path.push_str(&escape(segment));
    list(family, child, shape, path, depth + 1, found);
    path.truncate(length);
}

/// List `value`, which is at `path`, and what it holds.
fn list<F: Family>(
    family: &F,
    value: &Value,
    shape: &F::Shape,
    path: &mut String,
    depth: usize,
    found: &mut Vec<(String, Value)>,
) {
    match value {
        // A path has no more segments than this.
        _ if depth >= MAX_SEGMENTS => found.push((path.clone(), value.clone())),
        Value::Object(members) if !members.is_empty() => {
            for (name, member) in members.iter().filter(|(_, member)| !member.is_null()) {
                let of_member = family.member(shape, name, Some(member));
                match family.listed(shape, name) {
                    Listed::Member => inside(family, name, member, &of_member, path, depth, found),
                    Listed::As(other) => {
                        inside(family, other, member, &of_member, path, depth, found);
                    }
                    Listed::Hidden => {}
                    Listed::Through => list(family, member, &of_member, path, depth, found),
                    Listed::Root => {
                        let mut rooted = format!("/{}", family.root().name);
                        let before = found.len();
                        list(family, member, &of_member, &mut rooted, 1, found);
                        // The member that the root is short for is there, with nothing
                        // in it.
                        if found.len() == before {
                            found.push((rooted, member.clone()));
                        }
                    }
                }
            }
        }
        Value::Array(entries) if entries.iter().any(|e| e.is_object() || e.is_array()) => {
            // Once for the list, whatever its length.
            let names = family.entry_names(shape, entries);
            let there = entries
                .iter()
                .enumerate()
                .filter(|(_, entry)| !entry.is_null());
            for (index, entry) in there {
                let of_entry = family.entry(shape, Some(entry));
                let named = names
                    .as_ref()
                    .and_then(|names| names.get(index)?.as_deref());
                match named {
                    Some(name) => inside(family, name, entry, &of_entry, path, depth, found),
                    None => {
                        let position = index.to_string();
                        inside(family, &position, entry, &of_entry, path, depth, found);
                    }
                }
            }
        }
        plain => found.push((path.clone(), plain.clone())),
    }
}

// ── Edits ───────────────────────────────────────────────────────────────────

/// Give the member or the entry at `path` the value `value`, in a tree of the family
/// `family`. `null` takes it out, as [`remove_in`] does.
///
/// A member that an object does not have is added to it, at its end, unless the family
/// says that the value cannot have it.
///
/// # Errors
///
/// Those of [`select_in`], and the kind `NothingSelected` for a path that selects
/// nothing. After an error the tree is as it was: an edit is done on a copy, whole or
/// not at all.
pub fn set_in<F: Family>(
    family: &F,
    tree: &mut Value,
    path: &str,
    value: Value,
) -> Result<(), Error> {
    match value {
        Value::Null => edit(family, tree, path, Edit::Remove, None),
        value => edit(family, tree, path, Edit::Set, Some(value)),
    }
}

/// Take the member or the entry at `path` out of a tree of the family `family`. The
/// entries after one that is taken out move up.
///
/// # Errors
///
/// Those of [`set_in`]: a member that is not there, or that is `null`, is nothing to
/// take out.
pub fn remove_in<F: Family>(family: &F, tree: &mut Value, path: &str) -> Result<(), Error> {
    edit(family, tree, path, Edit::Remove, None)
}

/// Add `value` to a list of a tree of the family `family`: before the entry at `path`,
/// or at the end of the list for a path that ends with `-`.
///
/// # Errors
///
/// Those of [`set_in`], and the kind `NotAList` for a path that ends at a member.
pub fn insert_in<F: Family>(
    family: &F,
    tree: &mut Value,
    path: &str,
    value: Value,
) -> Result<(), Error> {
    edit(family, tree, path, Edit::Insert, Some(value))
}

/// [`set_in`] for [`Plain`]: a tree of objects and lists.
///
/// ```
/// use oxirush_utils::tree;
/// use serde_json::json;
///
/// let mut tree = json!({"slices": [{"sst": 1}]});
/// tree::set(&mut tree, "/slices/0/sd", json!("0000ff"))?;
/// tree::insert(&mut tree, "/slices/-", json!({"sst": 2}))?;
/// tree::insert(&mut tree, "/slices/0", json!({"sst": 0}))?;
/// tree::remove(&mut tree, "/slices/1/sst")?;
/// assert_eq!(tree, json!({"slices": [{"sst": 0}, {"sd": "0000ff"}, {"sst": 2}]}));
///
/// // An edit that selects nothing is an error, and the tree is as it was
/// assert!(tree::set(&mut tree, "/slices/7/sst", json!(1)).is_err());
/// # Ok::<(), tree::Error>(())
/// ```
///
/// # Errors
///
/// Those of [`set_in`].
pub fn set(tree: &mut Value, path: &str, value: Value) -> Result<(), Error> {
    set_in(&Plain, tree, path, value)
}

/// [`remove_in`] for [`Plain`].
///
/// # Errors
///
/// Those of [`remove_in`].
pub fn remove(tree: &mut Value, path: &str) -> Result<(), Error> {
    remove_in(&Plain, tree, path)
}

/// [`insert_in`] for [`Plain`].
///
/// # Errors
///
/// Those of [`insert_in`].
pub fn insert(tree: &mut Value, path: &str, value: Value) -> Result<(), Error> {
    insert_in(&Plain, tree, path, value)
}

/// The keys of a place, from the tree.
fn keys(at: &Option<Rc<Link>>) -> Vec<Key> {
    let mut keys = Vec::new();
    let mut link = at.as_deref();
    while let Some(Link { key, before }) = link {
        keys.push(key.clone());
        link = before.as_deref();
    }
    keys.reverse();
    keys
}

fn edit<F: Family>(
    family: &F,
    tree: &mut Value,
    path: &str,
    operation: Edit,
    value: Option<Value>,
) -> Result<(), Error> {
    let nothing = || Error::new(ErrorKind::NothingSelected, path, "the path selects nothing");
    // The places of the edit, as keys: nothing of the tree is borrowed after this.
    let places: Vec<(Vec<Key>, Removal)> = {
        let places = walk(family, tree, path, Some(operation))?;
        let edited = places.iter().filter(|place| match operation {
            Edit::Set => true,
            // What is not there is not taken out, and nothing is inserted before it.
            Edit::Remove | Edit::Insert => {
                place.value.is_some_and(|value| !value.is_null())
                    || matches!(place.at.as_deref(), Some(Link { key: Key::End, .. }))
            }
        });
        let places = edited.map(|place| (keys(&place.at), family.removal(&place.shape)));
        places.collect()
    };
    if places.is_empty() {
        return Err(nothing());
    }
    if places.iter().any(|(keys, _)| keys.is_empty()) {
        let reason = "an edit takes a path into the tree, and this is the path of the tree itself";
        return Err(Error::new(ErrorKind::Path, path, reason));
    }
    let mut edited = tree.clone();
    // The last place first: an entry that goes, or comes, moves those after it alone.
    for (keys, removal) in places.iter().rev() {
        apply(&mut edited, keys, operation, value.as_ref(), *removal, path)?;
    }
    for (keys, _) in &places {
        let done = family.edited(&mut edited, keys, operation);
        done.map_err(|error| error.located(path))?;
    }
    *tree = edited;
    Ok(())
}

/// Do `operation` at `keys` in `tree`.
fn apply(
    tree: &mut Value,
    keys: &[Key],
    operation: Edit,
    value: Option<&Value>,
    removal: Removal,
    path: &str,
) -> Result<(), Error> {
    let gone = || Error::new(ErrorKind::NothingSelected, path, "the path selects nothing");
    let Some((last, before)) = keys.split_last() else {
        return Err(gone());
    };
    let mut holder = tree;
    for key in before {
        // A member that is added goes into an object, which a place that is null
        // becomes.
        if holder.is_null() && matches!(key, Key::Name(_)) {
            *holder = Value::Object(Map::new());
        }
        holder = match (key, holder) {
            (Key::Name(name), Value::Object(members)) => {
                members.entry(name.as_str()).or_insert(Value::Null)
            }
            (Key::Position(position), Value::Array(entries)) => {
                entries.get_mut(*position).ok_or_else(gone)?
            }
            _ => return Err(gone()),
        };
    }
    if holder.is_null() && matches!(last, Key::Name(_)) && operation == Edit::Set {
        *holder = Value::Object(Map::new());
    }
    let not_a_list = || {
        let reason = "insert adds to a list, and the path ends at a member";
        Error::new(ErrorKind::NotAList, path, reason)
    };
    match (last, holder, operation) {
        (Key::Name(name), Value::Object(members), Edit::Set) => {
            members.insert(name.clone(), value.cloned().unwrap_or_default());
        }
        (Key::Name(name), Value::Object(members), Edit::Remove) => match removal {
            Removal::Take => {
                members.shift_remove(name.as_str()).ok_or_else(gone)?;
            }
            Removal::Null => *members.get_mut(name.as_str()).ok_or_else(gone)? = Value::Null,
        },
        (Key::Name(_), _, Edit::Insert) => return Err(not_a_list()),
        (Key::Position(position), Value::Array(entries), _) if *position < entries.len() => {
            match operation {
                Edit::Set => entries[*position] = value.cloned().unwrap_or_default(),
                Edit::Remove => {
                    entries.remove(*position);
                }
                Edit::Insert => entries.insert(*position, value.cloned().unwrap_or_default()),
            }
        }
        (Key::End, Value::Array(entries), Edit::Insert) => {
            entries.push(value.cloned().unwrap_or_default());
        }
        _ => return Err(gone()),
    }
    Ok(())
}
