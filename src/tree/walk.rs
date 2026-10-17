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
use super::value::{Map, Value};
use super::{Error, ErrorKind};
use alloc::format;
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

// ── What a family says ──────────────────────────────────────────────────────

/// The first segment of the paths of a family: [`Root::new`] makes one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
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

impl<'a> Root<'a> {
    /// No root: a path starts at the tree itself.
    pub const NONE: Root<'static> = Root {
        name: "",
        stands_for: &[],
        required: false,
    };

    /// The root `name`, which is short for the segments `stands_for` from the tree, and
    /// which a path has to start with when it is `required`.
    pub const fn new(name: &'a str, stands_for: &'a [&'a str], required: bool) -> Self {
        Self {
            name,
            stands_for,
            required,
        }
    }
}

/// What an edit does at its path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Edit {
    /// Give a member or an entry a value.
    Set,
    /// Take a member or an entry out.
    Remove,
    /// Add an entry to a list.
    Insert,
    /// Show a value on the way of the path as the family opens it: see [`open_in`].
    Open,
}

/// One step from the tree to a place of it: the name of a member as the tree has it, the
/// position of an entry of a list, or the end of a list, where `insert` adds.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
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
    /// The keys of each place that the family opens, and the value that it opens there.
    opened: Vec<(Option<Rc<Link>>, Value)>,
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

    /// The end of the list at the place, where [`insert_in`] adds an entry: what `-`
    /// selects, as the last segment of an insertion.
    pub fn end(&mut self, shape: S) {
        let at = self.link(|| Key::End);
        self.found.push(Place {
            value: None,
            shape,
            at,
        });
    }

    /// [`absent`](Self::absent), unless a place without a value of the same shape is
    /// there already: a family that selects by its types, where a tree has no value,
    /// adds each type once.
    pub fn absent_once(&mut self, name: &str, shape: S)
    where
        S: PartialEq,
    {
        let known = |place: &Place<'v, S>| place.value.is_none() && place.shape == shape;
        if !self.found.iter().any(known) {
            self.absent(name, shape);
        }
    }

    /// The same place again, as `place`, with what the family now knows of it: for a
    /// segment that says what the place is and does not go into it.
    pub fn stay(&mut self, place: &Place<'v, S>, shape: S) {
        let (value, at) = (place.value, place.at.clone());
        self.found.push(Place { value, shape, at });
    }

    /// Show the value at `place` as `value`, which the family opens from it, where the
    /// edit is [`Edit::Open`]: the value takes the place of the one there once the
    /// segment is taken at every place, and the path is taken again from the tree. See
    /// [`open_in`].
    pub fn open(&mut self, place: &Place<'v, S>, value: Value) {
        self.opened.push((place.at.clone(), value));
    }

    /// Take the segment in the member `name` of `place` as if the name stood before it,
    /// as [`Step::Through`] does, with the family's own rule there: `step` adds what the
    /// segment selects in that member, which the places that it adds are under. What
    /// `step` returns is returned, so that a family words what the member refuses as
    /// its own refusal. The member is a place without a value when `place` has no such
    /// member.
    pub fn through<R>(
        &mut self,
        place: &Place<'v, S>,
        name: &str,
        shape: S,
        step: impl FnOnce(&Place<'v, S>, &mut Self) -> R,
    ) -> R {
        let members = place.value.and_then(Value::as_object);
        let held = members.and_then(|members| {
            let name = key(members, name)?;
            Some((name, &members[name]))
        });
        let at = self.link(|| Key::Name(held.map_or(name, |(name, _)| name).to_string()));
        let inside = Place {
            value: held.map(|(_, value)| value),
            shape,
            at,
        };
        let from = core::mem::replace(&mut self.from, inside.at.clone());
        let taken = step(&inside, self);
        self.from = from;
        taken
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
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
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
/// use oxirush_utils::{tree, tree::{Family, Next, Place, Root, Step, Value}};
///
/// /// A list of `{"id": .., "value": ..}` whose entries are selected by their `id`.
/// struct ById;
///
/// impl Family for ById {
///     type Shape = ();
///
///     fn root(&self) -> Root<'_> {
///         Root::new("list", &[], true)
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
///         let named = |entry: &Value| entry["id"].as_str() == Some(segment);
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
/// let mut list = tree!([{"id": "cause", "value": 3}, {"id": "timer", "value": 60}]);
/// assert_eq!(tree::select_in(&ById, &list, "/list/timer/value")?, [&tree!(60)]);
/// tree::set_in(&ById, &mut list, "/list/cause/value", 7)?;
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

    /// Whether `null` at a place of `shape` is a value: the one value of a type that has
    /// no other, as the NULL of ASN.1, which a CHOICE has as an alternative. A tree is
    /// written so, `{"pLMNWide": null}`; elsewhere `null` is read as a member that is
    /// absent, and selects nothing.
    fn null_is_value(&self, shape: &Self::Shape) -> bool {
        let _ = shape;
        false
    }

    /// What else an edit changes: it is called for each place that was edited, with the
    /// tree as the edit left it, the keys to the place and what the family knew of the
    /// place when the path reached it. It cannot refuse the edit, which is done: a family
    /// refuses one in its [`step`](Self::step), which sees what the edit is
    /// ([`Next::edit`]) and whether the segment is the last, before anything of the tree
    /// changes.
    fn edited(&self, tree: &mut Value, at: &[Key], shape: &Self::Shape, edit: Edit) {
        let _ = (tree, at, shape, edit);
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
fn no_member(members: &Map, part: &str, path: &str) -> Error {
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
        None => Ok(()),
        Some(Value::Null) if !family.null_is_value(shape) => Ok(()),
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
                next.end(family.entry(shape, None));
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
        Step::Taken if !next.opened.is_empty() && next.edit != Some(Edit::Open) => {
            let reason = "the family opens a value where the edit is not to open one";
            Err(Error::new(ErrorKind::Refused, path, reason))
        }
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
    match walk_to_open(family, tree, path, edit)? {
        Walked::Places(places) => Ok(places),
        // Only an edit that opens is told that a value is opened.
        Walked::Open(..) => Err(Error::new(
            ErrorKind::Refused,
            path,
            "the family opens a value where the edit is not to open one",
        )),
    }
}

/// Where a walk ends: at the places that the path selects, or at the first segment where
/// the family opens values, with the places that it opens there and their values.
enum Walked<'v, S> {
    Places(Vec<Place<'v, S>>),
    Open(Vec<(Vec<Key>, Value)>),
}

/// The places that `path` selects in `tree`, or the places of the first segment on the
/// way where the family opens values.
fn walk_to_open<'v, F: Family>(
    family: &F,
    tree: &'v Value,
    path: &str,
    edit: Option<Edit>,
) -> Result<Walked<'v, F::Shape>, Error> {
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
            opened: Vec::new(),
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
            if next.found.len() + next.opened.len() > MAX_SELECTED {
                let reason = format!("a path selects {MAX_SELECTED} values at most");
                return Err(Error::new(ErrorKind::Limit, path, reason));
            }
        }
        // What the family opens at this segment, at every place, is opened at once.
        if !next.opened.is_empty() {
            let opened = next.opened.into_iter();
            return Ok(Walked::Open(
                opened.map(|(at, value)| (keys(&at), value)).collect(),
            ));
        }
        // What one place refuses, another may have: an error when none has it.
        if let (true, Some(refusal)) = (next.found.is_empty(), refusal) {
            return Err(refusal);
        }
        places = next.found;
    }
    Ok(Walked::Places(places))
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
/// The kinds `Path` and `Limit` of [`segments`], and `Limit` for more than 65,536 values;
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
    let values = places.into_iter().filter_map(|place| there(family, &place));
    Ok(values.collect())
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
        let value = there(family, &place)?;
        Some((value, family.form(&place.shape, value)))
    });
    Ok(selected.collect())
}

/// The value of a place that is there: one that is not `null`, or `null` where the family
/// says that it is a value.
fn there<'v, F: Family>(family: &F, place: &Place<'v, F::Shape>) -> Option<&'v Value> {
    let value = place.value?;
    (!value.is_null() || family.null_is_value(&place.shape)).then_some(value)
}

/// The places that `path` selects in a tree of the family `family`, in the order of the
/// tree: each with its value, or with none where the family says that one could be
/// ([`Next::absent`]), and with what the family knows of it. [`select_in`] gives the
/// values of those that have one; a family that reads its own tables at the places
/// takes them all.
///
/// # Errors
///
/// Those of [`select_in`].
pub fn places_in<'v, F: Family>(
    family: &F,
    tree: &'v Value,
    path: &str,
) -> Result<Vec<Place<'v, F::Shape>>, Error> {
    walk(family, tree, path, None)
}

/// The values at `path` in a tree of objects and lists: [`select_in`] for [`Plain`].
///
/// ```
/// use oxirush_utils::tree;
///
/// let tree = tree!({"slices": [{"sst": 1}, {"sst": 2, "sd": "0000ff"}], "t3512": null});
/// assert_eq!(tree::select(&tree, "/slices/*/sst")?, [&tree!(1), &tree!(2)]);
/// assert_eq!(tree::select(&tree, "/Slices/1/SD")?, [&tree!("0000ff")]);
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
///
/// let tree = tree!({"cause": 3, "slices": [{"sst": 1}], "tac": [0, 0, 1], "t3512": null});
/// let paths = tree::paths(&tree);
/// assert_eq!(paths, [
///     ("/cause".to_string(), tree!(3)),
///     ("/slices/0/sst".to_string(), tree!(1)),
///     ("/tac".to_string(), tree!([0, 0, 1])),
/// ]);
/// ```
pub fn paths(tree: &Value) -> Vec<(String, Value)> {
    paths_in(&Plain, tree)
}

/// Whether a value is an object or a list with nothing in it.
fn is_empty(value: &Value) -> bool {
    match value {
        Value::Object(members) => members.is_empty(),
        Value::Array(entries) => entries.is_empty(),
        _ => false,
    }
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
            for (name, member) in members.iter() {
                let of_member = family.member(shape, name, Some(member));
                if member.is_null() && !family.null_is_value(&of_member) {
                    continue;
                }
                match family.listed(shape, name) {
                    Listed::Member => inside(family, name, member, &of_member, path, depth, found),
                    Listed::As(other) => {
                        inside(family, other, member, &of_member, path, depth, found);
                    }
                    Listed::Hidden => {}
                    // A member that is listed in the place of its holder and has nothing
                    // in it lists nothing: its own value is not one at that place.
                    Listed::Through if is_empty(member) => {}
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
            for (index, entry) in entries.iter().enumerate() {
                let of_entry = family.entry(shape, Some(entry));
                if entry.is_null() && !family.null_is_value(&of_entry) {
                    continue;
                }
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
/// nothing. After an error the tree is as it was: an edit is done in the place of the
/// tree, which keeps what it changes and puts it back when a place cannot take the
/// edit, so that it is done whole or not at all, and nothing of the tree is copied.
pub fn set_in<F: Family>(
    family: &F,
    tree: &mut Value,
    path: &str,
    value: impl Into<Value>,
) -> Result<(), Error> {
    match value.into() {
        Value::Null if !null_is_value_at(family, tree, path) => {
            edit(family, tree, path, Edit::Remove, None)
        }
        value => edit(family, tree, path, Edit::Set, Some(value)),
    }
}

/// Whether `path` selects places where `null` is a value, and only such places: `null`
/// is then set there, where elsewhere it takes a member out.
fn null_is_value_at<F: Family>(family: &F, tree: &Value, path: &str) -> bool {
    match walk(family, tree, path, Some(Edit::Set)) {
        Ok(places) => {
            !places.is_empty()
                && places
                    .iter()
                    .all(|place| family.null_is_value(&place.shape))
        }
        Err(_) => false,
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
    value: impl Into<Value>,
) -> Result<(), Error> {
    edit(family, tree, path, Edit::Insert, Some(value.into()))
}

/// [`set_in`] for [`Plain`]: a tree of objects and lists.
///
/// ```
/// use oxirush_utils::tree;
///
/// let mut tree = tree!({"slices": [{"sst": 1}]});
/// tree::set(&mut tree, "/slices/0/sd", tree!("0000ff"))?;
/// tree::insert(&mut tree, "/slices/-", tree!({"sst": 2}))?;
/// tree::insert(&mut tree, "/slices/0", tree!({"sst": 0}))?;
/// tree::remove(&mut tree, "/slices/1/sst")?;
/// assert_eq!(tree, tree!({"slices": [{"sst": 0}, {"sd": "0000ff"}, {"sst": 2}]}));
///
/// // An edit that selects nothing is an error, and the tree is as it was
/// assert!(tree::set(&mut tree, "/slices/7/sst", tree!(1)).is_err());
/// # Ok::<(), tree::Error>(())
/// ```
///
/// # Errors
///
/// Those of [`set_in`].
pub fn set(tree: &mut Value, path: &str, value: impl Into<Value>) -> Result<(), Error> {
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
pub fn insert(tree: &mut Value, path: &str, value: impl Into<Value>) -> Result<(), Error> {
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
    let places: Vec<(Vec<Key>, Removal, F::Shape)> = {
        let places = walk(family, tree, path, Some(operation))?;
        let edited = places.iter().filter(|place| match operation {
            Edit::Set | Edit::Open => true,
            // What is not there is not taken out, and nothing is inserted before it.
            Edit::Remove | Edit::Insert => {
                there(family, place).is_some()
                    || matches!(place.at.as_deref(), Some(Link { key: Key::End, .. }))
            }
        });
        let places = edited.map(|place| {
            let removal = family.removal(&place.shape);
            (keys(&place.at), removal, place.shape.clone())
        });
        places.collect()
    };
    if places.is_empty() {
        return Err(nothing());
    }
    if places.iter().any(|(keys, ..)| keys.is_empty()) {
        let reason = "an edit takes a path into the tree, and this is the path of the tree itself";
        return Err(Error::new(ErrorKind::Path, path, reason));
    }
    // The last place first: an entry that goes, or comes, moves those after it alone.
    let mut undo = Vec::new();
    for (keys, removal, _) in places.iter().rev() {
        let done = apply(
            tree,
            keys,
            operation,
            value.as_ref(),
            *removal,
            path,
            &mut undo,
        );
        if let Err(error) = done {
            take_back(tree, undo);
            return Err(error);
        }
    }
    for (keys, _, shape) in &places {
        family.edited(tree, keys, shape, operation);
    }
    Ok(())
}

/// Show the values on the way of `path` in a tree of the family `family` as the family
/// opens them: octets that carry a value of a type, say, which the tree then shows as
/// that value. Where the [`step`](Family::step) of the family opens a value
/// ([`Next::open`]) for an edit [`Edit::Open`], the value that it gives takes the place
/// of the one there, and the path is taken again from the tree. Nothing else changes,
/// and a path that selects nothing is no error.
///
/// ```
/// use oxirush_utils::{tree, tree::{Edit, Family, Next, Place, Step}};
///
/// /// Text that holds a number, which is opened as the number.
/// struct Numbers;
///
/// impl Family for Numbers {
///     type Shape = ();
///
///     fn step<'v>(
///         &self,
///         place: &Place<'v, ()>,
///         segment: &str,
///         next: &mut Next<'v, ()>,
///     ) -> Result<Step<()>, tree::Error> {
///         match (place.value().and_then(|value| value.as_str()), segment, next.edit()) {
///             (Some(text), "number", Some(Edit::Open)) => match text.parse::<u64>() {
///                 Ok(number) => {
///                     next.open(place, tree!({"number": number}));
///                     Ok(Step::Taken)
///                 }
///                 Err(_) => Ok(Step::Plain),
///             },
///             _ => Ok(Step::Plain),
///         }
///     }
/// }
///
/// let mut tree = tree!({"timers": ["60", "3600"]});
/// tree::open_in(&Numbers, &mut tree, "/timers/*/number")?;
/// assert_eq!(tree, tree!({"timers": [{"number": 60}, {"number": 3600}]}));
/// assert_eq!(tree::select_in(&Numbers, &tree, "/timers/1/number")?, [&tree!(3600)]);
/// # Ok::<(), tree::Error>(())
/// ```
///
/// # Errors
///
/// Those of [`select_in`]: the tree is then as it was, with nothing opened.
pub fn open_in<F: Family>(family: &F, tree: &mut Value, path: &str) -> Result<(), Error> {
    let mut undo = Vec::new();
    // Each turn opens what the family opens at one segment, at each of its places, and
    // the next takes the path again from the tree to the segments after it: a path has
    // as many turns as segments at most, and a turn walks the tree once.
    for _ in 0..=MAX_SEGMENTS {
        let walked = walk_to_open(family, tree, path, Some(Edit::Open));
        let opened = match walked {
            Ok(Walked::Places(_)) => return Ok(()),
            Ok(Walked::Open(opened)) => opened,
            Err(error) => {
                take_back(tree, undo);
                return Err(error);
            }
        };
        // The last first, as an edit does: no place is in another one.
        for (at, value) in opened.into_iter().rev() {
            let Some(place) = at_mut(tree, &at) else {
                take_back(tree, undo);
                let reason = "the family opens a value at a place that the tree does not have";
                return Err(Error::new(ErrorKind::Refused, path, reason));
            };
            undo.push(Undo::Was(at, core::mem::replace(place, value)));
        }
    }
    take_back(tree, undo);
    let reason = format!("a path opens values at {MAX_SEGMENTS} segments at most");
    Err(Error::new(ErrorKind::Limit, path, reason))
}

/// What an edit changed in a tree, to put it back when a later place cannot take it.
#[derive(Debug)]
enum Undo {
    /// The value at these keys was this one.
    Was(Vec<Key>, Value),
    /// The member at these keys was added.
    Added(Vec<Key>),
    /// The object at these keys had this member at this position.
    Taken(Vec<Key>, usize, String, Value),
    /// The list at these keys had this entry at this position.
    Removed(Vec<Key>, usize, Value),
    /// The list at these keys took an entry at this position.
    Inserted(Vec<Key>, usize),
}

/// The value at `keys` in `tree`.
fn at_mut<'t>(tree: &'t mut Value, keys: &[Key]) -> Option<&'t mut Value> {
    let mut holder = tree;
    for key in keys {
        holder = match (key, holder) {
            (Key::Name(name), Value::Object(members)) => members.get_mut(name.as_str())?,
            (Key::Position(position), Value::Array(entries)) => entries.get_mut(*position)?,
            _ => return None,
        };
    }
    Some(holder)
}

/// Put back what `undo` says an edit changed, from the last change: each one finds the
/// tree as the change left it.
fn take_back(tree: &mut Value, undo: Vec<Undo>) {
    for done in undo.into_iter().rev() {
        match done {
            Undo::Was(keys, was) => {
                if let Some(value) = at_mut(tree, &keys) {
                    *value = was;
                }
            }
            Undo::Added(keys) => {
                if let Some((Key::Name(name), holder)) = keys.split_last()
                    && let Some(Value::Object(members)) = at_mut(tree, holder)
                {
                    members.shift_remove(name.as_str());
                }
            }
            Undo::Taken(keys, position, name, was) => {
                if let Some(Value::Object(members)) = at_mut(tree, &keys) {
                    members.shift_insert(position, name, was);
                }
            }
            Undo::Removed(keys, position, was) => {
                if let Some(Value::Array(entries)) = at_mut(tree, &keys) {
                    entries.insert(position, was);
                }
            }
            Undo::Inserted(keys, position) => {
                if let Some(Value::Array(entries)) = at_mut(tree, &keys) {
                    entries.remove(position);
                }
            }
        }
    }
}

/// Do `operation` at `keys` in `tree`, and keep in `undo` what it changes.
fn apply(
    tree: &mut Value,
    keys: &[Key],
    operation: Edit,
    value: Option<&Value>,
    removal: Removal,
    path: &str,
    undo: &mut Vec<Undo>,
) -> Result<(), Error> {
    let gone = || Error::new(ErrorKind::NothingSelected, path, "the path selects nothing");
    let Some((last, before)) = keys.split_last() else {
        return Err(gone());
    };
    let mut holder = tree;
    for (depth, key) in before.iter().enumerate() {
        // A member that is added goes into an object, which a place that is null
        // becomes.
        if holder.is_null() && matches!(key, Key::Name(_)) {
            undo.push(Undo::Was(before[..depth].to_vec(), Value::Null));
            *holder = Value::Object(Map::new());
        }
        holder = match (key, holder) {
            (Key::Name(name), Value::Object(members)) => {
                if !members.contains_key(name.as_str()) {
                    undo.push(Undo::Added(before[..=depth].to_vec()));
                }
                members.entry(name.as_str()).or_insert(Value::Null)
            }
            (Key::Position(position), Value::Array(entries)) => {
                entries.get_mut(*position).ok_or_else(gone)?
            }
            _ => return Err(gone()),
        };
    }
    if holder.is_null() && matches!(last, Key::Name(_)) && operation == Edit::Set {
        undo.push(Undo::Was(before.to_vec(), Value::Null));
        *holder = Value::Object(Map::new());
    }
    let not_a_list = || {
        let reason = "insert adds to a list, and the path ends at a member";
        Error::new(ErrorKind::NotAList, path, reason)
    };
    let written = || value.cloned().unwrap_or_default();
    match (last, holder, operation) {
        (Key::Name(name), Value::Object(members), Edit::Set) => {
            match members.insert(name.clone(), written()) {
                Some(was) => undo.push(Undo::Was(keys.to_vec(), was)),
                None => undo.push(Undo::Added(keys.to_vec())),
            }
        }
        (Key::Name(name), Value::Object(members), Edit::Remove) => match removal {
            Removal::Take => {
                let position = members.keys().position(|known| known == name);
                let position = position.ok_or_else(gone)?;
                let was = members.shift_remove(name.as_str()).ok_or_else(gone)?;
                undo.push(Undo::Taken(before.to_vec(), position, name.clone(), was));
            }
            Removal::Null => {
                let member = members.get_mut(name.as_str()).ok_or_else(gone)?;
                undo.push(Undo::Was(keys.to_vec(), core::mem::take(member)));
            }
        },
        (Key::Name(_), _, Edit::Insert) => return Err(not_a_list()),
        (Key::Position(position), Value::Array(entries), _) if *position < entries.len() => {
            match operation {
                Edit::Set => {
                    let was = core::mem::replace(&mut entries[*position], written());
                    undo.push(Undo::Was(keys.to_vec(), was));
                }
                Edit::Remove => {
                    let was = entries.remove(*position);
                    undo.push(Undo::Removed(before.to_vec(), *position, was));
                }
                Edit::Insert => {
                    entries.insert(*position, written());
                    undo.push(Undo::Inserted(before.to_vec(), *position));
                }
                Edit::Open => return Err(gone()),
            }
        }
        (Key::End, Value::Array(entries), Edit::Insert) => {
            undo.push(Undo::Inserted(before.to_vec(), entries.len()));
            entries.push(written());
        }
        _ => return Err(gone()),
    }
    Ok(())
}
