# au-casa — Working Instructions

CASA (Australia) national aviation regulatory package. Sibling to
[`icao-shared-kernel`](https://github.com/Open-Aviation-Solutions/icao-shared-kernel)
(the ICAO-universal domain), on which it builds. Grew out of the earlier
Python kernel's tasks `0011` (which trimmed CASA-specific fields out of the
kernel `Aircraft`) and `0012` (which specified this package before it
existed), which are not published; this repo is where the CASA-specific
design now lives and evolves.

## Purpose and scope

This package resolves **national** (CASA Part 61) facts that the kernel
deliberately does not carry, because they aren't ICAO-universal (the
kernel's `INSTRUCTIONS.md` keeps a State's own rules out of it). Each module
answers one such question:

- `aircraft` (implemented, task `0001`): CASA category and class (regs
  61.015 and 61.020), from the description of an aircraft's type (the table
  of types in aircraft-type-descriptions) and the characteristics recorded
  for its airframe (logbook-shared-kernel's `AircraftCharacteristic`), and
  the design features that require an endorsement (reg 61.755), which only
  `Casa::classify` gives (au-casa#80). Gas turbine engine is read from the
  description's engine type; every other design feature is the
  characteristic recorded for it, and nothing is implied from the
  description: a seaplane or amphibian with neither float alighting gear
  nor a floating hull recorded is cautioned, not given one. A multi-engine
  centre-line thrust aeroplane is in the single-engine class (reg
  61.020(2)(a), au-casa#13); reg 61.020(2)(b)'s prescribed aeroplanes are
  not read.
- `fstd` (implemented, task `0002`): recognition of a flight simulation
  training device under reg 61.010, and whether a device is one at all
  (`counts_for_part61`). Builds on the `FlightSimulationTrainingDevice` /
  `FstdSession` aggregates in `icao-shared-kernel`. Reg 61.010 defines two
  terms: a "flight simulation training device" (any of five recognised kinds,
  which is what `fstd` models), and an "approved flight simulation training
  device" for a purpose (one such device that, by the definition's
  paragraphs, may be used for that purpose). The recency rules (reg 61.870,
  for one) use the second, so whether a session
  counts for them is `logbook`'s `approved_fstd`, which the pilot records,
  not `counts_for_part61`.
- `licence` (issue #14): for each of the kernel's Annex 1 titles of licence
  (`icao_shared_kernel::LicenceTitle`), whether CASA issues it and, if so, its
  Part 61 term (the flight crew licence and aircraft category rating), with
  citations. A CASA host names and offers titles through it, not by the
  kernel's ICAO names. The `match` is exhaustive on purpose: a title the
  kernel adds is a compile error here, and a decision. The pairs Part 61
  grants that Annex 1 has no title for (the recreational pilot licence, and
  the gyroplane category rating) are `NationalTitle`s (issue #15), listed by
  an exhaustive `match` the same way, each with its licence's minimum age;
  `IssuedNationalTitle` is one held from a date of first issue. They sit
  beside the kernel's titles: a consumer stores them with the licence, keyed
  by its identity, and the kernel stays the source for what Annex 1 can say.
  `Casa` supplies them through `LicensingAuthority::licence_titles`: Annex 1's
  titles CASA issues, each under its Part 61 name, then the `NationalTitle`s
  with their minimum ages. `minimum_ages` is Annex 1's own table
  (`MinimumAges::default()`), which CASA's regs 61.515(1), 61.580(1),
  61.655(1), 61.700(1) and 61.1540(1) agree with; a test holds each to its
  regulation, so the test fails when au-casa takes up a kernel whose default differs from Part 61 (a consumer resolving a newer kernel patch is not stopped by it).
  `FlightCrewLicence::is_pilot_licence` says which licences are pilot
  licences (reg 61.010), so no caller matches on the licences' names for it.
- `logbook` (au-casa#29): CASA's rules for a pilot's logbook,
  as `Casa`, which implements logbook-shared-kernel's `LicensingAuthority`
  for `AU-CASA`; and CASA's authority-specific information on an entry
  (`EntryInformation`) and on an assessment (`AssessmentInformation`; neither
  is exported, see below: the
  check type of regs 61.745(3) and (3A), and its outcome or, for a flight
  test, which has no outcome of its own, its purposes, each a kind
  (`licence`, `category_rating`, `class_rating`, `operational_rating` or
  `operational_endorsement`: not any endorsement, since reg 61.745(3)(c)
  has a design feature endorsement's flight training completed, not a
  flight test passed) and a value with an outcome of its own, as the
  kernel's `Answer::Choices` stores them, in the order of `CheckPurpose::ALL`, which is part of the
  format; the checks read an assessment without the answers to questions
  that do not apply to it, as an entry; au-casa#60), two
  stored formats whose every key and spelling is mapped by hand, with no
  derived serde, each under a version of its own (the kernel's reserved key
  `_version`). A stored value is an identifier (`flight_test`,
  `instrument_rating`), and its wording is its choice's label, so a value
  does not change when its wording does. `Casa` says which entries count (`counts`: a session
  only in an approved flight simulation training device, reg 61.010), what
  an aircraft counts as and requires (`classify`: its class's aircraft
  class rating and its design feature endorsements, each term with a
  stable id; see "Term ids" below), the ratings and endorsements it issues
  (`ratings_and_endorsements`, in `logbook::ratings_and_endorsements`: the
  class ratings, the design feature endorsements, the instrument rating and
  its instrument endorsements (table 61.890), and the private instrument and
  night VFR ratings; every term `classify` requires
  is one of them, under the same id. The list is not yet every rating and
  endorsement CASA issues, as the kernel's contract asks: the other
  operational ratings and endorsements, and the type ratings (#101), are
  not listed yet), the problems with what is held under a licence
  (`licence_problems`: a refusal at the instrument rating for each part of
  table 61.890 none of whose instrument endorsements is held with it, reg
  61.885(1)(b)(i) and (ii), reported at the rating rather than at the
  licence; the private instrument rating's endorsements, table 61.935,
  are not listed, so it is not checked, #113), which requirements it checks, each with a
  stable id, a name in its regulation's words, a description of what
  meets it, the noun its events are counted in, the set its
  regulation sets it in (a key, a name and a description, equal on every
  member, which apply together) and its kind (`requirements`; see "Currency
  checks"), which of them apply to what a pilot holds (`requirements_for`), the
  pilot's currency against them (`currency`), and CASA's own totals
  (`totals`, over the entries that count, each with a stable id and the
  total it is part of, the classes of aeroplane under aeroplane time; solo
  flight time is the flights whose `sole_occupant` is true, and no rule
  makes solo count as pilot in command; cross-country flight time is the
  flights CASA's information answers were cross-country flights, whatever
  the capacity, as solo flight time is); the questions it asks for its information on an entry and on
  an assessment (`questions_for`, in `logbook::questions`: one for each key
  of the format, every one in the list whatever the record, and one that
  applies only to some records has a kernel `Condition`, never left out,
  so that the host's `retain_applicable_answers_for` removes its
  answer when it saves a record it no longer applies to; the checks read an
  entry's information through `information`, which removes those answers
  too, so an answer already stored to a question that does not apply counts
  for nothing; whether a flight was a cross-country flight
  (`cross_country_flight`, reg 61.010) is asked of a flight
  (`Condition::Flight`, since a session has simulated flight time, not
  flight time), for the whole flight, first of the questions that apply to
  every flight, and is not required: an unanswered flight is not one, and
  the pilot answers it for a flight stored before it was asked; the DME or
  GNSS arrivals, which exclude operations also recorded in the entry's
  approaches (both count towards reg 61.870(2), and a count cannot be
  de-duplicated against them, so the wording must keep excluding them), are
  asked whatever the flight rules, and before the azimuth guidance and course deviation indicator
  operations, which apply to an entry that records an approach or a DME or
  GNSS arrival (`Condition::Any` of `AnyApproach` and a `CountAtLeast` of
  one on the arrivals), as an arrival is an instrument approach operation
  too; the exclusion is no format change: the count was always meant as reg 61.870(2)'s operations, narrowing it only rules out counting one operation twice, and no stored answer existed when 0.11.0 released it); and the problems with the
  answers (`problems_for`): information it cannot read, so that the host
  does not stamp and save it, reg 61.095(3)(b)'s "the person is not
  receiving flight training", where that question applies, and an
  assessment whose entry's aircraft cannot be placed in a category, with
  what is missing (its type's description, or a session's type) and where
  to record it, worded to read whether the assessment or its entry is being
  saved: every check CASA records is evidence about an aircraft
  category or class, whatever its kind (au-casa#99, pilot-logbook#249).
  That one's information reads, but it is still refused at the information
  as a whole, which is not only for information that cannot be read; and
  the entry given with an assessment must be the one it names, as a
  stand-in (a session with no type, say) is refused. Stored information must carry its version
  (`read`); only information about to be saved may lack it
  (`read_to_save`), which is how `problems_for` reads what it is given. A `null` answer is no answer, as an
  absent key is. The currency checks are in `logbook::currency`, which
  also lists the requirements and runs them; see "Currency checks" below.

**The information's readers are crate-private (au-casa#68).** `logbook`
exports only `Casa`; the migration of stored information is
`LicensingAuthority::migrate_information` on it (the function it calls for an
entry's version 1, and its error `EntryMigrationError`, are crate-private from
0.12.0). The readers `EntryInformation` and
`AssessmentInformation`, their read errors (`InformationError` and
`AssessmentInformationError`), `CheckType`, `CheckOutcome`, `CheckPurpose`, `AssessedPurpose`,
`PurposeKind` and `SupervisionRoute` are `pub(crate)`, with no writer outside
the tests. A host reads CASA's information on a record only through
`LicensingAuthority` (`counts`, `questions_for`, `problems_for`, `currency` and
`totals`) and the kernel's `Questions`, as au-casa's own checks do, so that the
compiler, and not only this file, keeps it to the trait: a host that read the
typed information would depend on CASA's stored keys and spellings, which are
the authority's to change, and a second authority could not be added without
it. Before, a host could read and write the format directly; do not re-export
a typed reader to give it that again (`compile_fail` doctests in `logbook`
keep it). A migration is not a reader: it is offered on the kernel's types
(see "Releasing"). Where a host needs something else the trait lacks, add it
to the trait.

## Currency checks

- **One module for each regulation**, an `EntryAccumulator` and
  `CurrencyCheck` (or the assessment traits) over the kernel's
  `EventWindow`, in days or calendar months as its regulation counts them.
  Its doc comment quotes the regulation it implements. A validity that
  runs to "the end of the Nth month after the month in which" (the flight
  review, reg 61.745(1A)(a); the instrument proficiency check, reg
  61.880(3)) is not a look-back the kernel's `EventWindow` has, so those
  checks keep their own passes and build their result with the kernel's
  `end_of_nth_month_after` (decided on au-casa#53: the rule stays here
  until another authority needs it). Both also renew early, from the expiry
  of the validity they renew (regs 61.745(1A)(b), 61.880(3)(f): a pass in the 3 months before it, the
  window opening on the 1st of the month; for the flight review every check
  of reg 61.745(3) renews, for the instrument proficiency check only an
  instrument proficiency check does), and
  share `currency/renewal.rs` for that chaining (au-casa#54).
  Regulations whose checks differ only in their figures, in whether
  they count by night only, and in whether a simulator session counts,
  share one, with a constructor for each that quotes its regulation
  (`take_offs_and_landings`, for regs 61.395(1) and (2) and 61.965(a)).
- **Each sees only the entries `Casa::counts` lets through**: the kernel's
  `run` applies it once, so a check does not repeat it. A check over
  assessments that reads their aircraft, which `run` does not reach, is
  given those entries by its caller (`Casa::currency`) when it is built.
- **An assessment's aircraft is its entry's.** A check over assessments
  that reads it (the flight review) is given the entries, and an assessment
  is in the aircraft of the entry it names (`Assessment::entry`, by its
  `EntryId`); one whose entry is not among them, or is a session's, is
  unclassified and does not count. An assessment's information is read
  with its entry (the kernel's `Record::Assessment`), so the instrument
  proficiency checks, which read a failure's information whether or not
  its entry counts, are given every entry too; one naming no entry of the
  logbook, which a host does not let happen, they read as stored, so that
  its failure still ends them all. The instrument
  proficiency checks read it too (au-casa#58): a pass counts for a check
  when its aircraft is a "relevant aircraft" for it (reg 61.880(9)), so one
  in a multi-engine aeroplane counts for the aeroplane category as well as
  for multi-engine aeroplanes; a failure ends the checks of its aircraft's
  category, the multi-engine one included (reg 61.880(4), read as ending
  both until au-casa#94 settles whether it ends only the kind of check
  failed), and one whose aircraft is not known ends them all. That is the
  one exception to an unclassified assessment not counting: a pilot shown
  current after a failure that could not be placed is the worse error
  (decided on au-casa#93). `problems_for` refuses such a check
  (au-casa#99), so the fallback is for records saved before it did; keep
  it.
- **An unknown type does not count where a category is required.** A check
  that requires a category (`guards::is_category`) needs positive evidence;
  regs 61.870(2) and (4) to (7) require none ("in an aircraft"), so any
  type counts for them. The type is classified
  from the table's description of its designator or, for a designator the
  table does not catalogue, from the description the entry carries, as
  pilot-logbook did; an entry with neither is not of any category.
- **Each requirement has a stable id**, from `logbook::currency`'s
  `CasaRequirement`: the regulation (bare, `61.870(2)`), and for a flight
  review `|` and its class rating's stored name, as pilot-logbook keyed
  them. Reg 61.870(3), which is per category, has one for each of reg
  61.015's five, the regulation, `|` and the category's stored name
  (`61.870(3)|aeroplane`); pilot-logbook had no key for it, so these ids
  originate here. Regs 61.395(1) and (2) and 61.965(a) require take-offs
  and landings, counted separately, so each is two requirements, one for
  its take-offs and one for its landings, with the regulation as its
  `regulation` and the paragraph that requires them as its id
  (`61.395(1)(a)` for the take-offs, `61.395(1)(b)` for the landings,
  `61.965(a)(i)` and `61.965(a)(ii)`); these originate here too
  (au-casa#67). Reg 61.880 has one for each of reg 61.015's categories
  (`61.880|aeroplane`) and one each for its subregulation (2)'s
  multi-engine aeroplanes and helicopters (`61.880|multi-engine-aeroplane`,
  `61.880|multi-engine-helicopter`), with the regulation as its
  `regulation`; it replaced the bare `61.880` in 0.11.0 (au-casa#58). A
  multi-engine helicopter, which Part 61 does not define, is a helicopter
  with 2 or more engines. A host may keep an id, so changing one is a breaking
  change. A new check is a `CasaRequirement` variant (a `match` on it will
  not compile without its id, name, description, events noun, set, kind
  and regulation), a row of
  `REQUIREMENTS`, and its result paired with that variant in `currency`; a
  test fails until the last two agree.
- **Which requirements apply is CASA's rule** (`requirements_for`,
  au-casa#106, pilot-logbook#240): each `CasaRequirement` applies to the
  holder its regulation names (`applies_to`, an exhaustive `match`, so a new
  requirement does not compile until it says whom it applies to), from the
  ids of the titles, ratings and endorsements held. A requirement in a
  category, which regs 61.395 and 61.965 are here in an aeroplane, applies
  for a title with that category rating, the title being the licence and its
  category rating together (the night VFR rating as well, for reg 61.965);
  the glider pilot licence is not a pilot licence. Reg 61.395(2) (a
  passenger at night) is listed only for a pilot who may fly an aeroplane at
  night too (au-casa#110): the night VFR rating or the instrument rating
  with an aeroplane instrument endorsement (table 61.375 item 3, regs
  61.855(b) and 61.865(1)), or the multi-crew or air transport
  pilot licence with the aeroplane category rating (reg 61.375(8)(a)(ii),
  (b)(ii)); without one it is left out, as reg 61.965 is without the night
  VFR rating, and reg 61.395(1) is unchanged. The instrument rating
  gives a privilege in a category or an approach only with its instrument
  endorsement (reg 61.865), so reg 61.870(3) and reg 61.880 apply for the
  aircraft category and class endorsements held (Part 1 of table 61.890), the
  multi-engine checks for the multi-engine ones only, and reg 61.870(4) and
  (5) for the IAP 2D and IAP 3D instrument endorsements; regs 61.870(2), (6)
  and (7) and 61.875 apply for the rating. An instrument endorsement held
  implies the instrument rating (its applicant must hold one, reg
  61.905(1)(a)), and the IAP 3D instrument endorsement the IAP 2D one (item
  9 of table 61.890), so an endorsement recorded alone does not give only
  some of its rows. A flight review applies for its
  class rating. A test holds each to what it applies to, and every
  requirement to some holder.
- **A requirement is shown in CASA's words** (au-casa#96), so that a host
  holds no wording or grouping keyed by CASA's ids. Its `description` says
  what meets it; the texts were pilot-logbook's `REQUIREMENT_WORDING`,
  moved here as the owner reviewed them, and reg 61.870(3)'s the owner's
  on au-casa#96. Its `events` noun, both forms, is given for the take-offs,
  landings and operations regs 61.395, 61.965 and 61.870 count. Its `set`
  is for reg 61.395(1)'s and (2)'s paragraphs and reg 61.965(a)'s: a key (the
  provision that sets them together, `61.395(1)`, `61.965(a)`; the kernel
  calls it opaque, so a host groups by it and does not read it), a name and
  a description in CASA's words (au-casa#109), equal on every member, which
  nothing in the kernel checks and au-casa's test does. A set's members
  apply together, all or none (`applies_to` chooses by kind alone). Its
  kind is recent experience (regs 61.395, 61.870, 61.875 and 61.965) or a
  periodic assessment (regs 61.745 and 61.880).
- **Testing:** each check's unit tests assert its boundaries (the window's
  edge and the expiry, the category guard where it has one, an entry dated after the
  reference date), with entries built by `logbook::test_support` from the
  real table of types. A check over `EventWindow` does not depend on the
  order entries arrive in, by construction (it sorts and filters the whole
  set). A check that decides over the whole set itself, such as one that
  groups a flight's entries, asserts the same result for both orders.

`icao-shared-kernel`, logbook-shared-kernel and aircraft-type-descriptions
are taken by a caret range (`^0.7`, `^0.13.0` and `^0.1.0`), not pinned exactly
(au-casa#24): sibling country crates (nz-caa and the like) depend on them
too, and a consumer such as a logbook imports several at once. Exact pins on
different patches cannot be resolved together; with caret ranges Cargo builds
one copy of each, so the kernel's `LicenceTitle` and the table's `AirframeKind` are the same types in
every crate. A caret range takes up patches but not a minor version, and each
crate releases a change that could alter a classification as a minor version:
the kernel by [its rule](https://github.com/Open-Aviation-Solutions/icao-shared-kernel/blob/main/INSTRUCTIONS.md#consumers), the table by [its rule](https://github.com/Open-Aviation-Solutions/aircraft-type-descriptions/blob/main/INSTRUCTIONS.md#releasing). A patch of the
table can still change an answer (a new row, a confidence, a citation). A
minor release is taken by the country crates together. A consumer takes them
from the same source, by a caret range compatible with au-casa's (the same
`0.x` minor), since Cargo builds one copy only within one such range.

Before starting work, check whether any of the three has a newer release
(`cargo search icao-shared-kernel --limit 1`, and the same for
logbook-shared-kernel and aircraft-type-descriptions). A newer patch is taken
up with `cargo update -p <crate>` in the same pull request. A newer minor
version is breaking, and is moved to first, in a pull request of its own,
before the work; it then needs an au-casa minor release of its own, as above.
An issue can settle otherwise: au-casa#29 takes icao-shared-kernel 0.5 and
logbook-shared-kernel 0.2 in the first pull request of its stack, which
releases once, at its end.

## Term ids

`Casa::classify`, `Casa::ratings_and_endorsements` and `Casa::totals` give
each term a logbook-shared-kernel `TermId`, which a host may keep (pilot-logbook stores them), so once
released an id does not change; a name may. Each is mapped by hand
(`category_id` and `class_id` in `logbook`, `DesignFeature::id`; a test
holds the first two to the stored names):

- A category's is its stored name, as a requirement of reg 61.870(3) has it
  (`aeroplane`, `powered-lift`, `registered-sailplane`).
- A class's is its stored name, as a flight review's requirement has it
  (`single-engine-aeroplane`), but for the airship class, `airship-class`,
  since `airship` is the category's.
- A design feature's is pilot-logbook's stored spelling of it
  (`tailwheel-undercarriage`, `floatplane` for an aeroplane's float
  alighting gear, `float-alighting-gear` for a helicopter's,
  `gas-turbine-engine`).
- A rating or endorsement that `classify` requires has the same term in
  `ratings_and_endorsements`, so a host matches a held one by equal ids. An
  aircraft class rating has an id of its own, so that no id names two
  things (the owner, on au-casa#100): its class's id followed by
  `-class-rating` (`single-engine-aeroplane-class-rating`), but for the
  airship class rating, `airship-class-rating`, as the airship class's id
  already ends in `-class`. Its name is Part 61's for the rating
  ("single-engine aeroplane class rating"); the class's own term in
  `counts_as` and the totals keeps the class's id and name. A design
  feature endorsement has its design feature's id. A test holds that no
  listed id is a class's or a category's. An operational rating's id, and an
  instrument endorsement's (table 61.890, au-casa#106), is its Part 61 name
  in lower case joined by `-` (`instrument-rating`, `private-instrument-rating`,
  `night-vfr-rating`, `single-engine-aeroplane-instrument-endorsement`,
  `iap-2d-instrument-endorsement`). A test lists them all.
- A title of licence's (`licence_titles`, in `logbook::licence_titles`) is
  its licence's name and its category rating's, joined by `-`, which is how
  pilot-logbook spells a stored title's pair, so its own titles read without
  a migration (`recreational-aeroplane`, `private-gyroplane`;
  `air-transport-aeroplane`, `multi-crew-aeroplane`; `glider` alone, as it has
  no category rating). The category's is `category_id`'s. A host stores the
  ids of the national titles, so they do not change; a test lists them all.
- A total that is a category's or a class's time has that term's id; the
  others are `simulated-flight-time`, `solo-flight-time` and
  `cross-country-flight-time`.

## Reading Part 61 for licences and category ratings

Division 61.L.2 does not list which aircraft category ratings each licence can
carry. Reg 61.730 grants a category rating on meeting the requirements for
"the pilot licence with the aircraft category rating", so the pairs are those
each licence's Subpart (61.H to 61.K) gives aeronautical experience
requirements for. A pair cites those requirements and, for the multi-crew
and air transport pilot licences, also the privileges regulation that names
the category (regs 61.635 and 61.665). Some pairs appear only in the
requirements for applicants who have not completed an integrated training
course (Divisions 61.H.3 and 61.I.3); CASA still issues them. The glider pilot licence is a flight crew licence but not a pilot
licence (reg 61.010), and a glider is not a category of aircraft (reg 61.015),
so it carries no category rating (Subpart 61.Z).

The recreational pilot licence is the exception to reading the pairs from
the regulations: reg 61.475 grants it with "the associated aircraft category
rating", naming none. Its pairs are those the Part 61 Manual of Standards has
standards for (Schedule 1, Section G, Appendices G.1 to G.3: aeroplane,
helicopter and gyroplane), each cited with reg 61.475. Appendix G.4, the
airship, is reserved, and there is no appendix for a powered-lift aircraft
(decided on issue #15).

**One part of `aircraft` is not national, and is not here.** `AircraftDescription`
and the `designator -> description` table (Layer A) are the same for any
country, so they live in
[aircraft-type-descriptions](https://github.com/Open-Aviation-Solutions/aircraft-type-descriptions)
(au-casa#7), which this crate depends on by a compatible range (see above). It
is the vocabulary for aircraft facts shared by the country crates, as the
kernel is for `AircraftType` and `LicenceTitle`, so au-casa does not wrap,
redefine or re-export its types (`AircraftDescription`, `AirframeKind`,
`EngineType` and `Confidence`): a consumer names them from that crate. Only Layer B, the derivation to CASA category/class/design
features, is regulatory interpretation, and it is what stays here. A type is
added to the table there, not here; a type the table lacks is
classified from the description the host gives for it until it is.

## Conventions

- **Stateless capabilities only.** This package provides pure derivation
  functions and value objects — no repository protocols, no persistence, no
  identity-keyed storage. Per the Dependency Inversion Principle, storage of
  any consumer-specific data (e.g. the characteristics recorded for an
  aircraft) belongs
  to the consuming application that owns that data's lifecycle, not here —
  the same reasoning that kept repository protocols out of
  `icao-shared-kernel`.
- **No vendored ICAO-copyrighted data.** The table of types, and how it is
  sourced, is aircraft-type-descriptions' to say (and it sources US public
  domain data).
- **The MPL notice heads each file** in `src/`, `tests/`, the `Makefile`,
  `Cargo.toml` and `.github/workflows/` (the licence's Exhibit A, in that
  file's comment syntax); `make license-add` adds it to each tracked file
  that has none (so `git add` a new file first). `make license-check`, part
  of `make check`, runs `addlicense -check` over the tracked files, which
  fails on a Rust, TOML or YAML file with no licence header, though it does
  not check that the header is the MPL's wording. Neither covers the
  `Makefile`, which addlicense does not recognise.
- **Regulatory terms only**: names and terminology must be verifiable against
  CASA Part 61 (or the specific regulation cited).
- **Discuss before implementing**: a design decision is discussed in an issue
  first, and the issue records what was decided. `tasks/archived/` holds the
  earlier design notes (`NNNN-*.md`), kept as history; tasks `0001` and `0002`
  cite tasks of the earlier Python kernel that are not published.

## Releasing

Released to crates.io as the kernel is, from `main` only. Bump `version` in
`Cargo.toml` in a pull request, and merge it: that is the release. On each
push to `main`, the Release workflow (`.github/workflows/release.yml`) asks
crates.io whether `Cargo.toml`'s version is there yet. When it is not, it runs
`make check`, publishes it through trusted publishing (no crates.io token is
kept in the repository), and tags the commit `v<version>`. Don't push version
tags by hand: nothing runs on them.

Only `main` can release. The publishing job runs in the `release` environment,
which GitHub allows only on `main`, and crates.io's trusted publisher accepts
only that environment; `main` changes only through a pull request with CI
passing.

CASA's information on an entry and on an assessment (`EntryInformation`,
`AssessmentInformation`) are stored formats: a host stores them in its
records, though it cannot read them (see above). A change to either, to a
key, a spelling, or what an answer means, is breaking: it bumps that format's
`VERSION`, is released as a breaking release, and ships with a migration of
the data repository's stored information.

The migration is in two parts (decided on au-casa#60, and kept on #68). au-casa
ships a public, tested function that rewrites one record's stored
information from the previous version to the new one, taking and returning
the kernel's `AuthoritySpecificInformation`, so that it needs no reader to be
public. It refuses, with the reason, what it cannot rewrite without
guessing, so that the pilot fills it in by hand. Information of the new
version already is given back as it is, once it reads, so the migration can
be run again. It knows the format, and nothing of where a host keeps its
records. The host's take-up of the release runs it over the data repository,
on a data repository branch merged with the host's take-up, not with
au-casa's pull request, since a deployed page refuses records of a version it
does not read until it takes up the release.

`Casa` offers the migration through the kernel's
`LicensingAuthority::migrate_information` (au-casa#88, in 0.11.1), for an
entry and for an assessment, and refuses by the kernel's kinds: newer than the
current version, older than every migration kept (version 0 of an entry; any
version before 2 of an assessment, as none is kept) and not readable
(information with no version, that is not an object, or of version 1 of an
entry that does not read). Version 2 of an entry or of an assessment gives
`Ok(None)` on its version alone, without its answers read (`Ok(None)` is not a
check of them, which is `problems_for`'s: the owner's decision on
logbook-shared-kernel#45).

The migration so far is `entry_information::migrate_from_version_1`
(au-casa#86, public in 0.10.0 to 0.11.1 as
`logbook::migrate_entry_information_from_version_1`, crate-private from
0.12.0, as `migrate_information` supersedes it), from version 1 of the information on an entry to
version 2, which added `cross_country_flight`: it restamps the version and
keeps every answer, so it refuses only information that reads as neither
version. An earlier one, `AssessmentInformation::migrate_from_version_1`
(run by pilot-logbook#206's `make migrate-assessment-purposes`), was removed
in 0.7.0: it was written over the typed reader, which is no longer public, and
the data repository was past it. It is in the history before 0.7.0.

While on `0.x`, a breaking release bumps the minor version (`0.2.0`) and a
compatible one the patch, as the kernel's do (decided on au-casa#24).
Breaking includes a change to the API; a change to the Part 61 reading
that changes an existing designator's category, class or design features, or
a title of licence's Part 61 term; and a requirement added or removed, or a
change to what makes an existing one current. Consumers take au-casa by a
caret range on the current minor (such as `^0.14.0`), so a `cargo update` takes up patches but not a breaking release,
which they move to on purpose.

## Commands

```sh
make test           # cargo test
make lint           # cargo clippy -- -D warnings
make fmt            # cargo fmt
make license-add    # add the MPL notice where a tracked file has none
make license-check  # addlicense -check over the tracked files
make check          # lint + fmt check + licence headers + test
```
