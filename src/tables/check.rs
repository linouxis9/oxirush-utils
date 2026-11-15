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

//! What a scope breaks of its table: the IEs that it needs, may have and may repeat on
//! the interface of the context, the IEs that it keeps as octets, and the conditions that
//! it shows, each a [`TableFinding`] that the protocol makes its deviation, with its clause
//! and the Cause of the answer ([`Checking`]).

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::{Presence, Protocol, Raw, Row, Scope, Table, kept_positions, reread, value_positions};
use crate::fields::{Path, Segment};

/// What a check of a table finds, by the key of its IE.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableFinding<K> {
    /// A mandatory IE is not there.
    MandatoryMissing(K),
    /// A mandatory IE is there, and its value does not decode as its type.
    MandatoryIncorrect(K),
    /// A conditional IE is not there where the scope shows that its condition holds.
    ConditionalMissing(K),
    /// A mandatory IE, or a conditional one where the scope shows that its condition
    /// holds, of a null length.
    NullLength(K),
    /// A conditional IE that is there, and whose value does not decode as its type.
    ConditionalIncorrect(K),
    /// An optional IE whose value does not decode as its type.
    Incorrect(K),
    /// An IE that is there more times than its table allows.
    Repeated(K),
    /// An IE that is there where the condition beside it, which the scope shows, has it
    /// absent.
    Excluded(K),
    /// An IE that its table has, but not on the interface.
    NotOnInterface(K),
    /// An IE in a response with a rejection Cause, which may not carry it.
    WithRejection(K),
    /// An IE whose Length is not the length of its content.
    InvalidLength(K),
}

/// What a protocol gives the checks of its tables: its context, its deviation, and the
/// rules that are its own.
pub trait Checking: Protocol {
    /// A deviation.
    type Deviation;
    /// Where a scope is checked: the interface, and what the message says of the answer.
    type Context: Copy;

    /// The interfaces of the context.
    fn interfaces(context: Self::Context) -> Self::Interfaces;
    /// The deviation of `finding` at `path` in a scope of `table`, with its clause and the
    /// Cause of the answer that the context gives it.
    fn deviation(
        path: &dyn core::fmt::Display,
        table: &'static Table<Self>,
        finding: TableFinding<Self::Key>,
        context: Self::Context,
    ) -> Self::Deviation;
    /// The IEs of a row that a message needs in the context.
    fn mandatory(row: &Row<Self>, context: Self::Context) -> usize;
    /// Whether a response of the context may not have `count` IEs of the row, as one that
    /// rejects the request.
    fn rejected(table: &'static str, row: &Row<Self>, count: usize, context: Self::Context)
    -> bool;
    /// What else the protocol checks of a row that has `count` values on the interface.
    fn check_row_more(
        table: &'static str,
        row: &Row<Self>,
        count: usize,
        path: &Path<'_>,
        context: Self::Context,
        found: &mut Vec<Self::Deviation>,
    );
    /// The deviation of an IE of no row of its table that the scope keeps as octets, which
    /// frames: `None` for one that breaks nothing.
    fn of_no_row(path: &str, ie: &Self::Raw) -> Option<Self::Deviation>;
    /// Whether an IE kept as octets has a null length.
    fn is_null(ie: &Self::Raw) -> bool;
    /// Whether the Length of an IE kept as octets is that of its content.
    fn frames(ie: &Self::Raw) -> bool;
    /// Let an IE kept as octets have the Length of its content.
    fn clear_length(ie: &mut Self::Raw);
    /// The name of an IE in the tree of a scope of `table`: the name of its row, of its
    /// type, or its number.
    fn name_of(key: Self::Key, table: Option<&Table<Self>>) -> String;
    /// The IE whose Length runs past the grouped IE of `row` whose content is `octets`:
    /// the first whose Length runs past the others, or the grouped IE itself where fewer
    /// octets than an IE header are left. `None` for an IE of another row, and for IEs
    /// that frame.
    fn unframed_within(row: &Row<Self>, ie: &Self::Raw) -> Option<Self::Key>;
}

/// The key of an IE of a scope's protocol.
type Key<S> = <<S as Scope>::P as Protocol>::Key;

/// The presence of a row on the interface of the context. One that the interface does not
/// have is taken as optional: its IEs are discarded, and what they break has no Cause.
pub fn presence<P: Checking>(row: &Row<P>, context: P::Context) -> P::Presence {
    row.on(P::interfaces(context))
        .and_then(|on| on.presence)
        .unwrap_or(P::Presence::OPTIONAL)
}

/// Check a row that has `count` values, or IEs kept as octets alone when 0: whether the
/// interface has it, whether a rejection may carry it, and whether the table allows that
/// many. Whether the interface has it.
#[inline]
pub fn check_row<P: Checking>(
    table: &'static Table<P>,
    row: &Row<P>,
    count: usize,
    path: &Path<'_>,
    context: P::Context,
    found: &mut Vec<P::Deviation>,
) -> bool {
    let rejected = P::rejected(table.id, row, count, context);
    let on = row.on(P::interfaces(context));
    // The receiver discards each IE of a row that is not on the interface.
    let off = on.is_none();
    let over = on.is_some_and(|on| !on.most.allows(count));
    if rejected || off || over {
        row_deviations(table, row, [rejected, off, over], path, context, found);
    }
    if !off {
        P::check_row_more(table.id, row, count, path, context, found);
    }
    !off
}

#[cold]
#[inline(never)]
fn row_deviations<P: Checking>(
    table: &'static Table<P>,
    row: &Row<P>,
    [rejected, off, over]: [bool; 3],
    path: &Path<'_>,
    context: P::Context,
    found: &mut Vec<P::Deviation>,
) {
    let here = Path::Row {
        parent: path,
        row: row.name,
        nth: None,
    };
    let key = row.ie_type;
    for (there, finding) in [
        (rejected, TableFinding::WithRejection(key)),
        (off, TableFinding::NotOnInterface(key)),
        (over, TableFinding::Repeated(key)),
    ] {
        if there {
            found.push(P::deviation(&here, table, finding, context));
        }
    }
}

/// Check the values of the row at `at` of a scope: the row, then each value, at its
/// position in the tree, which the IEs of the row that the scope keeps as octets share,
/// with `check` the check of a value at its path and in its presence.
#[inline(always)]
pub fn check_values<S: Scope, T>(
    scope: &S,
    at: usize,
    values: &[T],
    path: &Path<'_>,
    context: <S::P as Checking>::Context,
    found: &mut Vec<<S::P as Checking>::Deviation>,
    mut check: impl FnMut(
        &T,
        &Path<'_>,
        <S::P as Protocol>::Presence,
        &mut Vec<<S::P as Checking>::Deviation>,
    ),
) where
    S::P: Checking,
{
    if values.is_empty() {
        return;
    }
    let table = S::table();
    let row = &table.rows[at];
    if !check_row(table, row, values.len(), path, context, found) {
        // The receiver discards each IE of the row, whatever it holds.
        return;
    }
    // Of the values alone, their positions; with kept IEs of the row, theirs in the tree.
    let kept = scope.raw().iter().any(|ie| ie.key() == row.ie_type);
    let positions = match kept {
        false => Vec::new(),
        true => value_positions(scope, row),
    };
    for (nth, value) in values.iter().enumerate() {
        let nth = match kept {
            false => row.is_list().then_some(nth),
            true => Some(positions[nth]),
        };
        let here = Path::Row {
            parent: path,
            row: row.name,
            nth,
        };
        check(value, &here, presence(row, context), found);
    }
}

/// Check a scope against its table: its mandatory IEs on the interface, the IEs that it
/// keeps as octets (repeated, incorrect, of no row of the table, of no release), then
/// each row that has values, and each value, with `values` the check of its values.
pub fn check_scope<S: Scope>(
    scope: &S,
    path: &Path<'_>,
    context: <S::P as Checking>::Context,
    found: &mut Vec<<S::P as Checking>::Deviation>,
    values: impl FnOnce(
        &S,
        &Path<'_>,
        <S::P as Checking>::Context,
        &mut Vec<<S::P as Checking>::Deviation>,
    ),
) where
    S::P: Checking,
{
    let table = S::table();
    for row in table.rows {
        let needed = <S::P as Checking>::mandatory(row, context);
        if scope.count(row.ie_type) < needed {
            mandatory_missing(scope, row, needed, path, context, found);
        }
    }
    if !scope.raw().is_empty() {
        raw_deviations(scope, path, context, found);
    }
    values(scope, path, context, found);
}

/// A mandatory row that has fewer values than it needs: missing, there and incorrect, or
/// there with a null length, which its receiver takes. Of a row there once, the first IE
/// is the one handled.
#[cold]
#[inline(never)]
fn mandatory_missing<S: Scope>(
    scope: &S,
    row: &Row<S::P>,
    needed: usize,
    path: &Path<'_>,
    context: <S::P as Checking>::Context,
    found: &mut Vec<<S::P as Checking>::Deviation>,
) where
    S::P: Checking,
{
    let mut kept = scope.raw().iter().filter(|ie| ie.key() == row.ie_type);
    // An IE of the row that does not frame, or whose IEs do not, is told as such
    // ([`raw_deviations`]).
    if kept.clone().any(|ie| {
        !<S::P as Checking>::frames(ie) || <S::P as Checking>::unframed_within(row, ie).is_some()
    }) {
        return;
    }
    let null = |ie| <S::P as Checking>::is_null(ie);
    let (nulls, incorrect) = match row.is_list() {
        false => match kept.next() {
            Some(ie) if null(ie) => (1, false),
            Some(_) => (0, true),
            None => (0, false),
        },
        _ => kept.fold((0, false), |(nulls, incorrect), ie| match null(ie) {
            true => (nulls + 1, incorrect),
            false => (nulls, incorrect || !decodes(scope, ie)),
        }),
    };
    let at = Path::Row {
        parent: path,
        row: row.name,
        nth: None,
    };
    let finding = match () {
        _ if scope.count(row.ie_type) + nulls >= needed => TableFinding::NullLength(row.ie_type),
        // Missing where its table has it, or there and incorrect.
        _ if incorrect => TableFinding::MandatoryIncorrect(row.ie_type),
        _ => TableFinding::MandatoryMissing(row.ie_type),
    };
    found.push(<S::P as Checking>::deviation(
        &at,
        S::table(),
        finding,
        context,
    ));
}

/// Whether the octets of an IE kept as octets decode as the value of its row.
fn decodes<S: Scope>(scope: &S, ie: &<S::P as Protocol>::Raw) -> bool {
    scope.decodes(ie.key(), ie.content())
}

/// The IEs that a scope keeps as octets: of no row, repeated, or incorrect: a conditional
/// one semantically invalid, an optional one taken as absent. Of a row there once, the
/// first IE is the one handled, and the others are repetitions; an IE of a null length is
/// no error. A row of kept IEs alone is checked as one of values ([`check_row`]): on the
/// interface, and in a rejection.
#[cold]
#[inline(never)]
fn raw_deviations<S: Scope>(
    scope: &S,
    path: &Path<'_>,
    context: <S::P as Checking>::Context,
    found: &mut Vec<<S::P as Checking>::Deviation>,
) where
    S::P: Checking,
{
    let table = S::table();
    let raws = scope.raw();
    let positions = kept_positions(scope);
    // The rows of the kept IEs met so far: a few, whatever the number of the IEs.
    let mut handled: Vec<Key<S>> = Vec::new();
    for (at, ie) in raws.iter().enumerate() {
        let Some(row) = table.row_of(ie.key()) else {
            let here = kept_path(scope, &positions, at, path);
            if !<S::P as Checking>::frames(ie) {
                let finding = TableFinding::InvalidLength(ie.key());
                found.push(<S::P as Checking>::deviation(
                    &here, table, finding, context,
                ));
                continue;
            }
            if let Some(deviation) = <S::P as Checking>::of_no_row(&here, ie) {
                found.push(deviation);
            }
            continue;
        };
        let alone = !handled.contains(&row.ie_type) && scope.count(row.ie_type) == 0;
        if !handled.contains(&row.ie_type) {
            handled.push(row.ie_type);
        }
        if alone {
            check_row(table, row, 0, path, context, found);
        }
        let here = Path::Row {
            parent: path,
            row: row.name,
            nth: positions[at],
        };
        // One that does not frame, whose receiver frames the IEs after it amiss, of the
        // interface or not
        if !<S::P as Checking>::frames(ie) {
            let finding = TableFinding::InvalidLength(row.ie_type);
            found.push(<S::P as Checking>::deviation(
                &here, table, finding, context,
            ));
            continue;
        }
        // The receiver discards each IE of a row that is not on the interface, which its
        // row tells.
        if row.on(<S::P as Checking>::interfaces(context)).is_none() {
            continue;
        }
        let null = <S::P as Checking>::is_null(ie);
        let first = match row.is_list() {
            false => alone,
            true => !null && !decodes(scope, ie),
        };
        // A grouped IE that its receiver handles, whose IEs run past it.
        if let Some(inner) = first
            .then(|| <S::P as Checking>::unframed_within(row, ie))
            .flatten()
        {
            let finding = TableFinding::InvalidLength(inner);
            found.push(<S::P as Checking>::deviation(
                &here, table, finding, context,
            ));
            continue;
        }
        let finding = match (first, null) {
            // A null length says what its row allows it to.
            (true, true) => continue,
            // An incorrect mandatory IE is reported as such.
            (true, false)
                if <S::P as Checking>::mandatory(row, context) > scope.count(row.ie_type) =>
            {
                continue;
            }
            // One of a mandatory list beyond those that it needs, or a verifiable
            // conditional IE: semantically invalid, as one that decodes and breaks a rule.
            // An optional one is taken as absent.
            (true, false) => {
                let key = row.ie_type;
                let presence = presence(row, context);
                match (presence.is_mandatory(), presence.rank()) {
                    (true, _) => TableFinding::MandatoryIncorrect(key),
                    (false, rank) if rank < <S::P as Protocol>::Presence::OPTIONAL.rank() => {
                        TableFinding::ConditionalIncorrect(key)
                    }
                    _ => TableFinding::Incorrect(key),
                }
            }
            (false, true) if row.is_list() => continue,
            (false, _) => TableFinding::Repeated(row.ie_type),
        };
        found.push(<S::P as Checking>::deviation(
            &here, table, finding, context,
        ));
    }
}

/// The scope as its receiver reads its octets, which a check checks: `None` for a scope
/// that is as they read ([`Scope::plain`]). One whose octets do not decode, as the Length
/// of an IE that it keeps as octets does not frame, has those Lengths told in `found`
/// (`unframed`), and is read with them cleared. One that does not encode is taken with
/// them cleared.
pub fn as_read<S: Scope + Clone>(
    scope: &S,
    path: &Path<'_>,
    context: <S::P as Checking>::Context,
    found: &mut Vec<<S::P as Checking>::Deviation>,
) -> Option<S>
where
    S::P: Checking,
{
    if scope.plain() {
        return None;
    }
    if let Some(read) = reread(scope) {
        return Some(read);
    }
    unframed(scope, path, context, found);
    let mut cleared = scope.clone();
    for ie in cleared.raw_mut() {
        <S::P as Checking>::clear_length(ie);
    }
    Some(reread(&cleared).unwrap_or(cleared))
}

/// The IEs that a scope keeps as octets whose Length is not that of their content, which do
/// not frame, each at its path.
fn unframed<S: Scope>(
    scope: &S,
    path: &Path<'_>,
    context: <S::P as Checking>::Context,
    found: &mut Vec<<S::P as Checking>::Deviation>,
) where
    S::P: Checking,
{
    let positions = kept_positions(scope);
    for (at, ie) in scope.raw().iter().enumerate() {
        if !<S::P as Checking>::frames(ie) {
            let here = kept_path(scope, &positions, at, path);
            let finding = TableFinding::InvalidLength(ie.key());
            found.push(<S::P as Checking>::deviation(
                &here,
                S::table(),
                finding,
                context,
            ));
        }
    }
}

/// The path of the IE `at` that a scope keeps as octets: at the position of its row, or
/// by its name in the tree and its position among those of its type there.
pub fn kept_path<S: Scope>(
    scope: &S,
    positions: &[Option<usize>],
    at: usize,
    path: &Path<'_>,
) -> String
where
    S::P: Checking,
{
    let (table, raws) = (S::table(), scope.raw());
    let ie = &raws[at];
    if let Some(row) = table.row_of(ie.key()) {
        let here = Path::Row {
            parent: path,
            row: row.name,
            nth: positions[at],
        };
        return format!("{here}");
    }
    let name = <S::P as Checking>::name_of(ie.key(), Some(table));
    let same = |other: &&<S::P as Protocol>::Raw| other.key() == ie.key();
    let name = Segment(&name);
    match raws.iter().filter(same).count() {
        1 => format!("{path}/{name}"),
        _ => format!("{path}/{name}/{}", raws[..at].iter().filter(same).count()),
    }
}

/// The scope without the values of the rows of `off`, the bits of their positions: what a
/// receiver reads of it on an interface that lacks them. `None` for none.
pub fn taken<S: Scope + Clone>(scope: &S, off: u128) -> Option<S> {
    if off == 0 {
        return None;
    }
    let mut taken = scope.clone();
    let rows = S::table().rows.iter().enumerate();
    for (_, row) in rows.filter(|(at, _)| off >> at & 1 == 1) {
        taken.clear(row.ie_type);
    }
    Some(taken)
}

/// Whether each interface of the context has a row, which the condition beside it then
/// needs.
fn on_each<P: Checking>(row: &Row<P>, context: P::Context) -> bool {
    row.on(P::interfaces(context))
        .is_some_and(|on| on.presence.is_some())
}

/// A C row that is not there where the condition that its table writes beside it holds,
/// on the interface: missing, or of a null length. One that is there and incorrect is
/// answered as such.
pub fn condition_missing<S: Scope>(
    scope: &S,
    at: usize,
    path: &Path<'_>,
    context: <S::P as Checking>::Context,
    found: &mut Vec<<S::P as Checking>::Deviation>,
) where
    S::P: Checking,
{
    let table = S::table();
    let row = &table.rows[at];
    let mut kept = scope.raw().iter().filter(|ie| ie.key() == row.ie_type);
    let null = match kept.next() {
        _ if scope.count(row.ie_type) > 0 || !on_each(row, context) => return,
        Some(ie) if <S::P as Checking>::is_null(ie) => true,
        Some(_) => return,
        None => false,
    };
    let finding = match null {
        true => TableFinding::NullLength(row.ie_type),
        false => TableFinding::ConditionalMissing(row.ie_type),
    };
    let here = Path::Row {
        parent: path,
        row: row.name,
        nth: None,
    };
    found.push(<S::P as Checking>::deviation(
        &here, table, finding, context,
    ));
}

/// A row that is there, on the interface, where the condition that its table writes beside
/// it has it absent: an IE of a null length too.
pub fn condition_absent<S: Scope>(
    scope: &S,
    at: usize,
    path: &Path<'_>,
    context: <S::P as Checking>::Context,
    found: &mut Vec<<S::P as Checking>::Deviation>,
) where
    S::P: Checking,
{
    let table = S::table();
    let row = &table.rows[at];
    let there =
        scope.count(row.ie_type) > 0 || scope.raw().iter().any(|ie| ie.key() == row.ie_type);
    if !there || row.on(<S::P as Checking>::interfaces(context)).is_none() {
        return;
    }
    let here = Path::Row {
        parent: path,
        row: row.name,
        nth: None,
    };
    let finding = TableFinding::Excluded(row.ie_type);
    found.push(<S::P as Checking>::deviation(
        &here, table, finding, context,
    ));
}
