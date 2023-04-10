use common::{
    character::CharacterId,
    rtsim::{Actor, FactionId, NpcId},
};
use hashbrown::HashMap;
use rand::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BinaryHeap;

// Factions have a larger 'social memory' than individual NPCs and so we allow
// them to have more sentiments
pub const FACTION_MAX_SENTIMENTS: usize = 1024;
pub const NPC_MAX_SENTIMENTS: usize = 128;

/// Magic factor used to control sentiment decay speed (note: higher = slower
/// decay, for implementation reasons).
const DECAY_TIME_FACTOR: f32 = 1.0; //6.0; TODO: Use this value when we're happy that everything is working as intended

/// The target that a sentiment is felt toward.
// NOTE: More could be added to this! For example:
// - Animal species (dislikes spiders?)
// - Kind of food (likes meat?)
// - Occupations (hatred of hunters or chefs?)
// - Ideologies (dislikes democracy, likes monarchy?)
// - etc.
#[derive(Copy, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Target {
    Character(CharacterId),
    Npc(NpcId),
    Faction(FactionId),
}

impl From<NpcId> for Target {
    fn from(npc: NpcId) -> Self { Self::Npc(npc) }
}
impl From<FactionId> for Target {
    fn from(faction: FactionId) -> Self { Self::Faction(faction) }
}
impl From<CharacterId> for Target {
    fn from(character: CharacterId) -> Self { Self::Character(character) }
}
impl From<Actor> for Target {
    fn from(actor: Actor) -> Self {
        match actor {
            Actor::Character(character) => Self::Character(character),
            Actor::Npc(npc) => Self::Npc(npc),
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Sentiments {
    #[serde(rename = "m")]
    map: HashMap<Target, Sentiment>,
}

impl Sentiments {
    /// Return the sentiment that is felt toward the given target.
    pub fn toward(&self, target: impl Into<Target>) -> Sentiment {
        self.map.get(&target.into()).copied().unwrap_or_default()
    }

    /// Change the sentiment toward the given target by the given amount,
    /// capping out at the given value.
    pub fn change_by(&mut self, target: impl Into<Target>, change: f32, cap: f32) {
        let target = target.into();
        self.map.entry(target).or_default().change_by(change, cap);
    }

    /// Progressively decay the sentiment back to a neutral sentiment.
    ///
    /// Note that sentiment get decay gets slower the harsher the sentiment is.
    /// You can calculate the **average** number of seconds required for a
    /// sentiment to neutral decay with the following formula:
    ///
    /// ```ignore
    /// seconds_until_neutrality = ((sentiment_value * 127 * DECAY_TIME_FACTOR) ^ 2) / 2
    /// ```
    ///
    /// For example, a positive (see [`Sentiment::POSITIVE`]) sentiment has a
    /// value of `0.2`, so we get
    ///
    /// ```ignore
    /// seconds_until_neutrality = ((0.1 * 127 * DECAY_TIME_FACTOR) ^ 2) / 2 = ~2,903 seconds, or 48 minutes
    /// ```
    ///
    /// Some 'common' sentiment decay times are as follows:
    ///
    /// - `POSITIVE`/`NEGATIVE`: ~48 minutes
    /// - `ALLY`/`RIVAL`: ~7 hours
    /// - `FRIEND`/`ENEMY`: ~29 hours
    /// - `HERO`/`VILLAIN`: ~65 hours
    pub fn decay(&mut self, rng: &mut impl Rng, dt: f32) {
        self.map.retain(|_, sentiment| {
            sentiment.decay(rng, dt);
            // We can eliminate redundant sentiments that don't need remembering
            !sentiment.is_redundant()
        });
    }

    /// Clean up sentiments to avoid them growing too large
    pub fn cleanup(&mut self, max_sentiments: usize) {
        if self.map.len() > max_sentiments {
            let mut sentiments = self.map
                .iter()
                // For each sentiment, calculate how valuable it is for us to remember.
                // For now, we just use the absolute value of the sentiment but later on we might want to favour
                // sentiments toward factions and other 'larger' groups over, say, sentiments toward players/other NPCs
                .map(|(tgt, sentiment)| (sentiment.positivity.unsigned_abs(), *tgt))
                .collect::<BinaryHeap<_>>();

            // Remove the superfluous sentiments
            for (_, tgt) in sentiments.drain().take(self.map.len() - max_sentiments) {
                self.map.remove(&tgt);
            }
        }
    }
}

#[derive(Copy, Clone, Default, Serialize, Deserialize)]
pub struct Sentiment {
    /// How positive the sentiment is.
    ///
    /// Using i8 to reduce on-disk memory footprint.
    /// Semantically, this value is -1 <= x <= 1.
    #[serde(rename = "p")]
    positivity: i8,
}

impl Sentiment {
    /// Substantial positive sentiments: NPC may go out of their way to help
    /// actors associated with the target, greet them, etc.
    pub const ALLY: f32 = 0.3;
    /// Very negative sentiments: NPC may confront the actor, get aggressive
    /// with them, or even use force against them.
    pub const ENEMY: f32 = -0.6;
    /// Very positive sentiments: NPC may join the actor as a companion,
    /// encourage them to join their faction, etc.
    pub const FRIEND: f32 = 0.6;
    /// Extremely positive sentiments: NPC may switch sides to join the actor's
    /// faction, protect them at all costs, turn against friends for them,
    /// etc. Verging on cult-like behaviour.
    pub const HERO: f32 = 0.8;
    /// Minor negative sentiments: NPC might be less willing to provide
    /// information, give worse trade deals, etc.
    pub const NEGATIVE: f32 = -0.1;
    /// Minor positive sentiments: NPC might be more willing to provide
    /// information, give better trade deals, etc.
    pub const POSITIVE: f32 = 0.1;
    /// Substantial positive sentiments: NPC may reject attempts to trade or
    /// avoid actors associated with the target, insult them, but will not
    /// use physical force.
    pub const RIVAL: f32 = -0.3;
    /// Extremely negative sentiments: NPC may aggressively persue or hunt down
    /// the actor, organise others around them to do the same, and will
    /// generally try to harm the actor in any way they can.
    pub const VILLAIN: f32 = -0.8;

    fn value(&self) -> f32 { self.positivity as f32 * (1.0 / 126.0) }

    fn change_by(&mut self, change: f32, cap: f32) {
        // There's a bit of ceremony here for two reasons:
        // 1) Very small changes should not be rounded to 0
        // 2) Sentiment should never (over/under)flow
        if change != 0.0 {
            let abs = (change * 126.0).abs().clamp(1.0, 126.0) as i8;
            let cap = (cap.abs().min(1.0) * 126.0) as i8;
            self.positivity = if change > 0.0 {
                self.positivity.saturating_add(abs).min(cap)
            } else {
                self.positivity.saturating_sub(abs).max(-cap)
            };
        }
    }

    fn decay(&mut self, rng: &mut impl Rng, dt: f32) {
        if self.positivity != 0 {
            // TODO: Find a slightly nicer way to have sentiment decay, perhaps even by
            // remembering the last interaction instead of constant updates.
            if rng.gen_bool(
                (1.0 / (self.positivity.unsigned_abs() as f32 * DECAY_TIME_FACTOR.powi(2) * dt))
                    as f64,
            ) {
                self.positivity -= self.positivity.signum();
            }
        }
    }

    /// Return `true` if the sentiment can be forgotten without changing
    /// anything (i.e: is entirely neutral, the default stance).
    fn is_redundant(&self) -> bool { self.positivity == 0 }

    /// Returns `true` if the sentiment has reached the given threshold.
    pub fn is(&self, val: f32) -> bool {
        if val > 0.0 {
            self.value() >= val
        } else {
            self.value() <= val
        }
    }
}
