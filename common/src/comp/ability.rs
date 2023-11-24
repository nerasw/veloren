use crate::{
    assets::{self, Asset},
    combat::{self, CombatEffect, DamageKind, Knockback},
    comp::{
        self, aura, beam, buff,
        character_state::AttackFilters,
        inventory::{
            item::{
                tool::{
                    AbilityContext, AbilityItem, AbilityKind, ContextualIndex, Stats, ToolKind,
                },
                ItemKind,
            },
            slot::EquipSlot,
            Inventory,
        },
        melee::{MeleeConstructor, MeleeConstructorKind},
        projectile::ProjectileConstructor,
        skillset::{
            skills::{self, Skill, SKILL_MODIFIERS},
            SkillSet,
        },
        Body, CharacterState, LightEmitter, StateUpdate,
    },
    resources::Secs,
    states::{
        behavior::JoinData,
        sprite_summon::SpriteSummonAnchor,
        utils::{AbilityInfo, ComboConsumption, ScalingKind, StageSection},
        *,
    },
    terrain::SpriteKind,
};
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use std::{borrow::Cow, convert::TryFrom, time::Duration};

use super::shockwave::ShockwaveDodgeable;

pub const BASE_ABILITY_LIMIT: usize = 5;
pub type AuxiliaryKey = (Option<ToolKind>, Option<ToolKind>);

// TODO: Potentially look into storing previous ability sets for weapon
// combinations and automatically reverting back to them on switching to that
// set of weapons. Consider after UI is set up and people weigh in on memory
// considerations.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActiveAbilities {
    pub guard: GuardAbility,
    pub primary: PrimaryAbility,
    pub secondary: SecondaryAbility,
    pub movement: MovementAbility,
    pub limit: Option<usize>,
    pub auxiliary_sets: HashMap<AuxiliaryKey, Vec<AuxiliaryAbility>>,
}

impl Component for ActiveAbilities {
    type Storage = DerefFlaggedStorage<Self, specs::VecStorage<Self>>;
}

impl Default for ActiveAbilities {
    fn default() -> Self {
        Self {
            guard: GuardAbility::Tool,
            primary: PrimaryAbility::Tool,
            secondary: SecondaryAbility::Tool,
            movement: MovementAbility::Species,
            limit: None,
            auxiliary_sets: HashMap::new(),
        }
    }
}

impl ActiveAbilities {
    pub fn from_auxiliary(
        auxiliary_sets: HashMap<AuxiliaryKey, Vec<AuxiliaryAbility>>,
        limit: Option<usize>,
    ) -> Self {
        // Discard any sets that exceed the limit
        ActiveAbilities {
            auxiliary_sets: auxiliary_sets
                .into_iter()
                .filter(|(_, set)| limit.map_or(true, |limit| set.len() == limit))
                .collect(),
            limit,
            ..Self::default()
        }
    }

    pub fn default_limited(limit: usize) -> Self {
        ActiveAbilities {
            limit: Some(limit),
            ..Default::default()
        }
    }

    pub fn change_ability(
        &mut self,
        slot: usize,
        auxiliary_key: AuxiliaryKey,
        new_ability: AuxiliaryAbility,
        inventory: Option<&Inventory>,
        skill_set: Option<&SkillSet>,
    ) {
        let auxiliary_set = self
            .auxiliary_sets
            .entry(auxiliary_key)
            .or_insert(Self::default_ability_set(inventory, skill_set, self.limit));
        if let Some(ability) = auxiliary_set.get_mut(slot) {
            *ability = new_ability;
        }
    }

    pub fn active_auxiliary_key(inv: Option<&Inventory>) -> AuxiliaryKey {
        let tool_kind = |slot| {
            inv.and_then(|inv| inv.equipped(slot))
                .and_then(|item| match &*item.kind() {
                    ItemKind::Tool(tool) => Some(tool.kind),
                    _ => None,
                })
        };

        (
            tool_kind(EquipSlot::ActiveMainhand),
            tool_kind(EquipSlot::ActiveOffhand),
        )
    }

    pub fn auxiliary_set(
        &self,
        inv: Option<&Inventory>,
        skill_set: Option<&SkillSet>,
    ) -> Cow<Vec<AuxiliaryAbility>> {
        let aux_key = Self::active_auxiliary_key(inv);

        self.auxiliary_sets
            .get(&aux_key)
            .map(Cow::Borrowed)
            .unwrap_or_else(|| Cow::Owned(Self::default_ability_set(inv, skill_set, self.limit)))
    }

    pub fn get_ability(
        &self,
        input: AbilityInput,
        inventory: Option<&Inventory>,
        skill_set: Option<&SkillSet>,
    ) -> Ability {
        match input {
            AbilityInput::Guard => self.guard.into(),
            AbilityInput::Primary => self.primary.into(),
            AbilityInput::Secondary => self.secondary.into(),
            AbilityInput::Movement => self.movement.into(),
            AbilityInput::Auxiliary(index) => self
                .auxiliary_set(inventory, skill_set)
                .get(index)
                .copied()
                .map(|a| a.into())
                .unwrap_or(Ability::Empty),
        }
    }

    /// Returns the CharacterAbility from an ability input, and also whether the
    /// ability was from a weapon wielded in the offhand
    pub fn activate_ability(
        &self,
        input: AbilityInput,
        inv: Option<&Inventory>,
        skill_set: &SkillSet,
        body: Option<&Body>,
        char_state: Option<&CharacterState>,
        context: &AbilityContext,
        // bool is from_offhand
    ) -> Option<(CharacterAbility, bool, SpecifiedAbility)> {
        let ability = self.get_ability(input, inv, Some(skill_set));

        let ability_set = |equip_slot| {
            inv.and_then(|inv| inv.equipped(equip_slot))
                .map(|i| &i.item_config_expect().abilities)
        };

        let scale_ability = |ability: CharacterAbility, equip_slot| {
            let tool_kind = inv
                .and_then(|inv| inv.equipped(equip_slot))
                .and_then(|item| match &*item.kind() {
                    ItemKind::Tool(tool) => Some(tool.kind),
                    _ => None,
                });
            ability.adjusted_by_skills(skill_set, tool_kind)
        };

        let spec_ability = |context_index| SpecifiedAbility {
            ability,
            context_index,
        };

        match ability {
            Ability::ToolGuard => ability_set(EquipSlot::ActiveMainhand)
                .and_then(|abilities| {
                    abilities
                        .guard(Some(skill_set), context)
                        .map(|(a, i)| (a.ability.clone(), i))
                })
                .map(|(ability, i)| {
                    (
                        scale_ability(ability, EquipSlot::ActiveMainhand),
                        true,
                        spec_ability(i),
                    )
                })
                .or_else(|| {
                    ability_set(EquipSlot::ActiveOffhand)
                        .and_then(|abilities| {
                            abilities
                                .guard(Some(skill_set), context)
                                .map(|(a, i)| (a.ability.clone(), i))
                        })
                        .map(|(ability, i)| {
                            (
                                scale_ability(ability, EquipSlot::ActiveOffhand),
                                false,
                                spec_ability(i),
                            )
                        })
                }),
            Ability::ToolPrimary => ability_set(EquipSlot::ActiveMainhand)
                .and_then(|abilities| {
                    abilities
                        .primary(Some(skill_set), context)
                        .map(|(a, i)| (a.ability.clone(), i))
                })
                .map(|(ability, i)| {
                    (
                        scale_ability(ability, EquipSlot::ActiveMainhand),
                        false,
                        spec_ability(i),
                    )
                }),
            Ability::ToolSecondary => ability_set(EquipSlot::ActiveOffhand)
                .and_then(|abilities| {
                    abilities
                        .secondary(Some(skill_set), context)
                        .map(|(a, i)| (a.ability.clone(), i))
                })
                .map(|(ability, i)| {
                    (
                        scale_ability(ability, EquipSlot::ActiveOffhand),
                        true,
                        spec_ability(i),
                    )
                })
                .or_else(|| {
                    ability_set(EquipSlot::ActiveMainhand)
                        .and_then(|abilities| {
                            abilities
                                .secondary(Some(skill_set), context)
                                .map(|(a, i)| (a.ability.clone(), i))
                        })
                        .map(|(ability, i)| {
                            (
                                scale_ability(ability, EquipSlot::ActiveMainhand),
                                false,
                                spec_ability(i),
                            )
                        })
                }),
            Ability::SpeciesMovement => matches!(body, Some(Body::Humanoid(_)))
                .then(|| CharacterAbility::default_roll(char_state))
                .map(|ability| {
                    (
                        ability.adjusted_by_skills(skill_set, None),
                        false,
                        spec_ability(None),
                    )
                }),
            Ability::MainWeaponAux(index) => ability_set(EquipSlot::ActiveMainhand)
                .and_then(|abilities| {
                    abilities
                        .auxiliary(index, Some(skill_set), context)
                        .map(|(a, i)| (a.ability.clone(), i))
                })
                .map(|(ability, i)| {
                    (
                        scale_ability(ability, EquipSlot::ActiveMainhand),
                        false,
                        spec_ability(i),
                    )
                }),
            Ability::OffWeaponAux(index) => ability_set(EquipSlot::ActiveOffhand)
                .and_then(|abilities| {
                    abilities
                        .auxiliary(index, Some(skill_set), context)
                        .map(|(a, i)| (a.ability.clone(), i))
                })
                .map(|(ability, i)| {
                    (
                        scale_ability(ability, EquipSlot::ActiveOffhand),
                        true,
                        spec_ability(i),
                    )
                }),
            Ability::Empty => None,
        }
    }

    pub fn iter_available_abilities<'a>(
        inv: Option<&'a Inventory>,
        skill_set: Option<&'a SkillSet>,
        equip_slot: EquipSlot,
    ) -> impl Iterator<Item = usize> + 'a {
        inv.and_then(|inv| inv.equipped(equip_slot))
            .into_iter()
            .flat_map(|i| &i.item_config_expect().abilities.abilities)
            .enumerate()
            .filter_map(move |(i, a)| match a {
                AbilityKind::Simple(skill, _) => skill
                    .map_or(true, |s| skill_set.map_or(false, |ss| ss.has_skill(s)))
                    .then_some(i),
                AbilityKind::Contextualized {
                    pseudo_id: _,
                    abilities,
                } => abilities
                    .iter()
                    .any(|(_contexts, (skill, _))| {
                        skill.map_or(true, |s| skill_set.map_or(false, |ss| ss.has_skill(s)))
                    })
                    .then_some(i),
            })
    }

    fn default_ability_set<'a>(
        inv: Option<&'a Inventory>,
        skill_set: Option<&'a SkillSet>,
        limit: Option<usize>,
    ) -> Vec<AuxiliaryAbility> {
        let mut iter = Self::iter_available_abilities(inv, skill_set, EquipSlot::ActiveMainhand)
            .map(AuxiliaryAbility::MainWeapon)
            .chain(
                Self::iter_available_abilities(inv, skill_set, EquipSlot::ActiveOffhand)
                    .map(AuxiliaryAbility::OffWeapon),
            );

        if let Some(limit) = limit {
            (0..limit)
                .map(|_| iter.next().unwrap_or(AuxiliaryAbility::Empty))
                .collect()
        } else {
            iter.collect()
        }
    }
}

pub enum AbilityInput {
    Guard,
    Primary,
    Secondary,
    Movement,
    Auxiliary(usize),
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum Ability {
    ToolGuard,
    ToolPrimary,
    ToolSecondary,
    SpeciesMovement,
    MainWeaponAux(usize),
    OffWeaponAux(usize),
    Empty,
    /* For future use
     * ArmorAbility(usize), */
}

impl Ability {
    pub fn ability_id<'a>(
        self,
        inv: Option<&'a Inventory>,
        skillset: Option<&'a SkillSet>,
        context: &AbilityContext,
    ) -> Option<&'a str> {
        let ability_set = |equip_slot| {
            inv.and_then(|inv| inv.equipped(equip_slot))
                .map(|i| &i.item_config_expect().abilities)
        };

        let contextual_id = |kind: Option<&'a AbilityKind<_>>| -> Option<&'a str> {
            if let Some(AbilityKind::Contextualized {
                pseudo_id,
                abilities: _,
            }) = kind
            {
                Some(pseudo_id.as_str())
            } else {
                None
            }
        };

        match self {
            Ability::ToolGuard => ability_set(EquipSlot::ActiveMainhand)
                .and_then(|abilities| {
                    abilities
                        .guard(skillset, context)
                        .map(|a| a.0.id.as_str())
                        .or_else(|| {
                            abilities
                                .guard
                                .as_ref()
                                .and_then(|g| contextual_id(Some(g)))
                        })
                })
                .or_else(|| {
                    ability_set(EquipSlot::ActiveOffhand).and_then(|abilities| {
                        abilities
                            .guard(skillset, context)
                            .map(|a| a.0.id.as_str())
                            .or_else(|| {
                                abilities
                                    .guard
                                    .as_ref()
                                    .and_then(|g| contextual_id(Some(g)))
                            })
                    })
                }),
            Ability::ToolPrimary => ability_set(EquipSlot::ActiveMainhand).and_then(|abilities| {
                abilities
                    .primary(skillset, context)
                    .map(|a| a.0.id.as_str())
                    .or_else(|| contextual_id(Some(&abilities.primary)))
            }),
            Ability::ToolSecondary => ability_set(EquipSlot::ActiveOffhand)
                .and_then(|abilities| {
                    abilities
                        .secondary(skillset, context)
                        .map(|a| a.0.id.as_str())
                        .or_else(|| contextual_id(Some(&abilities.secondary)))
                })
                .or_else(|| {
                    ability_set(EquipSlot::ActiveMainhand).and_then(|abilities| {
                        abilities
                            .secondary(skillset, context)
                            .map(|a| a.0.id.as_str())
                            .or_else(|| contextual_id(Some(&abilities.secondary)))
                    })
                }),
            Ability::SpeciesMovement => None, // TODO: Make not None
            Ability::MainWeaponAux(index) => {
                ability_set(EquipSlot::ActiveMainhand).and_then(|abilities| {
                    abilities
                        .auxiliary(index, skillset, context)
                        .map(|a| a.0.id.as_str())
                        .or_else(|| contextual_id(abilities.abilities.get(index)))
                })
            },
            Ability::OffWeaponAux(index) => {
                ability_set(EquipSlot::ActiveOffhand).and_then(|abilities| {
                    abilities
                        .auxiliary(index, skillset, context)
                        .map(|a| a.0.id.as_str())
                        .or_else(|| contextual_id(abilities.abilities.get(index)))
                })
            },
            Ability::Empty => None,
        }
    }

    pub fn is_from_tool(&self) -> bool {
        match self {
            Ability::ToolPrimary
            | Ability::ToolSecondary
            | Ability::MainWeaponAux(_)
            | Ability::OffWeaponAux(_)
            | Ability::ToolGuard => true,
            Ability::SpeciesMovement | Ability::Empty => false,
        }
    }
}
#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub enum GuardAbility {
    Tool,
    Empty,
}

impl From<GuardAbility> for Ability {
    fn from(guard: GuardAbility) -> Self {
        match guard {
            GuardAbility::Tool => Ability::ToolGuard,
            GuardAbility::Empty => Ability::Empty,
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct SpecifiedAbility {
    pub ability: Ability,
    pub context_index: Option<ContextualIndex>,
}

impl SpecifiedAbility {
    pub fn ability_id(self, inv: Option<&Inventory>) -> Option<&str> {
        let ability_set = |equip_slot| {
            inv.and_then(|inv| inv.equipped(equip_slot))
                .map(|i| &i.item_config_expect().abilities)
        };

        fn ability_id(spec_ability: SpecifiedAbility, ability: &AbilityKind<AbilityItem>) -> &str {
            match ability {
                AbilityKind::Simple(_, a) => a.id.as_str(),
                AbilityKind::Contextualized {
                    pseudo_id,
                    abilities,
                } => spec_ability
                    .context_index
                    .and_then(|i| abilities.get(i.0))
                    .map_or(pseudo_id.as_str(), |(_, (_, a))| a.id.as_str()),
            }
        }

        match self.ability {
            Ability::ToolPrimary => ability_set(EquipSlot::ActiveMainhand)
                .map(|abilities| ability_id(self, &abilities.primary)),
            Ability::ToolSecondary => ability_set(EquipSlot::ActiveOffhand)
                .map(|abilities| ability_id(self, &abilities.secondary))
                .or_else(|| {
                    ability_set(EquipSlot::ActiveMainhand)
                        .map(|abilities| ability_id(self, &abilities.secondary))
                }),
            Ability::ToolGuard => ability_set(EquipSlot::ActiveMainhand)
                .and_then(|abilities| abilities.guard.as_ref().map(|a| ability_id(self, a)))
                .or_else(|| {
                    ability_set(EquipSlot::ActiveOffhand)
                        .and_then(|abilities| abilities.guard.as_ref().map(|a| ability_id(self, a)))
                }),
            Ability::SpeciesMovement => None, // TODO: Make not None
            Ability::MainWeaponAux(index) => ability_set(EquipSlot::ActiveMainhand)
                .and_then(|abilities| abilities.abilities.get(index).map(|a| ability_id(self, a))),
            Ability::OffWeaponAux(index) => ability_set(EquipSlot::ActiveOffhand)
                .and_then(|abilities| abilities.abilities.get(index).map(|a| ability_id(self, a))),
            Ability::Empty => None,
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub enum PrimaryAbility {
    Tool,
    Empty,
}

impl From<PrimaryAbility> for Ability {
    fn from(primary: PrimaryAbility) -> Self {
        match primary {
            PrimaryAbility::Tool => Ability::ToolPrimary,
            PrimaryAbility::Empty => Ability::Empty,
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub enum SecondaryAbility {
    Tool,
    Empty,
}

impl From<SecondaryAbility> for Ability {
    fn from(primary: SecondaryAbility) -> Self {
        match primary {
            SecondaryAbility::Tool => Ability::ToolSecondary,
            SecondaryAbility::Empty => Ability::Empty,
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub enum MovementAbility {
    Species,
    Empty,
}

impl From<MovementAbility> for Ability {
    fn from(primary: MovementAbility) -> Self {
        match primary {
            MovementAbility::Species => Ability::SpeciesMovement,
            MovementAbility::Empty => Ability::Empty,
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub enum AuxiliaryAbility {
    MainWeapon(usize),
    OffWeapon(usize),
    Empty,
}

impl From<AuxiliaryAbility> for Ability {
    fn from(primary: AuxiliaryAbility) -> Self {
        match primary {
            AuxiliaryAbility::MainWeapon(i) => Ability::MainWeaponAux(i),
            AuxiliaryAbility::OffWeapon(i) => Ability::OffWeaponAux(i),
            AuxiliaryAbility::Empty => Ability::Empty,
        }
    }
}

/// A lighter form of character state to pass around as needed for frontend
/// purposes
// Only add to this enum as needed for frontends, not necessary to immediately
// add a variant here when adding a new character state
#[derive(Copy, Clone, Hash, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum CharacterAbilityType {
    BasicMelee(StageSection),
    BasicRanged,
    Boost,
    ChargedMelee(StageSection),
    ChargedRanged,
    DashMelee(StageSection),
    BasicBlock,
    ComboMelee(StageSection, u32),
    ComboMelee2(StageSection),
    FinisherMelee(StageSection),
    DiveMelee(StageSection),
    RiposteMelee(StageSection),
    RapidMelee(StageSection),
    LeapMelee(StageSection),
    LeapShockwave(StageSection),
    Music(StageSection),
    Shockwave,
    BasicBeam,
    RepeaterRanged,
    BasicAura,
    SelfBuff,
    Other,
}

impl From<&CharacterState> for CharacterAbilityType {
    fn from(state: &CharacterState) -> Self {
        match state {
            CharacterState::BasicMelee(data) => Self::BasicMelee(data.stage_section),
            CharacterState::BasicRanged(_) => Self::BasicRanged,
            CharacterState::Boost(_) => Self::Boost,
            CharacterState::DashMelee(data) => Self::DashMelee(data.stage_section),
            CharacterState::BasicBlock(_) => Self::BasicBlock,
            CharacterState::LeapMelee(data) => Self::LeapMelee(data.stage_section),
            CharacterState::LeapShockwave(data) => Self::LeapShockwave(data.stage_section),
            CharacterState::ComboMelee(data) => Self::ComboMelee(data.stage_section, data.stage),
            CharacterState::ComboMelee2(data) => Self::ComboMelee2(data.stage_section),
            CharacterState::FinisherMelee(data) => Self::FinisherMelee(data.stage_section),
            CharacterState::DiveMelee(data) => Self::DiveMelee(data.stage_section),
            CharacterState::RiposteMelee(data) => Self::RiposteMelee(data.stage_section),
            CharacterState::RapidMelee(data) => Self::RapidMelee(data.stage_section),
            CharacterState::ChargedMelee(data) => Self::ChargedMelee(data.stage_section),
            CharacterState::ChargedRanged(_) => Self::ChargedRanged,
            CharacterState::Shockwave(_) => Self::Shockwave,
            CharacterState::BasicBeam(_) => Self::BasicBeam,
            CharacterState::RepeaterRanged(_) => Self::RepeaterRanged,
            CharacterState::BasicAura(_) => Self::BasicAura,
            CharacterState::SelfBuff(_) => Self::SelfBuff,
            CharacterState::Music(data) => Self::Music(data.stage_section),
            CharacterState::Idle(_)
            | CharacterState::Climb(_)
            | CharacterState::Sit
            | CharacterState::Dance
            | CharacterState::Talk
            | CharacterState::Glide(_)
            | CharacterState::GlideWield(_)
            | CharacterState::Stunned(_)
            | CharacterState::Equipping(_)
            | CharacterState::Wielding(_)
            | CharacterState::Roll(_)
            | CharacterState::Blink(_)
            | CharacterState::BasicSummon(_)
            | CharacterState::SpriteSummon(_)
            | CharacterState::UseItem(_)
            | CharacterState::SpriteInteract(_)
            | CharacterState::Skate(_)
            | CharacterState::Wallrun(_) => Self::Other,
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// For documentation on individual fields, see the corresponding character
/// state file in 'common/src/states/'
pub enum CharacterAbility {
    BasicMelee {
        energy_cost: f32,
        buildup_duration: f32,
        swing_duration: f32,
        recover_duration: f32,
        melee_constructor: MeleeConstructor,
        ori_modifier: f32,
        #[serde(default)]
        meta: AbilityMeta,
    },
    BasicRanged {
        energy_cost: f32,
        buildup_duration: f32,
        recover_duration: f32,
        projectile: ProjectileConstructor,
        projectile_body: Body,
        projectile_light: Option<LightEmitter>,
        projectile_speed: f32,
        num_projectiles: u32,
        projectile_spread: f32,
        damage_effect: Option<CombatEffect>,
        move_efficiency: f32,
        #[serde(default)]
        meta: AbilityMeta,
    },
    RepeaterRanged {
        energy_cost: f32,
        buildup_duration: f32,
        shoot_duration: f32,
        recover_duration: f32,
        max_speed: f32,
        half_speed_at: u32,
        projectile: ProjectileConstructor,
        projectile_body: Body,
        projectile_light: Option<LightEmitter>,
        projectile_speed: f32,
        damage_effect: Option<CombatEffect>,
        aoe: Option<repeater_ranged::AOE>,
        #[serde(default)]
        meta: AbilityMeta,
    },
    Boost {
        movement_duration: f32,
        only_up: bool,
        speed: f32,
        max_exit_velocity: f32,
        #[serde(default)]
        meta: AbilityMeta,
    },
    DashMelee {
        energy_cost: f32,
        energy_drain: f32,
        forward_speed: f32,
        buildup_duration: f32,
        charge_duration: f32,
        swing_duration: f32,
        recover_duration: f32,
        melee_constructor: MeleeConstructor,
        ori_modifier: f32,
        charge_through: bool,
        #[serde(default)]
        meta: AbilityMeta,
    },
    BasicBlock {
        buildup_duration: f32,
        recover_duration: f32,
        max_angle: f32,
        block_strength: f32,
        parry_window: basic_block::ParryWindow,
        energy_cost: f32,
        energy_regen: f32,
        can_hold: bool,
        blocked_attacks: AttackFilters,
        #[serde(default)]
        meta: AbilityMeta,
    },
    Roll {
        energy_cost: f32,
        buildup_duration: f32,
        movement_duration: f32,
        recover_duration: f32,
        roll_strength: f32,
        attack_immunities: AttackFilters,
        #[serde(default)]
        meta: AbilityMeta,
    },
    ComboMelee {
        stage_data: Vec<combo_melee::Stage<f32>>,
        initial_energy_gain: f32,
        max_energy_gain: f32,
        energy_increase: f32,
        speed_increase: f32,
        max_speed_increase: f32,
        scales_from_combo: u32,
        ori_modifier: f32,
        #[serde(default)]
        meta: AbilityMeta,
    },
    ComboMelee2 {
        strikes: Vec<combo_melee2::Strike<f32>>,
        energy_cost_per_strike: f32,
        #[serde(default)]
        auto_progress: bool,
        #[serde(default)]
        meta: AbilityMeta,
    },
    LeapMelee {
        energy_cost: f32,
        buildup_duration: f32,
        movement_duration: f32,
        swing_duration: f32,
        recover_duration: f32,
        melee_constructor: MeleeConstructor,
        forward_leap_strength: f32,
        vertical_leap_strength: f32,
        damage_effect: Option<CombatEffect>,
        specifier: Option<leap_melee::FrontendSpecifier>,
        #[serde(default)]
        meta: AbilityMeta,
    },
    LeapShockwave {
        energy_cost: f32,
        buildup_duration: f32,
        movement_duration: f32,
        swing_duration: f32,
        recover_duration: f32,
        damage: f32,
        poise_damage: f32,
        knockback: Knockback,
        shockwave_angle: f32,
        shockwave_vertical_angle: f32,
        shockwave_speed: f32,
        shockwave_duration: f32,
        dodgeable: ShockwaveDodgeable,
        move_efficiency: f32,
        damage_kind: DamageKind,
        specifier: comp::shockwave::FrontendSpecifier,
        damage_effect: Option<CombatEffect>,
        forward_leap_strength: f32,
        vertical_leap_strength: f32,
        #[serde(default)]
        meta: AbilityMeta,
    },
    ChargedMelee {
        energy_cost: f32,
        energy_drain: f32,
        buildup_strike: Option<(f32, MeleeConstructor)>,
        charge_duration: f32,
        swing_duration: f32,
        hit_timing: f32,
        recover_duration: f32,
        melee_constructor: MeleeConstructor,
        specifier: Option<charged_melee::FrontendSpecifier>,
        damage_effect: Option<CombatEffect>,
        #[serde(default)]
        additional_combo: i32,
        #[serde(default)]
        meta: AbilityMeta,
    },
    ChargedRanged {
        energy_cost: f32,
        energy_drain: f32,
        initial_regen: f32,
        scaled_regen: f32,
        initial_damage: f32,
        scaled_damage: f32,
        initial_knockback: f32,
        scaled_knockback: f32,
        buildup_duration: f32,
        charge_duration: f32,
        recover_duration: f32,
        projectile_body: Body,
        projectile_light: Option<LightEmitter>,
        initial_projectile_speed: f32,
        scaled_projectile_speed: f32,
        damage_effect: Option<CombatEffect>,
        move_speed: f32,
        #[serde(default)]
        meta: AbilityMeta,
    },
    Shockwave {
        energy_cost: f32,
        buildup_duration: f32,
        swing_duration: f32,
        recover_duration: f32,
        damage: f32,
        poise_damage: f32,
        knockback: Knockback,
        shockwave_angle: f32,
        shockwave_vertical_angle: f32,
        shockwave_speed: f32,
        shockwave_duration: f32,
        dodgeable: ShockwaveDodgeable,
        move_efficiency: f32,
        damage_kind: DamageKind,
        specifier: comp::shockwave::FrontendSpecifier,
        ori_rate: f32,
        damage_effect: Option<CombatEffect>,
        #[serde(default)]
        meta: AbilityMeta,
    },
    BasicBeam {
        buildup_duration: f32,
        recover_duration: f32,
        beam_duration: f64,
        damage: f32,
        tick_rate: f32,
        range: f32,
        max_angle: f32,
        damage_effect: Option<CombatEffect>,
        energy_regen: f32,
        energy_drain: f32,
        ori_rate: f32,
        specifier: beam::FrontendSpecifier,
        #[serde(default)]
        meta: AbilityMeta,
    },
    BasicAura {
        buildup_duration: f32,
        cast_duration: f32,
        recover_duration: f32,
        targets: combat::GroupTarget,
        auras: Vec<aura::AuraBuffConstructor>,
        aura_duration: Secs,
        range: f32,
        energy_cost: f32,
        scales_with_combo: bool,
        specifier: Option<aura::Specifier>,
        #[serde(default)]
        meta: AbilityMeta,
    },
    Blink {
        buildup_duration: f32,
        recover_duration: f32,
        max_range: f32,
        #[serde(default)]
        meta: AbilityMeta,
    },
    BasicSummon {
        buildup_duration: f32,
        cast_duration: f32,
        recover_duration: f32,
        summon_amount: u32,
        summon_distance: (f32, f32),
        summon_info: basic_summon::SummonInfo,
        duration: Option<Duration>,
        #[serde(default)]
        meta: AbilityMeta,
    },
    SelfBuff {
        buildup_duration: f32,
        cast_duration: f32,
        recover_duration: f32,
        buff_kind: buff::BuffKind,
        buff_strength: f32,
        buff_duration: Option<Secs>,
        energy_cost: f32,
        #[serde(default = "default_true")]
        enforced_limit: bool,
        #[serde(default)]
        combo_cost: u32,
        combo_scaling: Option<ScalingKind>,
        #[serde(default)]
        meta: AbilityMeta,
        specifier: Option<self_buff::FrontendSpecifier>,
    },
    SpriteSummon {
        buildup_duration: f32,
        cast_duration: f32,
        recover_duration: f32,
        sprite: SpriteKind,
        del_timeout: Option<(f32, f32)>,
        summon_distance: (f32, f32),
        sparseness: f64,
        angle: f32,
        #[serde(default)]
        anchor: SpriteSummonAnchor,
        #[serde(default)]
        move_efficiency: f32,
        #[serde(default)]
        meta: AbilityMeta,
    },
    Music {
        play_duration: f32,
        ori_modifier: f32,
        #[serde(default)]
        meta: AbilityMeta,
    },
    FinisherMelee {
        energy_cost: f32,
        buildup_duration: f32,
        swing_duration: f32,
        recover_duration: f32,
        melee_constructor: MeleeConstructor,
        minimum_combo: u32,
        scaling: Option<finisher_melee::Scaling>,
        #[serde(default)]
        combo_consumption: ComboConsumption,
        #[serde(default)]
        meta: AbilityMeta,
    },
    DiveMelee {
        energy_cost: f32,
        vertical_speed: f32,
        buildup_duration: Option<f32>,
        movement_duration: f32,
        swing_duration: f32,
        recover_duration: f32,
        melee_constructor: MeleeConstructor,
        max_scaling: f32,
        #[serde(default)]
        meta: AbilityMeta,
    },
    RiposteMelee {
        energy_cost: f32,
        buildup_duration: f32,
        swing_duration: f32,
        recover_duration: f32,
        melee_constructor: MeleeConstructor,
        #[serde(default)]
        meta: AbilityMeta,
    },
    RapidMelee {
        buildup_duration: f32,
        swing_duration: f32,
        recover_duration: f32,
        energy_cost: f32,
        max_strikes: Option<u32>,
        melee_constructor: MeleeConstructor,
        move_modifier: f32,
        ori_modifier: f32,
        frontend_specifier: Option<rapid_melee::FrontendSpecifier>,
        #[serde(default)]
        minimum_combo: u32,
        #[serde(default)]
        meta: AbilityMeta,
    },
}

impl Default for CharacterAbility {
    fn default() -> Self {
        CharacterAbility::BasicMelee {
            energy_cost: 0.0,
            buildup_duration: 0.25,
            swing_duration: 0.25,
            recover_duration: 0.5,
            melee_constructor: MeleeConstructor {
                kind: MeleeConstructorKind::Slash {
                    damage: 1.0,
                    knockback: 0.0,
                    poise: 0.0,
                    energy_regen: 0.0,
                },
                scaled: None,
                range: 3.5,
                angle: 15.0,
                multi_target: None,
                damage_effect: None,
                simultaneous_hits: 1,
                combo_gain: 1,
            },
            ori_modifier: 1.0,
            meta: Default::default(),
        }
    }
}

impl Asset for CharacterAbility {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

impl CharacterAbility {
    /// Attempts to fulfill requirements, mutating `update` (taking energy) if
    /// applicable.
    pub fn requirements_paid(&self, data: &JoinData, update: &mut StateUpdate) -> bool {
        let from_meta = {
            let AbilityMeta { requirements, .. } = self.ability_meta();
            requirements.requirements_met(data.stance)
        };
        from_meta
            && match self {
                CharacterAbility::Roll { energy_cost, .. } => {
                    data.physics.on_ground.is_some()
                        && data.inputs.move_dir.magnitude_squared() > 0.25
                        && update.energy.try_change_by(-*energy_cost).is_ok()
                },
                CharacterAbility::DashMelee { energy_cost, .. }
                | CharacterAbility::BasicMelee { energy_cost, .. }
                | CharacterAbility::BasicRanged { energy_cost, .. }
                | CharacterAbility::ChargedRanged { energy_cost, .. }
                | CharacterAbility::ChargedMelee { energy_cost, .. }
                | CharacterAbility::Shockwave { energy_cost, .. }
                | CharacterAbility::BasicBlock { energy_cost, .. }
                | CharacterAbility::RiposteMelee { energy_cost, .. }
                | CharacterAbility::ComboMelee2 {
                    energy_cost_per_strike: energy_cost,
                    ..
                } => update.energy.try_change_by(-*energy_cost).is_ok(),
                // Consumes energy within state, so value only checked before entering state
                CharacterAbility::RepeaterRanged { energy_cost, .. } => {
                    update.energy.current() >= *energy_cost
                },
                CharacterAbility::LeapMelee { energy_cost, .. }
                | CharacterAbility::LeapShockwave { energy_cost, .. } => {
                    update.vel.0.z >= 0.0 && update.energy.try_change_by(-*energy_cost).is_ok()
                },
                CharacterAbility::BasicAura {
                    energy_cost,
                    scales_with_combo,
                    ..
                } => {
                    ((*scales_with_combo && data.combo.map_or(false, |c| c.counter() > 0))
                        | !*scales_with_combo)
                        && update.energy.try_change_by(-*energy_cost).is_ok()
                },
                CharacterAbility::FinisherMelee {
                    energy_cost,
                    minimum_combo,
                    ..
                }
                | CharacterAbility::RapidMelee {
                    energy_cost,
                    minimum_combo,
                    ..
                }
                | CharacterAbility::SelfBuff {
                    energy_cost,
                    combo_cost: minimum_combo,
                    ..
                } => {
                    data.combo.map_or(false, |c| c.counter() >= *minimum_combo)
                        && update.energy.try_change_by(-*energy_cost).is_ok()
                },
                CharacterAbility::DiveMelee {
                    buildup_duration,
                    energy_cost,
                    ..
                } => {
                    // If either in the air or is on ground and able to be activated from
                    // ground.
                    //
                    // NOTE: there is a check in CharacterState::from below that must be kept in
                    // sync with the conditions here (it determines whether this starts in a
                    // movement or buildup stage).
                    (data.physics.on_ground.is_none() || buildup_duration.is_some())
                        && update.energy.try_change_by(-*energy_cost).is_ok()
                },
                CharacterAbility::ComboMelee { .. }
                | CharacterAbility::Boost { .. }
                | CharacterAbility::BasicBeam { .. }
                | CharacterAbility::Blink { .. }
                | CharacterAbility::Music { .. }
                | CharacterAbility::BasicSummon { .. }
                | CharacterAbility::SpriteSummon { .. } => true,
            }
    }

    pub fn default_roll(current_state: Option<&CharacterState>) -> CharacterAbility {
        let remaining_recover = if let Some(char_state) = current_state {
            if matches!(char_state.stage_section(), Some(StageSection::Recover)) {
                let timer = char_state.timer().map_or(0.0, |t| t.as_secs_f32());
                let recover_duration = char_state
                    .durations()
                    .and_then(|durs| durs.recover)
                    .map_or(timer, |rec| rec.as_secs_f32());
                recover_duration - timer
            } else {
                0.0
            }
        } else {
            0.0
        };
        CharacterAbility::Roll {
            // Energy cost increased by
            energy_cost: 12.0 + remaining_recover * 100.0,
            buildup_duration: 0.05,
            movement_duration: 0.33,
            recover_duration: 0.125,
            roll_strength: 3.0,
            attack_immunities: AttackFilters {
                melee: true,
                projectiles: false,
                beams: true,
                ground_shockwaves: false,
                air_shockwaves: true,
                explosions: true,
            },
            meta: Default::default(),
        }
    }

    #[must_use]
    pub fn adjusted_by_stats(mut self, stats: Stats) -> Self {
        use CharacterAbility::*;
        match self {
            BasicMelee {
                ref mut energy_cost,
                ref mut buildup_duration,
                ref mut swing_duration,
                ref mut recover_duration,
                ref mut melee_constructor,
                ori_modifier: _,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *swing_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
                *melee_constructor = melee_constructor.adjusted_by_stats(stats);
            },
            BasicRanged {
                ref mut energy_cost,
                ref mut buildup_duration,
                ref mut recover_duration,
                ref mut projectile,
                projectile_body: _,
                projectile_light: _,
                ref mut projectile_speed,
                num_projectiles: _,
                projectile_spread: _,
                damage_effect: _,
                move_efficiency: _,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *projectile = projectile.modified_projectile(stats.power, 1_f32, 1_f32);
                *projectile_speed *= stats.range;
                *energy_cost /= stats.energy_efficiency;
            },
            RepeaterRanged {
                ref mut energy_cost,
                ref mut buildup_duration,
                ref mut shoot_duration,
                ref mut recover_duration,
                max_speed: _,
                half_speed_at: _,
                ref mut projectile,
                projectile_body: _,
                projectile_light: _,
                ref mut projectile_speed,
                damage_effect: _,
                aoe: _,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *shoot_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *projectile = projectile.modified_projectile(stats.power, 1_f32, 1_f32);
                *projectile_speed *= stats.range;
                *energy_cost /= stats.energy_efficiency;
            },
            Boost {
                ref mut movement_duration,
                only_up: _,
                speed: ref mut boost_speed,
                max_exit_velocity: _,
                meta: _,
            } => {
                *movement_duration /= stats.speed;
                *boost_speed *= stats.power;
            },
            DashMelee {
                ref mut energy_cost,
                ref mut energy_drain,
                forward_speed: _,
                ref mut buildup_duration,
                charge_duration: _,
                ref mut swing_duration,
                ref mut recover_duration,
                ref mut melee_constructor,
                ori_modifier: _,
                charge_through: _,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *swing_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
                *energy_drain /= stats.energy_efficiency;
                *melee_constructor = melee_constructor.adjusted_by_stats(stats);
            },
            BasicBlock {
                ref mut buildup_duration,
                ref mut recover_duration,
                // Do we want angle to be adjusted by range?
                max_angle: _,
                // Block strength explicitly not modified by power, that will be a separate stat
                block_strength: _,
                parry_window: _,
                ref mut energy_cost,
                energy_regen: _,
                can_hold: _,
                blocked_attacks: _,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
            },
            Roll {
                ref mut energy_cost,
                ref mut buildup_duration,
                ref mut movement_duration,
                ref mut recover_duration,
                roll_strength: _,
                attack_immunities: _,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *movement_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
            },
            ComboMelee {
                ref mut stage_data,
                initial_energy_gain: _,
                max_energy_gain: _,
                energy_increase: _,
                speed_increase: _,
                max_speed_increase: _,
                scales_from_combo: _,
                ori_modifier: _,
                meta: _,
            } => {
                *stage_data = stage_data
                    .iter_mut()
                    .map(|s| s.adjusted_by_stats(stats))
                    .collect();
            },
            ComboMelee2 {
                ref mut strikes,
                ref mut energy_cost_per_strike,
                auto_progress: _,
                meta: _,
            } => {
                *energy_cost_per_strike /= stats.energy_efficiency;
                *strikes = strikes
                    .iter_mut()
                    .map(|s| s.adjusted_by_stats(stats))
                    .collect();
            },
            LeapMelee {
                ref mut energy_cost,
                ref mut buildup_duration,
                movement_duration: _,
                ref mut swing_duration,
                ref mut recover_duration,
                ref mut melee_constructor,
                forward_leap_strength: _,
                vertical_leap_strength: _,
                ref mut damage_effect,
                specifier: _,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *swing_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
                *melee_constructor = melee_constructor.adjusted_by_stats(stats);
                if let Some(CombatEffect::Buff(combat::CombatBuff {
                    kind: _,
                    dur_secs: _,
                    strength,
                    chance: _,
                })) = damage_effect
                {
                    *strength *= stats.buff_strength;
                }
            },
            LeapShockwave {
                ref mut energy_cost,
                ref mut buildup_duration,
                movement_duration: _,
                ref mut swing_duration,
                ref mut recover_duration,
                ref mut damage,
                ref mut poise_damage,
                knockback: _,
                shockwave_angle: _,
                shockwave_vertical_angle: _,
                shockwave_speed: _,
                ref mut shockwave_duration,
                dodgeable: _,
                move_efficiency: _,
                damage_kind: _,
                specifier: _,
                ref mut damage_effect,
                forward_leap_strength: _,
                vertical_leap_strength: _,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *swing_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *damage *= stats.power;
                *poise_damage *= stats.effect_power;
                *shockwave_duration *= stats.range;
                *energy_cost /= stats.energy_efficiency;
                if let Some(CombatEffect::Buff(combat::CombatBuff {
                    kind: _,
                    dur_secs: _,
                    strength,
                    chance: _,
                })) = damage_effect
                {
                    *strength *= stats.buff_strength;
                }
            },
            ChargedMelee {
                ref mut energy_cost,
                ref mut energy_drain,
                ref mut buildup_strike,
                ref mut charge_duration,
                ref mut swing_duration,
                hit_timing: _,
                ref mut recover_duration,
                ref mut melee_constructor,
                specifier: _,
                ref mut damage_effect,
                meta: _,
                additional_combo: _,
            } => {
                *swing_duration /= stats.speed;
                *buildup_strike = buildup_strike
                    .map(|(dur, strike)| (dur / stats.speed, strike.adjusted_by_stats(stats)));
                *charge_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
                *energy_drain *= stats.speed / stats.energy_efficiency;
                *melee_constructor = melee_constructor.adjusted_by_stats(stats);
                if let Some(CombatEffect::Buff(combat::CombatBuff {
                    kind: _,
                    dur_secs: _,
                    strength,
                    chance: _,
                })) = damage_effect
                {
                    *strength *= stats.buff_strength;
                }
            },
            ChargedRanged {
                ref mut energy_cost,
                ref mut energy_drain,
                initial_regen: _,
                scaled_regen: _,
                ref mut initial_damage,
                ref mut scaled_damage,
                initial_knockback: _,
                scaled_knockback: _,
                ref mut buildup_duration,
                ref mut charge_duration,
                ref mut recover_duration,
                projectile_body: _,
                projectile_light: _,
                ref mut initial_projectile_speed,
                ref mut scaled_projectile_speed,
                damage_effect: _,
                move_speed: _,
                meta: _,
            } => {
                *initial_damage *= stats.power;
                *scaled_damage *= stats.power;
                *buildup_duration /= stats.speed;
                *charge_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *initial_projectile_speed *= stats.range;
                *scaled_projectile_speed *= stats.range;
                *energy_cost /= stats.energy_efficiency;
                *energy_drain *= stats.speed / stats.energy_efficiency;
            },
            Shockwave {
                ref mut energy_cost,
                ref mut buildup_duration,
                ref mut swing_duration,
                ref mut recover_duration,
                ref mut damage,
                ref mut poise_damage,
                knockback: _,
                shockwave_angle: _,
                shockwave_vertical_angle: _,
                shockwave_speed: _,
                ref mut shockwave_duration,
                dodgeable: _,
                move_efficiency: _,
                damage_kind: _,
                specifier: _,
                ori_rate: _,
                ref mut damage_effect,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *swing_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *damage *= stats.power;
                *poise_damage *= stats.effect_power;
                *shockwave_duration *= stats.range;
                *energy_cost /= stats.energy_efficiency;
                *damage_effect = damage_effect.map(|de| de.adjusted_by_stats(stats));
            },
            BasicBeam {
                ref mut buildup_duration,
                ref mut recover_duration,
                ref mut beam_duration,
                ref mut damage,
                ref mut tick_rate,
                ref mut range,
                max_angle: _,
                ref mut damage_effect,
                energy_regen: _,
                ref mut energy_drain,
                ori_rate: _,
                specifier: _,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *damage *= stats.power;
                *tick_rate *= stats.speed;
                *range *= stats.range;
                // Duration modified to keep velocity constant
                *beam_duration *= stats.range as f64;
                *energy_drain /= stats.energy_efficiency;
                *damage_effect = damage_effect.map(|de| de.adjusted_by_stats(stats));
            },
            BasicAura {
                ref mut buildup_duration,
                ref mut cast_duration,
                ref mut recover_duration,
                targets: _,
                ref mut auras,
                aura_duration: _,
                ref mut range,
                ref mut energy_cost,
                scales_with_combo: _,
                specifier: _,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *cast_duration /= stats.speed;
                *recover_duration /= stats.speed;
                auras.iter_mut().for_each(
                    |aura::AuraBuffConstructor {
                         kind: _,
                         ref mut strength,
                         duration: _,
                         category: _,
                     }| {
                        *strength *= stats.diminished_buff_strength();
                    },
                );
                *range *= stats.range;
                *energy_cost /= stats.energy_efficiency;
            },
            Blink {
                ref mut buildup_duration,
                ref mut recover_duration,
                ref mut max_range,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *max_range *= stats.range;
            },
            BasicSummon {
                ref mut buildup_duration,
                ref mut cast_duration,
                ref mut recover_duration,
                summon_amount: _,
                summon_distance: (ref mut inner_dist, ref mut outer_dist),
                summon_info: _,
                duration: _,
                meta: _,
            } => {
                // TODO: Figure out how/if power should affect this
                *buildup_duration /= stats.speed;
                *cast_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *inner_dist *= stats.range;
                *outer_dist *= stats.range;
            },
            SelfBuff {
                ref mut buildup_duration,
                ref mut cast_duration,
                ref mut recover_duration,
                buff_kind: _,
                ref mut buff_strength,
                buff_duration: _,
                ref mut energy_cost,
                enforced_limit: _,
                combo_cost: _,
                combo_scaling: _,
                meta: _,
                specifier: _,
            } => {
                *buff_strength *= stats.diminished_buff_strength();
                *buildup_duration /= stats.speed;
                *cast_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
            },
            SpriteSummon {
                ref mut buildup_duration,
                ref mut cast_duration,
                ref mut recover_duration,
                sprite: _,
                del_timeout: _,
                summon_distance: (ref mut inner_dist, ref mut outer_dist),
                sparseness: _,
                angle: _,
                anchor: _,
                move_efficiency: _,
                meta: _,
            } => {
                // TODO: Figure out how/if power should affect this
                *buildup_duration /= stats.speed;
                *cast_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *inner_dist *= stats.range;
                *outer_dist *= stats.range;
            },
            Music {
                ref mut play_duration,
                ori_modifier: _,
                meta: _,
            } => {
                *play_duration /= stats.speed;
            },
            FinisherMelee {
                ref mut energy_cost,
                ref mut buildup_duration,
                ref mut swing_duration,
                ref mut recover_duration,
                ref mut melee_constructor,
                minimum_combo: _,
                scaling: _,
                combo_consumption: _,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *swing_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
                *melee_constructor = melee_constructor.adjusted_by_stats(stats);
            },
            DiveMelee {
                ref mut energy_cost,
                vertical_speed: _,
                movement_duration: _,
                ref mut buildup_duration,
                ref mut swing_duration,
                ref mut recover_duration,
                ref mut melee_constructor,
                max_scaling: _,
                meta: _,
            } => {
                *buildup_duration = buildup_duration.map(|b| b / stats.speed);
                *swing_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
                *melee_constructor = melee_constructor.adjusted_by_stats(stats);
            },
            RiposteMelee {
                ref mut energy_cost,
                ref mut buildup_duration,
                ref mut swing_duration,
                ref mut recover_duration,
                ref mut melee_constructor,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *swing_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
                *melee_constructor = melee_constructor.adjusted_by_stats(stats);
            },
            RapidMelee {
                ref mut buildup_duration,
                ref mut swing_duration,
                ref mut recover_duration,
                ref mut energy_cost,
                ref mut melee_constructor,
                max_strikes: _,
                move_modifier: _,
                ori_modifier: _,
                minimum_combo: _,
                frontend_specifier: _,
                meta: _,
            } => {
                *buildup_duration /= stats.speed;
                *swing_duration /= stats.speed;
                *recover_duration /= stats.speed;
                *energy_cost /= stats.energy_efficiency;
                *melee_constructor = melee_constructor.adjusted_by_stats(stats);
            },
        }
        self
    }

    pub fn energy_cost(&self) -> f32 {
        use CharacterAbility::*;
        match self {
            BasicMelee { energy_cost, .. }
            | BasicRanged { energy_cost, .. }
            | RepeaterRanged { energy_cost, .. }
            | DashMelee { energy_cost, .. }
            | Roll { energy_cost, .. }
            | LeapMelee { energy_cost, .. }
            | LeapShockwave { energy_cost, .. }
            | ChargedMelee { energy_cost, .. }
            | ChargedRanged { energy_cost, .. }
            | Shockwave { energy_cost, .. }
            | BasicAura { energy_cost, .. }
            | BasicBlock { energy_cost, .. }
            | SelfBuff { energy_cost, .. }
            | FinisherMelee { energy_cost, .. }
            | ComboMelee2 {
                energy_cost_per_strike: energy_cost,
                ..
            }
            | DiveMelee { energy_cost, .. }
            | RiposteMelee { energy_cost, .. }
            | RapidMelee { energy_cost, .. } => *energy_cost,
            BasicBeam { energy_drain, .. } => {
                if *energy_drain > f32::EPSILON {
                    1.0
                } else {
                    0.0
                }
            },
            Boost { .. }
            | ComboMelee { .. }
            | Blink { .. }
            | Music { .. }
            | BasicSummon { .. }
            | SpriteSummon { .. } => 0.0,
        }
    }

    #[allow(clippy::bool_to_int_with_if)]
    pub fn combo_cost(&self) -> u32 {
        use CharacterAbility::*;
        match self {
            BasicAura {
                scales_with_combo, ..
            } => {
                if *scales_with_combo {
                    1
                } else {
                    0
                }
            },
            FinisherMelee {
                minimum_combo: combo,
                ..
            }
            | RapidMelee {
                minimum_combo: combo,
                ..
            }
            | SelfBuff {
                combo_cost: combo, ..
            } => *combo,
            BasicMelee { .. }
            | BasicRanged { .. }
            | RepeaterRanged { .. }
            | DashMelee { .. }
            | Roll { .. }
            | LeapMelee { .. }
            | LeapShockwave { .. }
            | ChargedMelee { .. }
            | ChargedRanged { .. }
            | Shockwave { .. }
            | BasicBlock { .. }
            | ComboMelee2 { .. }
            | DiveMelee { .. }
            | RiposteMelee { .. }
            | BasicBeam { .. }
            | Boost { .. }
            | ComboMelee { .. }
            | Blink { .. }
            | Music { .. }
            | BasicSummon { .. }
            | SpriteSummon { .. } => 0,
        }
    }

    // TODO: Maybe consider making CharacterAbility a struct at some point?
    pub fn ability_meta(&self) -> AbilityMeta {
        use CharacterAbility::*;
        match self {
            BasicMelee { meta, .. }
            | BasicRanged { meta, .. }
            | RepeaterRanged { meta, .. }
            | DashMelee { meta, .. }
            | Roll { meta, .. }
            | LeapMelee { meta, .. }
            | LeapShockwave { meta, .. }
            | ChargedMelee { meta, .. }
            | ChargedRanged { meta, .. }
            | Shockwave { meta, .. }
            | BasicAura { meta, .. }
            | BasicBlock { meta, .. }
            | SelfBuff { meta, .. }
            | BasicBeam { meta, .. }
            | Boost { meta, .. }
            | ComboMelee { meta, .. }
            | ComboMelee2 { meta, .. }
            | Blink { meta, .. }
            | BasicSummon { meta, .. }
            | SpriteSummon { meta, .. }
            | FinisherMelee { meta, .. }
            | Music { meta, .. }
            | DiveMelee { meta, .. }
            | RiposteMelee { meta, .. }
            | RapidMelee { meta, .. } => *meta,
        }
    }

    #[must_use = "method returns new ability and doesn't mutate the original value"]
    pub fn adjusted_by_skills(mut self, skillset: &SkillSet, tool: Option<ToolKind>) -> Self {
        match tool {
            Some(ToolKind::Hammer) => self.adjusted_by_hammer_skills(skillset),
            Some(ToolKind::Bow) => self.adjusted_by_bow_skills(skillset),
            Some(ToolKind::Staff) => self.adjusted_by_staff_skills(skillset),
            Some(ToolKind::Sceptre) => self.adjusted_by_sceptre_skills(skillset),
            Some(ToolKind::Pick) => self.adjusted_by_mining_skills(skillset),
            None => self.adjusted_by_general_skills(skillset),
            Some(_) => {},
        }
        self
    }

    fn adjusted_by_mining_skills(&mut self, skillset: &SkillSet) {
        use skills::MiningSkill::Speed;

        if let CharacterAbility::BasicMelee {
            ref mut buildup_duration,
            ref mut swing_duration,
            ref mut recover_duration,
            ..
        } = self
        {
            if let Ok(level) = skillset.skill_level(Skill::Pick(Speed)) {
                let modifiers = SKILL_MODIFIERS.mining_tree;

                let speed = modifiers.speed.powi(level.into());
                *buildup_duration /= speed;
                *swing_duration /= speed;
                *recover_duration /= speed;
            }
        }
    }

    fn adjusted_by_general_skills(&mut self, skillset: &SkillSet) {
        if let CharacterAbility::Roll {
            ref mut energy_cost,
            ref mut roll_strength,
            ref mut movement_duration,
            ..
        } = self
        {
            use skills::RollSkill::{Cost, Duration, Strength};

            let modifiers = SKILL_MODIFIERS.general_tree.roll;

            if let Ok(level) = skillset.skill_level(Skill::Roll(Cost)) {
                *energy_cost *= modifiers.energy_cost.powi(level.into());
            }
            if let Ok(level) = skillset.skill_level(Skill::Roll(Strength)) {
                *roll_strength *= modifiers.strength.powi(level.into());
            }
            if let Ok(level) = skillset.skill_level(Skill::Roll(Duration)) {
                *movement_duration *= modifiers.duration.powi(level.into());
            }
        }
    }

    fn adjusted_by_hammer_skills(&mut self, skillset: &SkillSet) {
        #![allow(clippy::enum_glob_use)]
        use skills::{HammerSkill::*, Skill::Hammer};

        match self {
            CharacterAbility::ComboMelee {
                ref mut speed_increase,
                ref mut max_speed_increase,
                ref mut stage_data,
                ref mut max_energy_gain,
                ref mut scales_from_combo,
                ..
            } => {
                let modifiers = SKILL_MODIFIERS.hammer_tree.single_strike;

                if let Ok(level) = skillset.skill_level(Hammer(SsKnockback)) {
                    *stage_data = (*stage_data)
                        .iter()
                        .map(|s| s.modify_strike(modifiers.knockback.powi(level.into())))
                        .collect::<Vec<_>>();
                }
                let speed_segments = f32::from(Hammer(SsSpeed).max_level());
                let speed_level = f32::from(skillset.skill_level(Hammer(SsSpeed)).unwrap_or(0));
                *speed_increase *= speed_level / speed_segments;
                *max_speed_increase *= speed_level / speed_segments;

                let energy_level = skillset.skill_level(Hammer(SsRegen)).unwrap_or(0);

                let stages = u16::try_from(stage_data.len())
                    .expect("number of stages can't be more than u16");

                *max_energy_gain *= f32::from((energy_level + 1) * stages)
                    / f32::from((Hammer(SsRegen).max_level() + 1) * stages);

                *scales_from_combo = skillset.skill_level(Hammer(SsDamage)).unwrap_or(0).into();
            },
            CharacterAbility::ChargedMelee {
                ref mut energy_drain,
                ref mut charge_duration,
                ref mut melee_constructor,
                ..
            } => {
                let modifiers = SKILL_MODIFIERS.hammer_tree.charged;

                if let Some(MeleeConstructorKind::Bash {
                    ref mut damage,
                    ref mut knockback,
                    ..
                }) = melee_constructor.scaled
                {
                    if let Ok(level) = skillset.skill_level(Hammer(CDamage)) {
                        *damage *= modifiers.scaled_damage.powi(level.into());
                    }
                    if let Ok(level) = skillset.skill_level(Hammer(CKnockback)) {
                        *knockback *= modifiers.scaled_knockback.powi(level.into());
                    }
                }
                if let Ok(level) = skillset.skill_level(Hammer(CDrain)) {
                    *energy_drain *= modifiers.energy_drain.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Hammer(CSpeed)) {
                    let charge_time = 1.0 / modifiers.charge_rate;
                    *charge_duration *= charge_time.powi(level.into());
                }
            },
            CharacterAbility::LeapMelee {
                ref mut energy_cost,
                ref mut forward_leap_strength,
                ref mut vertical_leap_strength,
                ref mut melee_constructor,
                ..
            } => {
                let modifiers = SKILL_MODIFIERS.hammer_tree.leap;
                if let MeleeConstructorKind::Bash {
                    ref mut damage,
                    ref mut knockback,
                    ..
                } = melee_constructor.kind
                {
                    if let Ok(level) = skillset.skill_level(Hammer(LDamage)) {
                        *damage *= modifiers.base_damage.powi(level.into());
                    }
                    if let Ok(level) = skillset.skill_level(Hammer(LKnockback)) {
                        *knockback *= modifiers.knockback.powi(level.into());
                    }
                }
                if let Ok(level) = skillset.skill_level(Hammer(LCost)) {
                    *energy_cost *= modifiers.energy_cost.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Hammer(LDistance)) {
                    let strength = modifiers.leap_strength;
                    *forward_leap_strength *= strength.powi(level.into());
                    *vertical_leap_strength *= strength.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Hammer(LRange)) {
                    melee_constructor.range += modifiers.range * f32::from(level);
                }
            },
            _ => {},
        }
    }

    fn adjusted_by_bow_skills(&mut self, skillset: &SkillSet) {
        #![allow(clippy::enum_glob_use)]
        use skills::{BowSkill::*, Skill::Bow};

        let projectile_speed_modifier = SKILL_MODIFIERS.bow_tree.universal.projectile_speed;
        match self {
            CharacterAbility::ChargedRanged {
                ref mut initial_damage,
                ref mut scaled_damage,
                ref mut initial_regen,
                ref mut scaled_regen,
                ref mut initial_knockback,
                ref mut scaled_knockback,
                ref mut move_speed,
                ref mut initial_projectile_speed,
                ref mut scaled_projectile_speed,
                ref mut charge_duration,
                ..
            } => {
                let modifiers = SKILL_MODIFIERS.bow_tree.charged;
                if let Ok(level) = skillset.skill_level(Bow(ProjSpeed)) {
                    let projectile_speed_scaling = projectile_speed_modifier.powi(level.into());
                    *initial_projectile_speed *= projectile_speed_scaling;
                    *scaled_projectile_speed *= projectile_speed_scaling;
                }
                if let Ok(level) = skillset.skill_level(Bow(CDamage)) {
                    let damage_scaling = modifiers.damage_scaling.powi(level.into());
                    *initial_damage *= damage_scaling;
                    *scaled_damage *= damage_scaling;
                }
                if let Ok(level) = skillset.skill_level(Bow(CRegen)) {
                    let regen_scaling = modifiers.regen_scaling.powi(level.into());
                    *initial_regen *= regen_scaling;
                    *scaled_regen *= regen_scaling;
                }
                if let Ok(level) = skillset.skill_level(Bow(CKnockback)) {
                    let knockback_scaling = modifiers.knockback_scaling.powi(level.into());
                    *initial_knockback *= knockback_scaling;
                    *scaled_knockback *= knockback_scaling;
                }
                if let Ok(level) = skillset.skill_level(Bow(CSpeed)) {
                    let charge_time = 1.0 / modifiers.charge_rate;
                    *charge_duration *= charge_time.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Bow(CMove)) {
                    *move_speed *= modifiers.move_speed.powi(level.into());
                }
            },
            CharacterAbility::RepeaterRanged {
                ref mut energy_cost,
                ref mut projectile,
                ref mut max_speed,
                ref mut projectile_speed,
                ..
            } => {
                let modifiers = SKILL_MODIFIERS.bow_tree.repeater;
                if let Ok(level) = skillset.skill_level(Bow(ProjSpeed)) {
                    *projectile_speed *= projectile_speed_modifier.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Bow(RDamage)) {
                    let power = modifiers.power.powi(level.into());
                    *projectile = projectile.modified_projectile(power, 1_f32, 1_f32);
                }
                if let Ok(level) = skillset.skill_level(Bow(RCost)) {
                    *energy_cost *= modifiers.energy_cost.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Bow(RSpeed)) {
                    *max_speed *= modifiers.max_speed.powi(level.into());
                }
            },
            CharacterAbility::BasicRanged {
                ref mut projectile,
                ref mut energy_cost,
                ref mut num_projectiles,
                ref mut projectile_spread,
                ref mut projectile_speed,
                ..
            } => {
                let modifiers = SKILL_MODIFIERS.bow_tree.shotgun;
                if let Ok(level) = skillset.skill_level(Bow(ProjSpeed)) {
                    *projectile_speed *= projectile_speed_modifier.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Bow(SDamage)) {
                    let power = modifiers.power.powi(level.into());
                    *projectile = projectile.modified_projectile(power, 1_f32, 1_f32);
                }
                if let Ok(level) = skillset.skill_level(Bow(SCost)) {
                    *energy_cost *= modifiers.energy_cost.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Bow(SArrows)) {
                    *num_projectiles += u32::from(level) * modifiers.num_projectiles;
                }
                if let Ok(level) = skillset.skill_level(Bow(SSpread)) {
                    *projectile_spread *= modifiers.spread.powi(level.into());
                }
            },
            _ => {},
        }
    }

    fn adjusted_by_staff_skills(&mut self, skillset: &SkillSet) {
        #![allow(clippy::enum_glob_use)]
        use skills::{Skill::Staff, StaffSkill::*};

        match self {
            CharacterAbility::BasicRanged {
                ref mut projectile, ..
            } => {
                let modifiers = SKILL_MODIFIERS.staff_tree.fireball;
                let damage_level = skillset.skill_level(Staff(BDamage)).unwrap_or(0);
                let regen_level = skillset.skill_level(Staff(BRegen)).unwrap_or(0);
                let range_level = skillset.skill_level(Staff(BRadius)).unwrap_or(0);
                let power = modifiers.power.powi(damage_level.into());
                let regen = modifiers.regen.powi(regen_level.into());
                let range = modifiers.range.powi(range_level.into());
                *projectile = projectile.modified_projectile(power, regen, range);
            },
            CharacterAbility::BasicBeam {
                ref mut damage,
                ref mut range,
                ref mut energy_drain,
                ref mut beam_duration,
                ..
            } => {
                let modifiers = SKILL_MODIFIERS.staff_tree.flamethrower;
                if let Ok(level) = skillset.skill_level(Staff(FDamage)) {
                    *damage *= modifiers.damage.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Staff(FRange)) {
                    let range_mod = modifiers.range.powi(level.into());
                    *range *= range_mod;
                    // Duration modified to keep velocity constant
                    *beam_duration *= range_mod as f64;
                }
                if let Ok(level) = skillset.skill_level(Staff(FDrain)) {
                    *energy_drain *= modifiers.energy_drain.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Staff(FVelocity)) {
                    let velocity_increase = modifiers.velocity.powi(level.into());
                    let duration_mod = 1.0 / (1.0 + velocity_increase);
                    *beam_duration *= duration_mod as f64;
                }
            },
            CharacterAbility::Shockwave {
                ref mut damage,
                ref mut knockback,
                ref mut shockwave_duration,
                ref mut energy_cost,
                ..
            } => {
                let modifiers = SKILL_MODIFIERS.staff_tree.shockwave;
                if let Ok(level) = skillset.skill_level(Staff(SDamage)) {
                    *damage *= modifiers.damage.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Staff(SKnockback)) {
                    let knockback_mod = modifiers.knockback.powi(level.into());
                    *knockback = knockback.modify_strength(knockback_mod);
                }
                if let Ok(level) = skillset.skill_level(Staff(SRange)) {
                    *shockwave_duration *= modifiers.duration.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Staff(SCost)) {
                    *energy_cost *= modifiers.energy_cost.powi(level.into());
                }
            },
            _ => {},
        }
    }

    fn adjusted_by_sceptre_skills(&mut self, skillset: &SkillSet) {
        #![allow(clippy::enum_glob_use)]
        use skills::{SceptreSkill::*, Skill::Sceptre};

        match self {
            CharacterAbility::BasicBeam {
                ref mut damage,
                ref mut range,
                ref mut beam_duration,
                ref mut damage_effect,
                ref mut energy_regen,
                ..
            } => {
                let modifiers = SKILL_MODIFIERS.sceptre_tree.beam;
                if let Ok(level) = skillset.skill_level(Sceptre(LDamage)) {
                    *damage *= modifiers.damage.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Sceptre(LRange)) {
                    let range_mod = modifiers.range.powi(level.into());
                    *range *= range_mod;
                    // Duration modified to keep velocity constant
                    *beam_duration *= range_mod as f64;
                }
                if let Ok(level) = skillset.skill_level(Sceptre(LRegen)) {
                    *energy_regen *= modifiers.energy_regen.powi(level.into());
                }
                if let (Ok(level), Some(CombatEffect::Lifesteal(ref mut lifesteal))) =
                    (skillset.skill_level(Sceptre(LLifesteal)), damage_effect)
                {
                    *lifesteal *= modifiers.lifesteal.powi(level.into());
                }
            },
            CharacterAbility::BasicAura {
                ref mut auras,
                ref mut range,
                ref mut energy_cost,
                specifier: Some(aura::Specifier::HealingAura),
                ..
            } => {
                let modifiers = SKILL_MODIFIERS.sceptre_tree.healing_aura;
                if let Ok(level) = skillset.skill_level(Sceptre(HHeal)) {
                    auras.iter_mut().for_each(|ref mut aura| {
                        aura.strength *= modifiers.strength.powi(level.into());
                    });
                }
                if let Ok(level) = skillset.skill_level(Sceptre(HDuration)) {
                    auras.iter_mut().for_each(|ref mut aura| {
                        if let Some(ref mut duration) = aura.duration {
                            *duration *= modifiers.duration.powi(level.into()) as f64;
                        }
                    });
                }
                if let Ok(level) = skillset.skill_level(Sceptre(HRange)) {
                    *range *= modifiers.range.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Sceptre(HCost)) {
                    *energy_cost *= modifiers.energy_cost.powi(level.into());
                }
            },
            CharacterAbility::BasicAura {
                ref mut auras,
                ref mut range,
                ref mut energy_cost,
                specifier: Some(aura::Specifier::WardingAura),
                ..
            } => {
                let modifiers = SKILL_MODIFIERS.sceptre_tree.warding_aura;
                if let Ok(level) = skillset.skill_level(Sceptre(AStrength)) {
                    auras.iter_mut().for_each(|ref mut aura| {
                        aura.strength *= modifiers.strength.powi(level.into());
                    });
                }
                if let Ok(level) = skillset.skill_level(Sceptre(ADuration)) {
                    auras.iter_mut().for_each(|ref mut aura| {
                        if let Some(ref mut duration) = aura.duration {
                            *duration *= modifiers.duration.powi(level.into()) as f64;
                        }
                    });
                }
                if let Ok(level) = skillset.skill_level(Sceptre(ARange)) {
                    *range *= modifiers.range.powi(level.into());
                }
                if let Ok(level) = skillset.skill_level(Sceptre(ACost)) {
                    *energy_cost *= modifiers.energy_cost.powi(level.into());
                }
            },
            _ => {},
        }
    }
}

/// Small helper for #[serde(default)] booleans
fn default_true() -> bool { true }

impl From<(&CharacterAbility, AbilityInfo, &JoinData<'_>)> for CharacterState {
    fn from((ability, ability_info, data): (&CharacterAbility, AbilityInfo, &JoinData)) -> Self {
        match ability {
            CharacterAbility::BasicMelee {
                buildup_duration,
                swing_duration,
                recover_duration,
                melee_constructor,
                ori_modifier,
                energy_cost: _,
                meta: _,
            } => CharacterState::BasicMelee(basic_melee::Data {
                static_data: basic_melee::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    swing_duration: Duration::from_secs_f32(*swing_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    melee_constructor: *melee_constructor,
                    ori_modifier: *ori_modifier,
                    ability_info,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                exhausted: false,
            }),
            CharacterAbility::BasicRanged {
                buildup_duration,
                recover_duration,
                projectile,
                projectile_body,
                projectile_light,
                projectile_speed,
                energy_cost: _,
                num_projectiles,
                projectile_spread,
                damage_effect,
                move_efficiency,
                meta: _,
            } => CharacterState::BasicRanged(basic_ranged::Data {
                static_data: basic_ranged::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    projectile: *projectile,
                    projectile_body: *projectile_body,
                    projectile_light: *projectile_light,
                    projectile_speed: *projectile_speed,
                    num_projectiles: *num_projectiles,
                    projectile_spread: *projectile_spread,
                    ability_info,
                    damage_effect: *damage_effect,
                    move_efficiency: *move_efficiency,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                exhausted: false,
            }),
            CharacterAbility::Boost {
                movement_duration,
                only_up,
                speed,
                max_exit_velocity,
                meta: _,
            } => CharacterState::Boost(boost::Data {
                static_data: boost::StaticData {
                    movement_duration: Duration::from_secs_f32(*movement_duration),
                    only_up: *only_up,
                    speed: *speed,
                    max_exit_velocity: *max_exit_velocity,
                    ability_info,
                },
                timer: Duration::default(),
            }),
            CharacterAbility::DashMelee {
                energy_cost: _,
                energy_drain,
                forward_speed,
                buildup_duration,
                charge_duration,
                swing_duration,
                recover_duration,
                melee_constructor,
                ori_modifier,
                charge_through,
                meta: _,
            } => CharacterState::DashMelee(dash_melee::Data {
                static_data: dash_melee::StaticData {
                    energy_drain: *energy_drain,
                    forward_speed: *forward_speed,
                    charge_through: *charge_through,
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    charge_duration: Duration::from_secs_f32(*charge_duration),
                    swing_duration: Duration::from_secs_f32(*swing_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    melee_constructor: *melee_constructor,
                    ori_modifier: *ori_modifier,
                    ability_info,
                },
                auto_charge: false,
                timer: Duration::default(),
                charge_end_timer: Duration::from_secs_f32(*charge_duration),
                stage_section: StageSection::Buildup,
                exhausted: false,
            }),
            CharacterAbility::BasicBlock {
                buildup_duration,
                recover_duration,
                max_angle,
                block_strength,
                parry_window,
                energy_cost,
                energy_regen,
                can_hold,
                blocked_attacks,
                meta: _,
            } => CharacterState::BasicBlock(basic_block::Data {
                static_data: basic_block::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    max_angle: *max_angle,
                    block_strength: *block_strength,
                    parry_window: *parry_window,
                    energy_cost: *energy_cost,
                    energy_regen: *energy_regen,
                    can_hold: *can_hold,
                    blocked_attacks: *blocked_attacks,
                    ability_info,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
            }),
            CharacterAbility::Roll {
                energy_cost: _,
                buildup_duration,
                movement_duration,
                recover_duration,
                roll_strength,
                attack_immunities,
                meta: _,
            } => CharacterState::Roll(roll::Data {
                static_data: roll::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    movement_duration: Duration::from_secs_f32(*movement_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    roll_strength: *roll_strength,
                    attack_immunities: *attack_immunities,
                    ability_info,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                was_wielded: false, // false by default. utils might set it to true
                prev_aimed_dir: None,
                is_sneaking: false,
                was_combo: None,
            }),
            CharacterAbility::ComboMelee {
                stage_data,
                initial_energy_gain,
                max_energy_gain,
                energy_increase,
                speed_increase,
                max_speed_increase,
                scales_from_combo,
                ori_modifier,
                meta: _,
            } => CharacterState::ComboMelee(combo_melee::Data {
                static_data: combo_melee::StaticData {
                    num_stages: stage_data.len() as u32,
                    stage_data: stage_data.iter().map(|stage| stage.to_duration()).collect(),
                    initial_energy_gain: *initial_energy_gain,
                    max_energy_gain: *max_energy_gain,
                    energy_increase: *energy_increase,
                    speed_increase: 1.0 - *speed_increase,
                    max_speed_increase: *max_speed_increase,
                    scales_from_combo: *scales_from_combo,
                    ori_modifier: *ori_modifier,
                    ability_info,
                },
                exhausted: false,
                stage: 1,
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
            }),
            CharacterAbility::ComboMelee2 {
                strikes,
                energy_cost_per_strike,
                auto_progress,
                meta: _,
            } => CharacterState::ComboMelee2(combo_melee2::Data {
                static_data: combo_melee2::StaticData {
                    strikes: strikes.iter().map(|s| s.to_duration()).collect(),
                    energy_cost_per_strike: *energy_cost_per_strike,
                    auto_progress: *auto_progress,
                    ability_info,
                },
                exhausted: false,
                start_next_strike: false,
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                completed_strikes: 0,
            }),
            CharacterAbility::LeapMelee {
                energy_cost: _,
                buildup_duration,
                movement_duration,
                swing_duration,
                recover_duration,
                melee_constructor,
                forward_leap_strength,
                vertical_leap_strength,
                damage_effect,
                specifier,
                meta: _,
            } => CharacterState::LeapMelee(leap_melee::Data {
                static_data: leap_melee::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    movement_duration: Duration::from_secs_f32(*movement_duration),
                    swing_duration: Duration::from_secs_f32(*swing_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    melee_constructor: *melee_constructor,
                    forward_leap_strength: *forward_leap_strength,
                    vertical_leap_strength: *vertical_leap_strength,
                    ability_info,
                    damage_effect: *damage_effect,
                    specifier: *specifier,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                exhausted: false,
            }),
            CharacterAbility::LeapShockwave {
                energy_cost: _,
                buildup_duration,
                movement_duration,
                swing_duration,
                recover_duration,
                damage,
                poise_damage,
                knockback,
                shockwave_angle,
                shockwave_vertical_angle,
                shockwave_speed,
                shockwave_duration,
                dodgeable,
                move_efficiency,
                damage_kind,
                specifier,
                damage_effect,
                forward_leap_strength,
                vertical_leap_strength,
                meta: _,
            } => CharacterState::LeapShockwave(leap_shockwave::Data {
                static_data: leap_shockwave::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    movement_duration: Duration::from_secs_f32(*movement_duration),
                    swing_duration: Duration::from_secs_f32(*swing_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    damage: *damage,
                    poise_damage: *poise_damage,
                    knockback: *knockback,
                    shockwave_angle: *shockwave_angle,
                    shockwave_vertical_angle: *shockwave_vertical_angle,
                    shockwave_speed: *shockwave_speed,
                    shockwave_duration: Duration::from_secs_f32(*shockwave_duration),
                    dodgeable: *dodgeable,
                    move_efficiency: *move_efficiency,
                    damage_kind: *damage_kind,
                    specifier: *specifier,
                    damage_effect: *damage_effect,
                    forward_leap_strength: *forward_leap_strength,
                    vertical_leap_strength: *vertical_leap_strength,
                    ability_info,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                exhausted: false,
            }),
            CharacterAbility::ChargedMelee {
                energy_cost,
                energy_drain,
                buildup_strike,
                charge_duration,
                swing_duration,
                hit_timing,
                recover_duration,
                melee_constructor,
                specifier,
                damage_effect,
                additional_combo,
                meta: _,
            } => CharacterState::ChargedMelee(charged_melee::Data {
                static_data: charged_melee::StaticData {
                    energy_cost: *energy_cost,
                    energy_drain: *energy_drain,
                    buildup_strike: buildup_strike
                        .map(|(dur, strike)| (Duration::from_secs_f32(dur), strike)),
                    charge_duration: Duration::from_secs_f32(*charge_duration),
                    swing_duration: Duration::from_secs_f32(*swing_duration),
                    hit_timing: *hit_timing,
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    melee_constructor: *melee_constructor,
                    ability_info,
                    specifier: *specifier,
                    damage_effect: *damage_effect,
                    additional_combo: *additional_combo,
                },
                stage_section: if buildup_strike.is_some() {
                    StageSection::Buildup
                } else {
                    StageSection::Charge
                },
                timer: Duration::default(),
                exhausted: false,
                charge_amount: 0.0,
            }),
            CharacterAbility::ChargedRanged {
                energy_cost: _,
                energy_drain,
                initial_regen,
                scaled_regen,
                initial_damage,
                scaled_damage,
                initial_knockback,
                scaled_knockback,
                buildup_duration,
                charge_duration,
                recover_duration,
                projectile_body,
                projectile_light,
                initial_projectile_speed,
                scaled_projectile_speed,
                damage_effect,
                move_speed,
                meta: _,
            } => CharacterState::ChargedRanged(charged_ranged::Data {
                static_data: charged_ranged::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    charge_duration: Duration::from_secs_f32(*charge_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    energy_drain: *energy_drain,
                    initial_regen: *initial_regen,
                    scaled_regen: *scaled_regen,
                    initial_damage: *initial_damage,
                    scaled_damage: *scaled_damage,
                    initial_knockback: *initial_knockback,
                    scaled_knockback: *scaled_knockback,
                    projectile_body: *projectile_body,
                    projectile_light: *projectile_light,
                    initial_projectile_speed: *initial_projectile_speed,
                    scaled_projectile_speed: *scaled_projectile_speed,
                    move_speed: *move_speed,
                    ability_info,
                    damage_effect: *damage_effect,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                exhausted: false,
            }),
            CharacterAbility::RepeaterRanged {
                energy_cost,
                buildup_duration,
                shoot_duration,
                recover_duration,
                max_speed,
                half_speed_at,
                projectile,
                projectile_body,
                projectile_light,
                projectile_speed,
                damage_effect,
                aoe,
                meta: _,
            } => CharacterState::RepeaterRanged(repeater_ranged::Data {
                static_data: repeater_ranged::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    shoot_duration: Duration::from_secs_f32(*shoot_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    energy_cost: *energy_cost,
                    // 1.0 is subtracted as 1.0 is added in state file
                    max_speed: *max_speed - 1.0,
                    half_speed_at: *half_speed_at,
                    projectile: *projectile,
                    projectile_body: *projectile_body,
                    projectile_light: *projectile_light,
                    projectile_speed: *projectile_speed,
                    ability_info,
                    damage_effect: *damage_effect,
                    aoe: *aoe,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                projectiles_fired: 0,
                speed: 1.0,
            }),
            CharacterAbility::Shockwave {
                energy_cost: _,
                buildup_duration,
                swing_duration,
                recover_duration,
                damage,
                poise_damage,
                knockback,
                shockwave_angle,
                shockwave_vertical_angle,
                shockwave_speed,
                shockwave_duration,
                dodgeable,
                move_efficiency,
                damage_kind,
                specifier,
                ori_rate,
                damage_effect,
                meta: _,
            } => CharacterState::Shockwave(shockwave::Data {
                static_data: shockwave::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    swing_duration: Duration::from_secs_f32(*swing_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    damage: *damage,
                    poise_damage: *poise_damage,
                    knockback: *knockback,
                    shockwave_angle: *shockwave_angle,
                    shockwave_vertical_angle: *shockwave_vertical_angle,
                    shockwave_speed: *shockwave_speed,
                    shockwave_duration: Duration::from_secs_f32(*shockwave_duration),
                    dodgeable: *dodgeable,
                    move_efficiency: *move_efficiency,
                    damage_effect: *damage_effect,
                    ability_info,
                    damage_kind: *damage_kind,
                    specifier: *specifier,
                    ori_rate: *ori_rate,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
            }),
            CharacterAbility::BasicBeam {
                buildup_duration,
                recover_duration,
                beam_duration,
                damage,
                tick_rate,
                range,
                max_angle,
                damage_effect,
                energy_regen,
                energy_drain,
                ori_rate,
                specifier,
                meta: _,
            } => CharacterState::BasicBeam(basic_beam::Data {
                static_data: basic_beam::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    beam_duration: Secs(*beam_duration),
                    damage: *damage,
                    tick_rate: *tick_rate,
                    range: *range,
                    end_radius: max_angle.to_radians().tan() * *range,
                    damage_effect: *damage_effect,
                    energy_regen: *energy_regen,
                    energy_drain: *energy_drain,
                    ability_info,
                    ori_rate: *ori_rate,
                    specifier: *specifier,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                aim_dir: data.ori.look_dir(),
                beam_offset: data.pos.0,
            }),
            CharacterAbility::BasicAura {
                buildup_duration,
                cast_duration,
                recover_duration,
                targets,
                auras,
                aura_duration,
                range,
                energy_cost: _,
                scales_with_combo,
                specifier,
                meta: _,
            } => CharacterState::BasicAura(basic_aura::Data {
                static_data: basic_aura::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    cast_duration: Duration::from_secs_f32(*cast_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    targets: *targets,
                    auras: auras.clone(),
                    aura_duration: *aura_duration,
                    range: *range,
                    ability_info,
                    scales_with_combo: *scales_with_combo,
                    combo_at_cast: data.combo.map_or(0, |c| c.counter()),
                    specifier: *specifier,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
            }),
            CharacterAbility::Blink {
                buildup_duration,
                recover_duration,
                max_range,
                meta: _,
            } => CharacterState::Blink(blink::Data {
                static_data: blink::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    max_range: *max_range,
                    ability_info,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
            }),
            CharacterAbility::BasicSummon {
                buildup_duration,
                cast_duration,
                recover_duration,
                summon_amount,
                summon_distance,
                summon_info,
                duration,
                meta: _,
            } => CharacterState::BasicSummon(basic_summon::Data {
                static_data: basic_summon::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    cast_duration: Duration::from_secs_f32(*cast_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    summon_amount: *summon_amount,
                    summon_distance: *summon_distance,
                    summon_info: *summon_info,
                    ability_info,
                    duration: *duration,
                },
                summon_count: 0,
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
            }),
            CharacterAbility::SelfBuff {
                buildup_duration,
                cast_duration,
                recover_duration,
                buff_kind,
                buff_strength,
                buff_duration,
                energy_cost: _,
                combo_cost,
                combo_scaling,
                enforced_limit,
                meta: _,
                specifier,
            } => CharacterState::SelfBuff(self_buff::Data {
                static_data: self_buff::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    cast_duration: Duration::from_secs_f32(*cast_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    buff_kind: *buff_kind,
                    buff_strength: *buff_strength,
                    buff_duration: *buff_duration,
                    combo_cost: *combo_cost,
                    combo_scaling: *combo_scaling,
                    combo_on_use: data.combo.map_or(0, |c| c.counter()),
                    enforced_limit: *enforced_limit,
                    ability_info,
                    specifier: *specifier,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
            }),
            CharacterAbility::SpriteSummon {
                buildup_duration,
                cast_duration,
                recover_duration,
                sprite,
                del_timeout,
                summon_distance,
                sparseness,
                angle,
                anchor,
                move_efficiency,
                meta: _,
            } => CharacterState::SpriteSummon(sprite_summon::Data {
                static_data: sprite_summon::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    cast_duration: Duration::from_secs_f32(*cast_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    sprite: *sprite,
                    del_timeout: *del_timeout,
                    summon_distance: *summon_distance,
                    sparseness: *sparseness,
                    angle: *angle,
                    anchor: *anchor,
                    move_efficiency: *move_efficiency,
                    ability_info,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                achieved_radius: summon_distance.0.floor() as i32 - 1,
            }),
            CharacterAbility::Music {
                play_duration,
                ori_modifier,
                meta: _,
            } => CharacterState::Music(music::Data {
                static_data: music::StaticData {
                    play_duration: Duration::from_secs_f32(*play_duration),
                    ori_modifier: *ori_modifier,
                    ability_info,
                },
                timer: Duration::default(),
                stage_section: StageSection::Action,
                exhausted: false,
            }),
            CharacterAbility::FinisherMelee {
                energy_cost: _,
                buildup_duration,
                swing_duration,
                recover_duration,
                melee_constructor,
                minimum_combo,
                scaling,
                combo_consumption,
                meta: _,
            } => CharacterState::FinisherMelee(finisher_melee::Data {
                static_data: finisher_melee::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    swing_duration: Duration::from_secs_f32(*swing_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    melee_constructor: *melee_constructor,
                    scaling: *scaling,
                    minimum_combo: *minimum_combo,
                    combo_on_use: data.combo.map_or(0, |c| c.counter()),
                    combo_consumption: *combo_consumption,
                    ability_info,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                exhausted: false,
            }),
            CharacterAbility::DiveMelee {
                buildup_duration,
                movement_duration,
                swing_duration,
                recover_duration,
                melee_constructor,
                energy_cost: _,
                vertical_speed,
                max_scaling,
                meta: _,
            } => CharacterState::DiveMelee(dive_melee::Data {
                static_data: dive_melee::StaticData {
                    buildup_duration: buildup_duration.map(Duration::from_secs_f32),
                    movement_duration: Duration::from_secs_f32(*movement_duration),
                    swing_duration: Duration::from_secs_f32(*swing_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    vertical_speed: *vertical_speed,
                    melee_constructor: *melee_constructor,
                    max_scaling: *max_scaling,
                    ability_info,
                },
                timer: Duration::default(),
                stage_section: if data.physics.on_ground.is_none() || buildup_duration.is_none() {
                    StageSection::Movement
                } else {
                    StageSection::Buildup
                },
                exhausted: false,
                max_vertical_speed: 0.0,
            }),
            CharacterAbility::RiposteMelee {
                energy_cost: _,
                buildup_duration,
                swing_duration,
                recover_duration,
                melee_constructor,
                meta: _,
            } => CharacterState::RiposteMelee(riposte_melee::Data {
                static_data: riposte_melee::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    swing_duration: Duration::from_secs_f32(*swing_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    melee_constructor: *melee_constructor,
                    ability_info,
                },
                timer: Duration::default(),
                stage_section: StageSection::Buildup,
                exhausted: false,
            }),
            CharacterAbility::RapidMelee {
                buildup_duration,
                swing_duration,
                recover_duration,
                melee_constructor,
                energy_cost,
                max_strikes,
                move_modifier,
                ori_modifier,
                minimum_combo,
                frontend_specifier,
                meta: _,
            } => CharacterState::RapidMelee(rapid_melee::Data {
                static_data: rapid_melee::StaticData {
                    buildup_duration: Duration::from_secs_f32(*buildup_duration),
                    swing_duration: Duration::from_secs_f32(*swing_duration),
                    recover_duration: Duration::from_secs_f32(*recover_duration),
                    melee_constructor: *melee_constructor,
                    energy_cost: *energy_cost,
                    max_strikes: *max_strikes,
                    move_modifier: *move_modifier,
                    ori_modifier: *ori_modifier,
                    minimum_combo: *minimum_combo,
                    frontend_specifier: *frontend_specifier,
                    ability_info,
                },
                timer: Duration::default(),
                current_strike: 1,
                stage_section: StageSection::Buildup,
                exhausted: false,
            }),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct AbilityMeta {
    #[serde(default)]
    pub capabilities: Capability,
    #[serde(default)]
    /// This is an event that gets emitted when the ability is first activated
    pub init_event: Option<AbilityInitEvent>,
    #[serde(default)]
    pub requirements: AbilityRequirements,
}

// TODO: Later move over things like energy and combo into here
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AbilityRequirements {
    pub stance: Option<Stance>,
}

impl AbilityRequirements {
    pub fn requirements_met(&self, stance: Option<&Stance>) -> bool {
        let AbilityRequirements { stance: req_stance } = self;
        req_stance.map_or(true, |req_stance| {
            stance.map_or(false, |char_stance| req_stance == *char_stance)
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash, PartialOrd, Ord)]
pub enum SwordStance {
    Crippling,
    Cleaving,
    Defensive,
    Heavy,
    Agile,
}

bitflags::bitflags! {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
    // If more are ever needed, first check if any not used anymore, as some were only used in intermediary stages so may be free
    pub struct Capability: u8 {
        // There used to be a capability here, to keep ordering the same below this is now a placeholder
        const PLACEHOLDER         = 0b00000001;
        // Allows blocking to interrupt the ability at any point
        const BLOCK_INTERRUPT     = 0b00000010;
        // When the ability is in the buildup section, it counts as a block with 50% DR
        const BLOCKS              = 0b00000100;
        // When in the ability, an entity only receives half as much poise damage
        const POISE_RESISTANT     = 0b00001000;
        // WHen in the ability, an entity only receives half as much knockback
        const KNOCKBACK_RESISTANT = 0b00010000;
        // The ability will parry melee attacks in the buildup portion
        const PARRIES             = 0b00100000;
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash, PartialOrd, Ord)]
pub enum Stance {
    None,
    Sword(SwordStance),
}

impl Stance {
    pub fn pseudo_ability_id(&self) -> &str {
        match self {
            Stance::Sword(SwordStance::Heavy) => "veloren.core.pseudo_abilities.sword.heavy_stance",
            Stance::Sword(SwordStance::Agile) => "veloren.core.pseudo_abilities.sword.agile_stance",
            Stance::Sword(SwordStance::Defensive) => {
                "veloren.core.pseudo_abilities.sword.defensive_stance"
            },
            Stance::Sword(SwordStance::Crippling) => {
                "veloren.core.pseudo_abilities.sword.crippling_stance"
            },
            Stance::Sword(SwordStance::Cleaving) => {
                "veloren.core.pseudo_abilities.sword.cleaving_stance"
            },
            Stance::None => "veloren.core.pseudo_abilities.no_stance",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AbilityInitEvent {
    EnterStance(Stance),
}

impl Default for Stance {
    fn default() -> Self { Self::None }
}

impl Component for Stance {
    type Storage = DerefFlaggedStorage<Self, specs::VecStorage<Self>>;
}
