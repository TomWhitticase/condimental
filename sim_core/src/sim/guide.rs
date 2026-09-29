//! `data/guide.ron` — the prose half of THE COOKBOOK, and the closed
//! vocabulary of facts its sentences are allowed to ask for.
//!
//! **WHY A REGISTRY AND NOT A STRING TABLE IN GDSCRIPT.** The rest of the
//! book is rendered from the content files, and `cookbook_panel.gd` names no
//! sauce, machine, price or contract — a test fails if a literal in it ever
//! matches one. The "how do I play this" pages had to hold to the same rule
//! or they would be the one part of the book that goes stale silently, so the
//! paragraphs are content in `data/guide.ron` and this module is their
//! loader.
//!
//! **THE PLACEHOLDER RULE, WHICH IS THE WHOLE POINT.** Prose in that file
//! never states a number: it writes `{starting_money}`, and the number is
//! filled in from the registries when the book is opened. A balance pass can
//! therefore not make the guide lie, which is exactly what a hand-written
//! "you start with 3000" would do the day someone changed
//! [`super::economy::STARTING_MONEY`].
//!
//! The vocabulary is [`GuideFact`], and it is CLOSED: a placeholder this
//! module does not know is a load error naming every one it does know.
//! Leaving it to the presentation layer would ship a typo as the literal
//! text `{starting_mony}` in the middle of a paragraph, which is a bug only
//! a player ever sees. The loader is the last place that can catch it.
//!
//! **THE SIM COMPUTES A FACT, GODOT FORMATS IT.** Each fact carries a
//! [`GuideFactKind`] — money, a count, a span of ticks, or a name — and the
//! value goes over the wire as a number or a string with the kind beside it.
//! Godot turns ticks into seconds and money into the game's money string,
//! because "seconds, never ticks" is a presentation rule and the tick rate
//! belongs to GDScript (`godot_project/CLAUDE.md`). Nothing in `sim_core`
//! knows how long a second is.
//!
//! The facts themselves are computed in [`super::cookbook`], which already
//! holds every registry the numbers come from; this module only says which
//! facts exist, what they are called in prose and what type each one is.

use std::collections::BTreeSet;

use serde::Deserialize;

/// Bumped when the shape of `data/guide.ron` changes, per ARCHITECTURE §4:
/// the loader refuses a version it was not written for rather than
/// misreading a changed shape.
const CURRENT_SCHEMA_VERSION: u32 = 1;

const GUIDE_RON: &str = include_str!("../../../data/guide.ron");

/// What kind of thing a fact's value is, which is what tells the
/// presentation layer how to print it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuideFactKind {
    /// A money amount, printed in the game's money style.
    Money,
    /// A plain count of things.
    Count,
    /// A span of simulation ticks, printed in SECONDS by the presentation
    /// layer — never as a tick count.
    Ticks,
    /// Player-facing prose that came out of a content file already written
    /// for a human — an unlock's `name`, say.
    Text,
    /// A content **id**, which the presentation layer titles the way it
    /// titles every other id on the wire (`_kind_display_name`). This module
    /// does not title anything: display names reach Godot through
    /// `display_names_json`, and a second title-caser in Rust would be a
    /// second thing to keep in step (`super::cookbook`'s module doc).
    KindId,
}

impl GuideFactKind {
    /// The string the wire carries, and the only one
    /// `cookbook_panel.gd` branches on.
    pub fn wire(self) -> &'static str {
        match self {
            GuideFactKind::Money => "money",
            GuideFactKind::Count => "count",
            GuideFactKind::Ticks => "ticks",
            GuideFactKind::Text => "text",
            GuideFactKind::KindId => "kind_id",
        }
    }
}

/// Every quantity `data/guide.ron`'s prose may ask for, by the name it is
/// written as between braces.
///
/// Adding one is a three-line change here plus its computation in
/// [`super::cookbook`] — and that is deliberately the only way to add one,
/// because a fact nobody computes would render as an empty gap in a
/// paragraph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuideFact {
    /// `sim::STARTING_MONEY`.
    StartingMoney,
    /// The id of the cheapest thing in `data/shop.ron`, titled by Godot.
    CheapestBuyable,
    /// What that cheapest thing costs.
    CheapestBuyablePrice,
    /// How many kinds `data/shop.ron` sells. Not "machines": the shelf holds
    /// belts, pipes and the dock too, so prose that said machine would be
    /// wrong by four.
    BuyableCount,
    /// How many priced raws `data/ingredients.ron` holds, gated ones
    /// included — the size of the file, not of the market.
    RawCount,
    /// How many of those raws the market will actually sell at the current
    /// boxes sold. Three of the six are behind the BBQ gate today, so
    /// prose that called `raw_count` "on the market" was wrong by three.
    OpenRawCount,
    /// How many sauces the book has pages for.
    SauceCount,
    /// How many orders `data/contracts.ron` holds in all, gated ones
    /// included.
    ContractCount,
    /// How many of those are open to sign at the current boxes sold — the
    /// ones with no gate, or whose gate has opened. The board lists the rest
    /// too, padlocked.
    OpenContractCount,
    /// How many of those are `starter: true`: active in a new game without
    /// being signed. Not "the ones a new player may sign", which is
    /// [`GuideFact::OpenContractCount`] — the shipped board has no starter,
    /// while most of its orders are ungated.
    StarterContracts,
    /// `dock::DOCK_CAPACITY`: the places on one dock.
    DockCapacity,
    /// Total starting headcount across `data/staff.ron`'s categories.
    StartingCrew,
    /// What that starting crew costs in one pay period.
    StartingWageBill,
    /// `data/staff.ron`'s `pay_period_ticks`.
    PaydayTicks,
    /// The name of the earliest gate in `data/unlocks.ron`.
    FirstGateName,
    /// How many boxes that gate wants.
    FirstGateBoxes,
    /// The product whose boxes it counts, as an item id. A
    /// [`GuideFactKind::KindId`] rather than a new `ItemId` kind, because
    /// Godot's `_item_display_name` is `_kind_display_name` — one map titles
    /// both, so one kind describes both.
    FirstGateProduct,
}

impl GuideFact {
    /// Every fact, in a stable order — the order the wire lists them in, so
    /// a diff of two snapshots reads consistently.
    pub const ALL: [GuideFact; 17] = [
        GuideFact::StartingMoney,
        GuideFact::CheapestBuyable,
        GuideFact::CheapestBuyablePrice,
        GuideFact::BuyableCount,
        GuideFact::RawCount,
        GuideFact::OpenRawCount,
        GuideFact::SauceCount,
        GuideFact::ContractCount,
        GuideFact::OpenContractCount,
        GuideFact::StarterContracts,
        GuideFact::DockCapacity,
        GuideFact::StartingCrew,
        GuideFact::StartingWageBill,
        GuideFact::PaydayTicks,
        GuideFact::FirstGateName,
        GuideFact::FirstGateBoxes,
        GuideFact::FirstGateProduct,
    ];

    /// What this fact is written as in `data/guide.ron`, without the braces.
    pub fn placeholder(self) -> &'static str {
        match self {
            GuideFact::StartingMoney => "starting_money",
            GuideFact::CheapestBuyable => "cheapest_buyable",
            GuideFact::CheapestBuyablePrice => "cheapest_buyable_price",
            GuideFact::BuyableCount => "buyable_count",
            GuideFact::RawCount => "raw_count",
            GuideFact::OpenRawCount => "open_raw_count",
            GuideFact::SauceCount => "sauce_count",
            GuideFact::ContractCount => "contract_count",
            GuideFact::OpenContractCount => "open_contract_count",
            GuideFact::StarterContracts => "starter_contracts",
            GuideFact::DockCapacity => "dock_capacity",
            GuideFact::StartingCrew => "starting_crew",
            GuideFact::StartingWageBill => "starting_wage_bill",
            GuideFact::PaydayTicks => "payday_ticks",
            GuideFact::FirstGateName => "first_gate_name",
            GuideFact::FirstGateBoxes => "first_gate_boxes",
            GuideFact::FirstGateProduct => "first_gate_product",
        }
    }

    pub fn kind(self) -> GuideFactKind {
        match self {
            GuideFact::StartingMoney
            | GuideFact::CheapestBuyablePrice
            | GuideFact::StartingWageBill => GuideFactKind::Money,
            GuideFact::BuyableCount
            | GuideFact::RawCount
            | GuideFact::OpenRawCount
            | GuideFact::SauceCount
            | GuideFact::ContractCount
            | GuideFact::OpenContractCount
            | GuideFact::StarterContracts
            | GuideFact::DockCapacity
            | GuideFact::StartingCrew
            | GuideFact::FirstGateBoxes => GuideFactKind::Count,
            GuideFact::PaydayTicks => GuideFactKind::Ticks,
            GuideFact::FirstGateName => GuideFactKind::Text,
            GuideFact::CheapestBuyable | GuideFact::FirstGateProduct => GuideFactKind::KindId,
        }
    }

    /// The reverse of [`GuideFact::placeholder`], which is how the loader
    /// tells a known placeholder from a typo.
    pub fn from_placeholder(name: &str) -> Option<GuideFact> {
        GuideFact::ALL.into_iter().find(|f| f.placeholder() == name)
    }
}

/// One page of the guide, exactly as authored. The placeholders are still in
/// the text: substitution happens at the presentation layer, after the
/// formatting decision the kind implies.
#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct GuideTopic {
    pub id: String,
    pub title: String,
    pub tagline: String,
    pub body: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GuideFile {
    schema_version: u32,
    topics: Vec<GuideTopic>,
}

/// What can be wrong with `data/guide.ron`. Every variant says what to do
/// about it: this file is authored prose, so its errors are read by whoever
/// was writing a sentence a moment ago.
#[derive(Debug)]
pub enum GuideLoadError {
    Parse(ron::error::SpannedError),
    UnsupportedSchemaVersion {
        found: u32,
        supported: u32,
    },
    /// No topics at all. The book would open its BASICS section on nothing.
    NoTopics,
    /// Two topics share an id.
    DuplicateId {
        id: String,
    },
    /// A title or tagline that is empty or only spaces — a page with no
    /// heading, which looks like a rendering bug rather than a content one.
    BlankField {
        id: String,
        field: &'static str,
    },
    /// A topic with no paragraphs, or one whose paragraph is blank.
    EmptyBody {
        id: String,
    },
    BlankParagraph {
        id: String,
        index: usize,
    },
    /// A `{placeholder}` no fact answers — a typo, or a fact somebody meant
    /// to add to [`GuideFact`] and did not.
    UnknownPlaceholder {
        id: String,
        name: String,
        known: Vec<String>,
    },
    /// A `{` with no `}` after it. Left to the presentation layer this would
    /// print as a brace in the middle of a sentence.
    UnclosedPlaceholder {
        id: String,
    },
}

impl std::fmt::Display for GuideLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GuideLoadError::Parse(e) => write!(f, "failed to parse guide.ron: {e}"),
            GuideLoadError::UnsupportedSchemaVersion { found, supported } => write!(
                f,
                "guide.ron schema_version {found} is not supported by this build \
                 (supports {supported}) — migrate the data file or update sim_core"
            ),
            GuideLoadError::NoTopics => write!(
                f,
                "guide.ron defines no topics — THE COOKBOOK's BASICS section would \
                 open on an empty page"
            ),
            GuideLoadError::DuplicateId { id } => write!(
                f,
                "guide.ron defines '{id}' more than once — topics are looked up by id, \
                 so a duplicate silently shadows the other"
            ),
            GuideLoadError::BlankField { id, field } => write!(
                f,
                "guide.ron's '{id}' has a blank {field} — a page with no {field} reads \
                 as a rendering fault rather than as missing content"
            ),
            GuideLoadError::EmptyBody { id } => write!(
                f,
                "guide.ron's '{id}' has no paragraphs — a topic that says nothing is a \
                 page in the book with nothing on it"
            ),
            GuideLoadError::BlankParagraph { id, index } => write!(
                f,
                "guide.ron's '{id}' has a blank paragraph at index {index} — delete it \
                 rather than shipping a gap the panel will space out"
            ),
            GuideLoadError::UnknownPlaceholder { id, name, known } => write!(
                f,
                "guide.ron's '{id}' asks for {{{name}}}, which no fact answers — the \
                 vocabulary is closed and lives in sim/guide.rs (GuideFact); it knows: \
                 {}",
                known.join(", ")
            ),
            GuideLoadError::UnclosedPlaceholder { id } => write!(
                f,
                "guide.ron's '{id}' opens a {{ and never closes it — a placeholder is \
                 {{name}} with no spaces"
            ),
        }
    }
}

impl std::error::Error for GuideLoadError {}

/// The guide's pages, in the order `data/guide.ron` lists them, which is the
/// order the panel shows them in: the file is the running order.
#[derive(Debug, Default)]
pub struct GuideRegistry {
    topics: Vec<GuideTopic>,
}

impl GuideRegistry {
    pub fn from_ron_str(text: &str) -> Result<Self, GuideLoadError> {
        let file: GuideFile = ron::from_str(text).map_err(GuideLoadError::Parse)?;
        if file.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(GuideLoadError::UnsupportedSchemaVersion {
                found: file.schema_version,
                supported: CURRENT_SCHEMA_VERSION,
            });
        }
        if file.topics.is_empty() {
            return Err(GuideLoadError::NoTopics);
        }

        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for topic in &file.topics {
            if !seen.insert(topic.id.as_str()) {
                return Err(GuideLoadError::DuplicateId {
                    id: topic.id.clone(),
                });
            }
            if topic.title.trim().is_empty() {
                return Err(GuideLoadError::BlankField {
                    id: topic.id.clone(),
                    field: "title",
                });
            }
            if topic.tagline.trim().is_empty() {
                return Err(GuideLoadError::BlankField {
                    id: topic.id.clone(),
                    field: "tagline",
                });
            }
            if topic.body.is_empty() {
                return Err(GuideLoadError::EmptyBody {
                    id: topic.id.clone(),
                });
            }
            for (index, paragraph) in topic.body.iter().enumerate() {
                if paragraph.trim().is_empty() {
                    return Err(GuideLoadError::BlankParagraph {
                        id: topic.id.clone(),
                        index,
                    });
                }
            }
            // Every piece of prose on the page is checked, not just the
            // body: the tagline carries placeholders (`money`'s does), and
            // the title is prose the panel renders through the same
            // substitution, so a typo there would put braces on screen in
            // the largest font the book owns.
            for text in std::iter::once(&topic.title)
                .chain(std::iter::once(&topic.tagline))
                .chain(topic.body.iter())
            {
                for name in
                    placeholders(text).map_err(|()| GuideLoadError::UnclosedPlaceholder {
                        id: topic.id.clone(),
                    })?
                {
                    if GuideFact::from_placeholder(name).is_none() {
                        return Err(GuideLoadError::UnknownPlaceholder {
                            id: topic.id.clone(),
                            name: name.to_string(),
                            known: GuideFact::ALL
                                .iter()
                                .map(|f| f.placeholder().to_string())
                                .collect(),
                        });
                    }
                }
            }
        }

        Ok(Self {
            topics: file.topics,
        })
    }

    /// Panics on a broken bundled file, exactly as every other registry's
    /// `bundled` does: the shipped content is not a runtime condition any
    /// caller could recover from.
    pub fn bundled() -> Self {
        Self::from_ron_str(GUIDE_RON)
            .unwrap_or_else(|e| panic!("bundled data/guide.ron must load: {e}"))
    }

    /// Every topic, in the data file's order.
    pub fn ordered(&self) -> &[GuideTopic] {
        &self.topics
    }

    pub fn get(&self, id: &str) -> Option<&GuideTopic> {
        self.topics.iter().find(|t| t.id == id)
    }

    pub fn len(&self) -> usize {
        self.topics.len()
    }

    pub fn is_empty(&self) -> bool {
        self.topics.is_empty()
    }
}

/// Every `{name}` in `text`, in order. `Err(())` for a `{` that is never
/// closed — the caller turns that into the topic's error, since it is the
/// one that knows which topic it was reading.
///
/// A bare `}` is not an error: it is a closing brace in ordinary prose, and
/// nothing here needs it to mean anything.
fn placeholders(text: &str) -> Result<Vec<&str>, ()> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let close = after.find('}').ok_or(())?;
        found.push(&after[..close]);
        rest = &after[close + 1..];
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_guide_loads() {
        let guide = GuideRegistry::bundled();
        assert!(
            guide.len() >= 4,
            "the guide is the game explaining itself; {} topics is not that",
            guide.len()
        );
    }

    #[test]
    fn every_shipped_topic_asks_for_at_least_one_fact_somewhere_in_the_guide() {
        // Not per topic — a topic may legitimately be all prose. The claim
        // is that the guide as a whole is wired to the registries rather
        // than being a static essay, which is the reason this file exists.
        let guide = GuideRegistry::bundled();
        let asked: BTreeSet<&str> = guide
            .ordered()
            .iter()
            .flat_map(|t| {
                std::iter::once(&t.title)
                    .chain(std::iter::once(&t.tagline))
                    .chain(t.body.iter())
                    .flat_map(|text| placeholders(text).expect("shipped prose parses"))
            })
            .collect();
        assert!(
            asked.len() >= 8,
            "only {} of the {} facts are used by the shipped prose: {asked:?}",
            asked.len(),
            GuideFact::ALL.len()
        );
    }

    #[test]
    fn no_shipped_paragraph_states_a_bare_number_where_a_fact_exists() {
        // The rule the file's header states, enforced: prose says
        // "{starting_money}", never "3000". A digit in the prose is allowed
        // only where no fact could have supplied it — and today none is
        // needed at all, so the check is simply that there are none.
        let guide = GuideRegistry::bundled();
        for topic in guide.ordered() {
            for text in std::iter::once(&topic.title)
                .chain(std::iter::once(&topic.tagline))
                .chain(topic.body.iter())
            {
                // Placeholders are stripped first: `{payday_ticks}` has no
                // digits, but a future fact name might.
                let mut prose = text.clone();
                for fact in GuideFact::ALL {
                    prose = prose.replace(&format!("{{{}}}", fact.placeholder()), "");
                }
                assert!(
                    !prose.chars().any(|c| c.is_ascii_digit()),
                    "guide.ron's '{}' states a number in prose ({prose:?}) — ask for a \
                     fact instead, or the file goes stale the next time it changes",
                    topic.id
                );
            }
        }
    }

    #[test]
    fn a_typo_in_a_placeholder_is_a_load_error_and_not_a_brace_on_screen() {
        let text = r#"(
            schema_version: 1,
            topics: [
                GuideTopic(
                    id: "money",
                    title: "Money",
                    tagline: "You open with {starting_mony}.",
                    body: ["Spend it well."],
                ),
            ],
        )"#;
        match GuideRegistry::from_ron_str(text) {
            Err(GuideLoadError::UnknownPlaceholder { id, name, known }) => {
                assert_eq!(id, "money");
                assert_eq!(name, "starting_mony");
                assert!(known.contains(&"starting_money".to_string()));
            }
            other => panic!("expected an unknown-placeholder error, got {other:?}"),
        }
    }

    #[test]
    fn a_typo_in_a_title_is_a_load_error_too() {
        // The title is substituted like every other line on the page, so a
        // placeholder nothing answers would render as braces in the biggest
        // font the book owns. This test exists because the first version of
        // the loader checked the tagline and the body and not the title.
        let text = r#"(
            schema_version: 1,
            topics: [
                GuideTopic(
                    id: "money",
                    title: "Money, and your {starting_mony}",
                    tagline: "You open with {starting_money}.",
                    body: ["Spend it well."],
                ),
            ],
        )"#;
        match GuideRegistry::from_ron_str(text) {
            Err(GuideLoadError::UnknownPlaceholder { id, name, .. }) => {
                assert_eq!(id, "money");
                assert_eq!(name, "starting_mony");
            }
            other => panic!("expected an unknown-placeholder error, got {other:?}"),
        }
    }

    #[test]
    fn an_unclosed_brace_in_a_title_is_a_load_error_too() {
        let text = r#"(
            schema_version: 1,
            topics: [
                GuideTopic(
                    id: "money",
                    title: "Money, and your {starting_money",
                    tagline: "You open with {starting_money}.",
                    body: ["Spend it well."],
                ),
            ],
        )"#;
        assert!(matches!(
            GuideRegistry::from_ron_str(text),
            Err(GuideLoadError::UnclosedPlaceholder { .. })
        ));
    }

    #[test]
    fn an_unclosed_brace_is_a_load_error() {
        let text = r#"(
            schema_version: 1,
            topics: [
                GuideTopic(
                    id: "money",
                    title: "Money",
                    tagline: "You open with {starting_money.",
                    body: ["Spend it well."],
                ),
            ],
        )"#;
        assert!(matches!(
            GuideRegistry::from_ron_str(text),
            Err(GuideLoadError::UnclosedPlaceholder { .. })
        ));
    }

    #[test]
    fn a_blank_paragraph_is_a_load_error() {
        let text = r#"(
            schema_version: 1,
            topics: [
                GuideTopic(
                    id: "money",
                    title: "Money",
                    tagline: "Mind it.",
                    body: ["Spend it well.", "   "],
                ),
            ],
        )"#;
        match GuideRegistry::from_ron_str(text) {
            Err(GuideLoadError::BlankParagraph { id, index }) => {
                assert_eq!(id, "money");
                assert_eq!(index, 1);
            }
            other => panic!("expected a blank-paragraph error, got {other:?}"),
        }
    }

    #[test]
    fn every_fact_has_a_unique_placeholder_and_answers_to_it() {
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        for fact in GuideFact::ALL {
            assert!(
                seen.insert(fact.placeholder()),
                "two facts are written {{{}}}",
                fact.placeholder()
            );
            assert_eq!(GuideFact::from_placeholder(fact.placeholder()), Some(fact));
        }
    }
}
