// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! How CASA's stored formats of information are read and written: the
//! rules [`EntryInformation`](super::EntryInformation) and
//! [`AssessmentInformation`](super::AssessmentInformation) share, each
//! reporting them through its own error type.

use logbook_shared_kernel::AuthoritySpecificInformation;
use serde_json::{Map, Value};

/// Why stored information cannot be read, before each format names it in
/// its own error.
#[derive(Debug)]
pub(crate) enum Unreadable {
    NotAnObject,
    UnsupportedVersion,
    UnknownKey(String),
    InvalidAnswer(&'static str),
}

/// The answers of information in a format of this `version` and these
/// `keys`. Stored information must carry the version; information about to
/// be saved (`stored` false) may lack it, but not carry another. A key that
/// is not the format's, beside the kernel's reserved
/// [`AuthoritySpecificInformation::VERSION_KEY`], is refused.
pub(crate) fn answers<'a>(
    information: &'a AuthoritySpecificInformation,
    version: u32,
    keys: &[&str],
    stored: bool,
) -> Result<&'a Map<String, Value>, Unreadable> {
    let answers = information
        .value()
        .as_object()
        .ok_or(Unreadable::NotAnObject)?;
    let versioned = stored || answers.contains_key(AuthoritySpecificInformation::VERSION_KEY);
    if versioned && information.version() != Some(version) {
        return Err(Unreadable::UnsupportedVersion);
    }
    if let Some(key) = answers.keys().find(|key| {
        key.as_str() != AuthoritySpecificInformation::VERSION_KEY && !keys.contains(&key.as_str())
    }) {
        return Err(Unreadable::UnknownKey(key.clone()));
    }
    Ok(answers)
}

/// The answer under `key`, read by `answer`, which gives `None` for a value
/// that is not an answer of the key's kind. An absent key and a `null` are
/// both no answer, as the kernel's questions count them
/// ([`Questions::required_unanswered_for`](logbook_shared_kernel::Questions::required_unanswered_for)).
pub(crate) fn answer<T>(
    answers: &Map<String, Value>,
    key: &'static str,
    answer: impl Fn(&Value) -> Option<T>,
) -> Result<Option<T>, Unreadable> {
    match answers.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => answer(value)
            .map(Some)
            .ok_or(Unreadable::InvalidAnswer(key)),
    }
}

/// The answers given, each under its key, as information without a
/// version: an answer not given has no key.
#[cfg(test)]
pub(crate) fn information<'a>(
    answers: impl IntoIterator<Item = (&'a str, Option<Value>)>,
) -> AuthoritySpecificInformation {
    let answers: Map<String, Value> = answers
        .into_iter()
        .filter_map(|(key, value)| Some((key.to_owned(), value?)))
        .collect();
    AuthoritySpecificInformation::new(Value::Object(answers))
}
