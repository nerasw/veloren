use crate::{
    assets::{self, Asset, AssetExt},
    comp::{
        self,
        body::{humanoid, Body},
        item::tool::ToolKind,
    },
};
use hashbrown::{HashMap, HashSet};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use specs::{Component, DerefFlaggedStorage};
use specs_idvs::IdvStorage;
use std::hash::Hash;
use tracing::{trace, warn};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillTreeMap(HashMap<SkillGroupKind, HashSet<Skill>>);

impl Asset for SkillTreeMap {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

pub struct SkillGroupDef {
    pub skills: HashSet<Skill>,
    pub total_skill_point_cost: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillLevelMap(HashMap<Skill, Option<u16>>);

impl Asset for SkillLevelMap {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SkillPrerequisitesMap(HashMap<Skill, HashMap<Skill, Option<u16>>>);

impl Asset for SkillPrerequisitesMap {
    type Loader = assets::RonLoader;

    const EXTENSION: &'static str = "ron";
}

lazy_static! {
    // Determines the skills that comprise each skill group.
    //
    // This data is used to determine which of a player's skill groups a
    // particular skill should be added to when a skill unlock is requested.
    pub static ref SKILL_GROUP_DEFS: HashMap<SkillGroupKind, SkillGroupDef> = {
        let map = SkillTreeMap::load_expect_cloned(
            "common.skill_trees.skills_skill-groups_manifest",
        ).0;
        map.iter().map(|(sgk, skills)|
            (*sgk, SkillGroupDef { skills: skills.clone(),
                total_skill_point_cost: skills
                    .iter()
                    .map(|skill| {
                        if let Some(max_level) = skill.max_level() {
                            (1..=max_level)
                                .into_iter()
                                .map(|level| skill.skill_cost(Some(level)))
                                .sum()
                        } else {
                            skill.skill_cost(None)
                        }
                    })
                    .sum()
            })
        )
        .collect()
    };
    // Creates a hashmap for the reverse lookup of skill groups from a skill
    pub static ref SKILL_GROUP_LOOKUP: HashMap<Skill, SkillGroupKind> = {
        let map = SkillTreeMap::load_expect_cloned(
            "common.skill_trees.skills_skill-groups_manifest",
        ).0;
        map.iter().map(|(sgk, skills)| skills.into_iter().map(move |s| (*s, *sgk))).flatten().collect()
    };
    // Loads the maximum level that a skill can obtain
    pub static ref SKILL_MAX_LEVEL: HashMap<Skill, Option<u16>> = {
        SkillLevelMap::load_expect_cloned(
            "common.skill_trees.skill_max_levels",
        ).0
    };
    // Loads the prerequisite skills for a particular skill
    pub static ref SKILL_PREREQUISITES: HashMap<Skill, HashMap<Skill, Option<u16>>> = {
        SkillPrerequisitesMap::load_expect_cloned(
            "common.skill_trees.skill_prerequisites",
        ).0
    };
}

/// Represents a skill that a player can unlock, that either grants them some
/// kind of active ability, or a passive effect etc. Obviously because this is
/// an enum it doesn't describe what the skill actually -does-, this will be
/// handled by dedicated ECS systems.
// NOTE: if skill does use some constant, add it to corresponding
// SkillTree Modifiers below.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum Skill {
    General(GeneralSkill),
    Sword(SwordSkill),
    Axe(AxeSkill),
    Hammer(HammerSkill),
    Bow(BowSkill),
    Staff(StaffSkill),
    Sceptre(SceptreSkill),
    UnlockGroup(SkillGroupKind),
    Roll(RollSkill),
    Climb(ClimbSkill),
    Swim(SwimSkill),
    Pick(MiningSkill),
}

/// Tree of modifiers that represent how stats are
/// changed per each skill level.
///
/// It's used as bridge between ECS systems
/// and voxygen Diary for skill descriptions and helps to sync them.
///
/// NOTE: Just adding constant does nothing, you need to use it in both
/// ECS systems and Diary.
pub struct SkillTreeModifiers {
    pub sword_tree: SwordTreeModifiers,
    pub axe_tree: AxeTreeModifiers,
    pub hammer_tree: HammerTreeModifiers,
    pub bow_tree: BowTreeModifiers,
    pub staff_tree: StaffTreeModifiers,
    pub sceptre_tree: SceptreTreeModifiers,
    pub mining_tree: MiningTreeModifiers,
    pub general_tree: GeneralTreeModifiers,
}

pub struct SwordTreeModifiers {
    pub dash: SwordDashModifiers,
    pub spin: SwordSpinModifiers,
}

pub struct SwordDashModifiers {
    pub energy_cost: f32,
    pub energy_drain: f32,
    pub base_damage: f32,
    pub scaled_damage: f32,
    pub forward_speed: f32,
}

pub struct SwordSpinModifiers {
    pub base_damage: f32,
    pub swing_duration: f32,
    pub energy_cost: f32,
    pub num: u32,
}

impl SwordTreeModifiers {
    pub const fn get() -> Self {
        Self {
            dash: SwordDashModifiers {
                energy_cost: 0.75,
                energy_drain: 0.75,
                base_damage: 1.2,
                scaled_damage: 1.2,
                forward_speed: 1.15,
            },
            spin: SwordSpinModifiers {
                base_damage: 1.4,
                swing_duration: 0.8,
                energy_cost: 0.75,
                num: 1,
            },
        }
    }
}

pub struct AxeTreeModifiers {
    pub spin: AxeSpinModifiers,
    pub leap: AxeLeapModifiers,
}

pub struct AxeSpinModifiers {
    pub base_damage: f32,
    pub swing_duration: f32,
    pub energy_cost: f32,
}

pub struct AxeLeapModifiers {
    pub base_damage: f32,
    pub knockback: f32,
    pub energy_cost: f32,
    // TODO: split to forward and vertical?
    pub leap_strength: f32,
}

impl AxeTreeModifiers {
    pub const fn get() -> Self {
        Self {
            spin: AxeSpinModifiers {
                base_damage: 1.3,
                swing_duration: 0.8,
                energy_cost: 0.75,
            },
            leap: AxeLeapModifiers {
                base_damage: 1.35,
                knockback: 1.4,
                energy_cost: 0.75,
                leap_strength: 1.2,
            },
        }
    }
}

pub struct HammerTreeModifiers {
    pub single_strike: HammerStrikeModifiers,
    pub charged: HammerChargedModifers,
    pub leap: HammerLeapModifiers,
}

pub struct HammerStrikeModifiers {
    pub knockback: f32,
}

pub struct HammerChargedModifers {
    pub scaled_damage: f32,
    pub scaled_knockback: f32,
    pub energy_drain: f32,
    pub charge_rate: f32,
}

pub struct HammerLeapModifiers {
    pub base_damage: f32,
    pub knockback: f32,
    pub energy_cost: f32,
    pub leap_strength: f32,
    pub range: f32,
}

impl HammerTreeModifiers {
    pub const fn get() -> Self {
        Self {
            single_strike: HammerStrikeModifiers { knockback: 1.5 },
            charged: HammerChargedModifers {
                scaled_damage: 1.25,
                scaled_knockback: 1.5,
                energy_drain: 0.75,
                charge_rate: 1.25,
            },
            leap: HammerLeapModifiers {
                base_damage: 1.4,
                knockback: 1.5,
                energy_cost: 0.75,
                leap_strength: 1.25,
                range: 1.0,
            },
        }
    }
}

pub struct BowTreeModifiers {
    pub universal: BowUniversalModifiers,
    pub charged: BowChargedModifiers,
    pub repeater: BowRepeaterModifiers,
    pub shotgun: BowShotgunModifiers,
}

pub struct BowUniversalModifiers {
    // TODO: split per abilities?
    pub projectile_speed: f32,
}

pub struct BowChargedModifiers {
    pub damage_scaling: f32,
    pub regen_scaling: f32,
    pub knockback_scaling: f32,
    pub charge_rate: f32,
    pub move_speed: f32,
}

pub struct BowRepeaterModifiers {
    pub power: f32,
    pub energy_cost: f32,
    pub max_speed: f32,
}

pub struct BowShotgunModifiers {
    pub power: f32,
    pub energy_cost: f32,
    pub num_projectiles: u32,
    pub spread: f32,
}

impl BowTreeModifiers {
    pub const fn get() -> Self {
        Self {
            universal: BowUniversalModifiers {
                projectile_speed: 1.2,
            },
            charged: BowChargedModifiers {
                damage_scaling: 1.2,
                regen_scaling: 1.2,
                knockback_scaling: 1.2,
                charge_rate: 1.1,
                move_speed: 1.1,
            },
            repeater: BowRepeaterModifiers {
                power: 1.2,
                energy_cost: 0.8,
                max_speed: 1.2,
            },
            shotgun: BowShotgunModifiers {
                power: 1.2,
                energy_cost: 1.2,
                num_projectiles: 1,
                spread: 0.8,
            },
        }
    }
}

pub struct StaffTreeModifiers {
    pub fireball: StaffFireballModifiers,
    pub flamethrower: StaffFlamethrowerModifiers,
    pub shockwave: StaffShockwaveModifiers,
}

pub struct StaffFireballModifiers {
    pub power: f32,
    pub regen: f32,
    pub range: f32,
}

pub struct StaffFlamethrowerModifiers {
    pub damage: f32,
    pub range: f32,
    pub energy_drain: f32,
    pub velocity: f32,
}

pub struct StaffShockwaveModifiers {
    pub damage: f32,
    pub knockback: f32,
    pub duration: f32,
    pub energy_cost: f32,
}

impl StaffTreeModifiers {
    pub const fn get() -> Self {
        Self {
            fireball: StaffFireballModifiers {
                power: 1.2,
                regen: 1.2,
                range: 1.15,
            },
            flamethrower: StaffFlamethrowerModifiers {
                damage: 1.3,
                range: 1.25,
                energy_drain: 0.8,
                velocity: 1.25,
            },
            shockwave: StaffShockwaveModifiers {
                damage: 1.3,
                knockback: 1.3,
                duration: 1.2,
                energy_cost: 0.8,
            },
        }
    }
}

pub struct SceptreTreeModifiers {
    pub beam: SceptreBeamModifiers,
    pub healing_aura: SceptreHealingAuraModifiers,
    pub warding_aura: SceptreWardingAuraModifiers,
}

pub struct SceptreBeamModifiers {
    pub damage: f32,
    pub range: f32,
    pub energy_regen: f32,
    pub lifesteal: f32,
}

pub struct SceptreHealingAuraModifiers {
    pub strength: f32,
    pub duration: f32,
    pub range: f32,
    pub energy_cost: f32,
}

pub struct SceptreWardingAuraModifiers {
    pub strength: f32,
    pub duration: f32,
    pub range: f32,
    pub energy_cost: f32,
}

impl SceptreTreeModifiers {
    pub const fn get() -> Self {
        Self {
            beam: SceptreBeamModifiers {
                damage: 1.2,
                range: 1.2,
                energy_regen: 1.2,
                lifesteal: 1.15,
            },
            healing_aura: SceptreHealingAuraModifiers {
                strength: 1.15,
                duration: 1.2,
                range: 1.25,
                energy_cost: 0.85,
            },
            warding_aura: SceptreWardingAuraModifiers {
                strength: 1.15,
                duration: 1.2,
                range: 1.25,
                energy_cost: 0.85,
            },
        }
    }
}

pub struct MiningTreeModifiers {
    pub speed: f32,
    pub gem_gain: f32,
    pub ore_gain: f32,
}

impl MiningTreeModifiers {
    pub const fn get() -> Self {
        Self {
            speed: 1.1,
            gem_gain: 0.05,
            ore_gain: 0.05,
        }
    }
}

pub struct GeneralTreeModifiers {
    pub roll: RollTreeModifiers,
    pub swim: SwimTreeModifiers,
    pub climb: ClimbTreeModifiers,
}

pub struct RollTreeModifiers {
    pub energy_cost: f32,
    pub strength: f32,
    pub duration: f32,
}

pub struct SwimTreeModifiers {
    pub speed: f32,
}

pub struct ClimbTreeModifiers {
    pub energy_cost: f32,
    pub speed: f32,
}

impl GeneralTreeModifiers {
    pub const fn get() -> Self {
        Self {
            roll: RollTreeModifiers {
                energy_cost: 0.9,
                strength: 1.1,
                duration: 1.1,
            },
            swim: SwimTreeModifiers { speed: 1.25 },
            climb: ClimbTreeModifiers {
                energy_cost: 0.8,
                speed: 1.2,
            },
        }
    }
}

/// Enum which returned as result from `boost` function
/// `Number` can represent values from -inf to +inf,
/// but it should generaly be in range -50..50
///
/// Number(-25) says that some value
/// will be reduced by 25% (for example energy consumption)
///
/// Number(15) says that some value
/// will be increased by 15% (for example damage)
// TODO: move it to voxygen diary code
// (and inline directly to formating skill descriptions)
#[derive(Debug, Clone, Copy)]
pub enum BoostValue {
    Number(i16),
    NonDescriptive,
}

impl From<i16> for BoostValue {
    fn from(number: i16) -> Self { BoostValue::Number(number) }
}

/// Returns value which corresponds to the boost given by this skill
pub trait Boost {
    fn boost(self) -> BoostValue;
}

impl Boost for Skill {
    fn boost(self) -> BoostValue {
        match self {
            // General tree boosts
            Skill::General(s) => s.boost(),
            // Weapon tree boosts
            Skill::Sword(s) => s.boost(),
            Skill::Axe(s) => s.boost(),
            Skill::Hammer(s) => s.boost(),
            Skill::Bow(s) => s.boost(),
            Skill::Staff(s) => s.boost(),
            Skill::Sceptre(s) => s.boost(),

            // Movement tree boosts
            Skill::Roll(s) => s.boost(),
            Skill::Climb(s) => s.boost(),
            Skill::Swim(s) => s.boost(),
            // Non-combat tree boosts
            Skill::Pick(s) => s.boost(),
            // Unlock Group has more complex semantic
            Skill::UnlockGroup(_) => BoostValue::NonDescriptive,
        }
    }
}

pub enum SkillError {
    MissingSkill,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwordSkill {
    // Sword passives
    InterruptingAttacks,
    // Triple strike upgrades
    TsCombo,
    TsDamage,
    TsRegen,
    TsSpeed,
    // Dash upgrades
    DCost,
    DDrain,
    DDamage,
    DScaling,
    DSpeed,
    DInfinite, // Represents charge through, not migrated because laziness
    // Spin upgrades
    UnlockSpin,
    SDamage,
    SSpeed,
    SCost,
    SSpins,
}

impl Boost for SwordSkill {
    fn boost(self) -> BoostValue {
        match self {
            // Dash
            Self::DDamage => 20.into(),
            Self::DCost => (-25_i16).into(),
            Self::DDrain => (-25_i16).into(),
            Self::DScaling => 20.into(),
            Self::DSpeed => 15.into(),
            // Spin
            Self::SDamage => 40.into(),
            Self::SSpeed => (-20_i16).into(),
            Self::SCost => (-25_i16).into(),
            // Non-descriptive values
            Self::InterruptingAttacks
            | Self::TsCombo
            | Self::TsDamage
            | Self::TsRegen
            | Self::TsSpeed
            | Self::DInfinite
            | Self::UnlockSpin
            | Self::SSpins => BoostValue::NonDescriptive,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum AxeSkill {
    // Double strike upgrades
    DsCombo,
    DsDamage,
    DsSpeed,
    DsRegen,
    // Spin upgrades
    SInfinite,
    SHelicopter,
    SDamage,
    SSpeed,
    SCost,
    // Leap upgrades
    UnlockLeap,
    LDamage,
    LKnockback,
    LCost,
    LDistance,
}

impl Boost for AxeSkill {
    fn boost(self) -> BoostValue {
        match self {
            // Spin upgrades
            Self::SDamage => 30.into(),
            Self::SSpeed => (-20_i16).into(),
            Self::SCost => (-25_i16).into(),
            // Leap upgrades
            Self::LDamage => 35.into(),
            Self::LKnockback => 40.into(),
            Self::LCost => (-25_i16).into(),
            Self::LDistance => 20.into(),
            // Non-descriptive boosts
            Self::UnlockLeap
            | Self::DsCombo
            | Self::DsDamage
            | Self::DsSpeed
            | Self::DsRegen
            | Self::SInfinite
            | Self::SHelicopter => BoostValue::NonDescriptive,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum HammerSkill {
    // Single strike upgrades
    SsKnockback,
    SsDamage,
    SsSpeed,
    SsRegen,
    // Charged melee upgrades
    CDamage,
    CKnockback,
    CDrain,
    CSpeed,
    // Leap upgrades
    UnlockLeap,
    LDamage,
    LCost,
    LDistance,
    LKnockback,
    LRange,
}

impl Boost for HammerSkill {
    fn boost(self) -> BoostValue {
        match self {
            // Single strike upgrades
            Self::SsKnockback => 50.into(),
            // Charged melee upgrades
            Self::CDamage => 25.into(),
            Self::CKnockback => 50.into(),
            Self::CDrain => (-25_i16).into(),
            Self::CSpeed => 25.into(),
            // Leap upgrades
            Self::LDamage => 40.into(),
            Self::LKnockback => 50.into(),
            Self::LCost => (-25_i16).into(),
            Self::LDistance => 25.into(),
            Self::LRange => 1.into(),
            // Non-descriptive values
            Self::UnlockLeap | Self::SsDamage | Self::SsSpeed | Self::SsRegen => {
                BoostValue::NonDescriptive
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum BowSkill {
    // Passives
    ProjSpeed,
    // Charged upgrades
    CDamage,
    CRegen,
    CKnockback,
    CSpeed,
    CMove,
    // Repeater upgrades
    RDamage,
    RCost,
    RSpeed,
    // Shotgun upgrades
    UnlockShotgun,
    SDamage,
    SCost,
    SArrows,
    SSpread,
}

impl Boost for BowSkill {
    fn boost(self) -> BoostValue {
        match self {
            // Passive
            Self::ProjSpeed => 20.into(),
            // Charged upgrades
            Self::CDamage => 20.into(),
            Self::CRegen => 20.into(),
            Self::CKnockback => 20.into(),
            Self::CSpeed => 10.into(),
            Self::CMove => 10.into(),
            // Repeater upgrades
            Self::RDamage => 20.into(),
            Self::RCost => (-20_i16).into(),
            Self::RSpeed => 20.into(),
            // Shotgun upgrades
            Self::SDamage => 20.into(),
            Self::SCost => (-20_i16).into(),
            Self::SArrows => 1.into(),
            Self::SSpread => (-20_i16).into(),
            // Non-descriptive values
            Self::UnlockShotgun => BoostValue::NonDescriptive,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum StaffSkill {
    // Basic ranged upgrades
    BDamage,
    BRegen,
    BRadius,
    // Flamethrower upgrades
    FDamage,
    FRange,
    FDrain,
    FVelocity,
    // Shockwave upgrades
    UnlockShockwave,
    SDamage,
    SKnockback,
    SRange,
    SCost,
}

impl Boost for StaffSkill {
    fn boost(self) -> BoostValue {
        match self {
            // Fireball upgrades
            Self::BDamage => 20.into(),
            Self::BRegen => 20.into(),
            Self::BRadius => 15.into(),
            // Flamethrower upgrades
            Self::FDamage => 30.into(),
            Self::FRange => 25.into(),
            Self::FDrain => (-20_i16).into(),
            Self::FVelocity => 25.into(),
            // Shockwave upgrades
            Self::SDamage => 30.into(),
            Self::SKnockback => 30.into(),
            Self::SRange => 20.into(),
            Self::SCost => (-20_i16).into(),
            // Non-descriptive values
            Self::UnlockShockwave => BoostValue::NonDescriptive,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceptreSkill {
    // Lifesteal beam upgrades
    LDamage,
    LRange,
    LLifesteal,
    LRegen,
    // Healing beam upgrades
    HHeal,
    HRange,
    HDuration,
    HCost,
    // Warding aura upgrades
    UnlockAura,
    AStrength,
    ADuration,
    ARange,
    ACost,
}

impl Boost for SceptreSkill {
    fn boost(self) -> BoostValue {
        match self {
            // Lifesteal beam upgrades
            Self::LDamage => 20.into(),
            Self::LRange => 20.into(),
            Self::LRegen => 20.into(),
            Self::LLifesteal => 15.into(),
            // Healing beam upgrades
            Self::HHeal => 20.into(),
            Self::HRange => 20.into(),
            Self::HDuration => 20.into(),
            Self::HCost => (-20_i16).into(),
            // Warding aura upgrades
            Self::AStrength => 15.into(),
            Self::ADuration => 20.into(),
            Self::ARange => 25.into(),
            Self::ACost => (-15_i16).into(),
            Self::UnlockAura => BoostValue::NonDescriptive,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeneralSkill {
    HealthIncrease,
    EnergyIncrease,
}

impl Boost for GeneralSkill {
    fn boost(self) -> BoostValue {
        // NOTE: These should be used only for UI.
        // Source of truth are corresponding systems
        match self {
            Self::HealthIncrease => {
                let health_increase =
                    (Body::Humanoid(humanoid::Body::random()).base_health_increase() / 10) as i16;
                health_increase.into()
            },
            Self::EnergyIncrease => {
                let energy_increase = (comp::energy::ENERGY_PER_LEVEL / 10) as i16;
                energy_increase.into()
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum RollSkill {
    Cost,
    Strength,
    Duration,
}

impl Boost for RollSkill {
    fn boost(self) -> BoostValue {
        match self {
            Self::Cost => (-10_i16).into(),
            Self::Strength => 10.into(),
            Self::Duration => 10.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClimbSkill {
    Cost,
    Speed,
}

impl Boost for ClimbSkill {
    fn boost(self) -> BoostValue {
        match self {
            Self::Cost => (-20_i16).into(),
            Self::Speed => 20.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwimSkill {
    Speed,
}

impl Boost for SwimSkill {
    fn boost(self) -> BoostValue {
        match self {
            Self::Speed => 25.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum MiningSkill {
    Speed,
    OreGain,
    GemGain,
}

impl Boost for MiningSkill {
    fn boost(self) -> BoostValue {
        match self {
            Self::Speed => 10.into(),
            Self::OreGain => 5.into(),
            Self::GemGain => 5.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillGroupKind {
    General,
    Weapon(ToolKind),
}

impl SkillGroupKind {
    /// Gets the cost in experience of earning a skill point
    pub fn skill_point_cost(self, level: u16) -> u16 {
        const EXP_INCREMENT: f32 = 10.0;
        const STARTING_EXP: f32 = 70.0;
        const EXP_CEILING: f32 = 1000.0;
        const SCALING_FACTOR: f32 = 0.125;
        (EXP_INCREMENT
            * (EXP_CEILING
                / EXP_INCREMENT
                / (1.0
                    + std::f32::consts::E.powf(-SCALING_FACTOR * level as f32)
                        * (EXP_CEILING / STARTING_EXP - 1.0)))
                .floor()) as u16
    }

    /// Gets the total amount of skill points that can be spent in a particular
    /// skill group
    pub fn total_skill_point_cost(self) -> u16 {
        if let Some(SkillGroupDef {
            total_skill_point_cost,
            ..
        }) = SKILL_GROUP_DEFS.get(&self)
        {
            *total_skill_point_cost
        } else {
            0
        }
    }
}

/// A group of skills that have been unlocked by a player. Each skill group has
/// independent exp and skill points which are used to unlock skills in that
/// skill group.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct SkillGroup {
    pub skill_group_kind: SkillGroupKind,
    pub exp: u16,
    pub available_sp: u16,
    pub earned_sp: u16,
}

impl SkillGroup {
    fn new(skill_group_kind: SkillGroupKind) -> SkillGroup {
        SkillGroup {
            skill_group_kind,
            exp: 0,
            available_sp: 0,
            earned_sp: 0,
        }
    }
}

/// Contains all of a player's skill groups and skills. Provides methods for
/// manipulating assigned skills and skill groups including unlocking skills,
/// refunding skills etc.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SkillSet {
    pub skill_groups: Vec<SkillGroup>,
    pub skills: HashMap<Skill, Option<u16>>,
    pub modify_health: bool,
    pub modify_energy: bool,
}

impl Component for SkillSet {
    type Storage = DerefFlaggedStorage<Self, IdvStorage<Self>>;
}

impl Default for SkillSet {
    /// Instantiate a new skill set with the default skill groups with no
    /// unlocked skills in them - used when adding a skill set to a new
    /// player
    fn default() -> Self {
        Self {
            skill_groups: vec![
                SkillGroup::new(SkillGroupKind::General),
                SkillGroup::new(SkillGroupKind::Weapon(ToolKind::Pick)),
            ],
            skills: HashMap::new(),
            modify_health: false,
            modify_energy: false,
        }
    }
}

impl SkillSet {
    ///  Unlocks a skill group for a player. It starts with 0 exp and 0 skill
    ///  points.
    ///
    /// ```
    /// use veloren_common::comp::{
    ///     item::tool::ToolKind,
    ///     skills::{SkillGroupKind, SkillSet},
    /// };
    ///
    /// let mut skillset = SkillSet::default();
    /// skillset.unlock_skill_group(SkillGroupKind::Weapon(ToolKind::Sword));
    ///
    /// assert_eq!(skillset.skill_groups.len(), 3);
    /// ```
    pub fn unlock_skill_group(&mut self, skill_group_kind: SkillGroupKind) {
        if !self.contains_skill_group(skill_group_kind) {
            self.skill_groups.push(SkillGroup::new(skill_group_kind));
        } else {
            warn!("Tried to unlock already known skill group");
        }
    }

    /// Unlocks a skill for a player, assuming they have the relevant skill
    /// group unlocked and available SP in that skill group.
    ///
    /// ```
    /// use veloren_common::comp::skills::{GeneralSkill, Skill, SkillGroupKind, SkillSet};
    ///
    /// let mut skillset = SkillSet::default();
    /// skillset.add_skill_points(SkillGroupKind::General, 1);
    ///
    /// skillset.unlock_skill(Skill::General(GeneralSkill::HealthIncrease));
    ///
    /// assert_eq!(skillset.skills.len(), 1);
    /// ```
    pub fn unlock_skill(&mut self, skill: Skill) {
        if let Some(skill_group_kind) = skill.skill_group_kind() {
            let next_level = self.next_skill_level(skill);
            let prerequisites_met = self.prerequisites_met(skill);
            if !matches!(self.skills.get(&skill), Some(level) if *level == skill.max_level()) {
                if let Some(mut skill_group) = self.skill_group_mut(skill_group_kind) {
                    if prerequisites_met {
                        if skill_group.available_sp >= skill.skill_cost(next_level) {
                            skill_group.available_sp -= skill.skill_cost(next_level);
                            if let Skill::UnlockGroup(group) = skill {
                                self.unlock_skill_group(group);
                            }
                            if matches!(skill, Skill::General(GeneralSkill::HealthIncrease)) {
                                self.modify_health = true;
                            }
                            if matches!(skill, Skill::General(GeneralSkill::EnergyIncrease)) {
                                self.modify_energy = true;
                            }
                            self.skills.insert(skill, next_level);
                        } else {
                            trace!("Tried to unlock skill for skill group with insufficient SP");
                        }
                    } else {
                        trace!("Tried to unlock skill without meeting prerequisite skills");
                    }
                } else {
                    trace!("Tried to unlock skill for a skill group that player does not have");
                }
            } else {
                trace!("Tried to unlock skill the player already has")
            }
        } else {
            warn!(
                ?skill,
                "Tried to unlock skill that does not exist in any skill group!"
            );
        }
    }

    /// Removes a skill from a player and refunds 1 skill point in the relevant
    /// skill group.
    ///
    /// ```
    /// use veloren_common::comp::skills::{GeneralSkill, Skill, SkillGroupKind, SkillSet};
    ///
    /// let mut skillset = SkillSet::default();
    /// skillset.add_skill_points(SkillGroupKind::General, 1);
    /// skillset.unlock_skill(Skill::General(GeneralSkill::HealthIncrease));
    ///
    /// skillset.refund_skill(Skill::General(GeneralSkill::HealthIncrease));
    ///
    /// assert_eq!(skillset.skills.len(), 0);
    /// ```
    pub fn refund_skill(&mut self, skill: Skill) {
        if let Ok(level) = self.skill_level(skill) {
            if let Some(skill_group_kind) = skill.skill_group_kind() {
                if let Some(mut skill_group) = self.skill_group_mut(skill_group_kind) {
                    skill_group.available_sp += skill.skill_cost(level);
                    if level.map_or(false, |l| l > 1) {
                        self.skills.insert(skill, level.map(|l| l - 1));
                    } else {
                        self.skills.remove(&skill);
                    }
                } else {
                    warn!("Tried to refund skill for a skill group that player does not have");
                }
            } else {
                warn!(
                    ?skill,
                    "Tried to refund skill that does not exist in any skill group"
                )
            }
        } else {
            warn!("Tried to refund skill that has not been unlocked");
        }
    }

    /// Adds skill points to a skill group as long as the player has that skill
    /// group type.
    ///
    /// ```
    /// use veloren_common::comp::skills::{SkillGroupKind, SkillSet};
    ///
    /// let mut skillset = SkillSet::default();
    /// skillset.add_skill_points(SkillGroupKind::General, 1);
    ///
    /// assert_eq!(skillset.skill_groups[0].available_sp, 1);
    /// ```
    pub fn add_skill_points(
        &mut self,
        skill_group_kind: SkillGroupKind,
        number_of_skill_points: u16,
    ) {
        if let Some(mut skill_group) = self.skill_group_mut(skill_group_kind) {
            skill_group.available_sp = skill_group
                .available_sp
                .saturating_add(number_of_skill_points);
            skill_group.earned_sp = skill_group.earned_sp.saturating_add(number_of_skill_points);
        } else {
            warn!("Tried to add skill points to a skill group that player does not have");
        }
    }

    /// Adds a skill point while subtracting the necessary amount of experience
    pub fn earn_skill_point(&mut self, skill_group_kind: SkillGroupKind) {
        let sp_cost = self.skill_point_cost(skill_group_kind);
        if let Some(mut skill_group) = self.skill_group_mut(skill_group_kind) {
            skill_group.exp = skill_group.exp.saturating_sub(sp_cost);
            skill_group.available_sp = skill_group.available_sp.saturating_add(1);
            skill_group.earned_sp = skill_group.earned_sp.saturating_add(1);
        }
    }

    /// Checks if the skill set of an entity contains a particular skill group
    /// type
    pub fn contains_skill_group(&self, skill_group_kind: SkillGroupKind) -> bool {
        self.skill_groups
            .iter()
            .any(|x| x.skill_group_kind == skill_group_kind)
    }

    /// Adds/subtracts experience to the skill group within an entity's skill
    /// set
    pub fn change_experience(&mut self, skill_group_kind: SkillGroupKind, amount: i32) {
        if let Some(mut skill_group) = self.skill_group_mut(skill_group_kind) {
            skill_group.exp = (skill_group.exp as i32 + amount) as u16;
        } else {
            warn!("Tried to add experience to a skill group that player does not have");
        }
    }

    /// Checks that the skill set contains all prerequisite skills for a
    /// particular skill
    pub fn prerequisites_met(&self, skill: Skill) -> bool {
        skill
            .prerequisite_skills()
            .all(|(s, l)| self.skill_level(s).map_or(false, |l_b| l_b >= l))
    }

    /// Returns a reference to a particular skill group in a skillset
    fn skill_group(&self, skill_group: SkillGroupKind) -> Option<&SkillGroup> {
        self.skill_groups
            .iter()
            .find(|s_g| s_g.skill_group_kind == skill_group)
    }

    /// Returns a reference to a particular skill group in a skillset
    fn skill_group_mut(&mut self, skill_group: SkillGroupKind) -> Option<&mut SkillGroup> {
        self.skill_groups
            .iter_mut()
            .find(|s_g| s_g.skill_group_kind == skill_group)
    }

    /// Gets the available points for a particular skill group
    pub fn available_sp(&self, skill_group: SkillGroupKind) -> u16 {
        self.skill_group(skill_group)
            .map_or(0, |s_g| s_g.available_sp)
    }

    /// Gets the total earned points for a particular skill group
    pub fn earned_sp(&self, skill_group: SkillGroupKind) -> u16 {
        self.skill_group(skill_group).map_or(0, |s_g| s_g.earned_sp)
    }

    /// Gets the available experience for a particular skill group
    pub fn experience(&self, skill_group: SkillGroupKind) -> u16 {
        self.skill_group(skill_group).map_or(0, |s_g| s_g.exp)
    }

    /// Gets skill point cost to purchase skill of next level
    pub fn skill_cost(&self, skill: Skill) -> u16 {
        let next_level = self.next_skill_level(skill);
        skill.skill_cost(next_level)
    }

    /// Checks if player has sufficient skill points to purchase a skill
    pub fn sufficient_skill_points(&self, skill: Skill) -> bool {
        if let Some(skill_group_kind) = skill.skill_group_kind() {
            if let Some(skill_group) = self
                .skill_groups
                .iter()
                .find(|x| x.skill_group_kind == skill_group_kind)
            {
                let needed_sp = self.skill_cost(skill);
                skill_group.available_sp >= needed_sp
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Checks if the player has available SP to spend
    pub fn has_available_sp(&self) -> bool {
        self.skill_groups.iter().any(|sg| {
            sg.available_sp > 0
                && (sg.earned_sp - sg.available_sp) < sg.skill_group_kind.total_skill_point_cost()
        })
    }

    /// Checks how much experience is needed for the next skill point in a tree
    pub fn skill_point_cost(&self, skill_group: SkillGroupKind) -> u16 {
        if let Some(level) = self.skill_group(skill_group).map(|sg| sg.earned_sp) {
            skill_group.skill_point_cost(level)
        } else {
            skill_group.skill_point_cost(0)
        }
    }

    /// Checks if the skill is at max level in a skill set
    pub fn is_at_max_level(&self, skill: Skill) -> bool {
        if let Ok(level) = self.skill_level(skill) {
            level == skill.max_level()
        } else {
            false
        }
    }

    /// Checks if skill set contains a skill
    pub fn has_skill(&self, skill: Skill) -> bool { self.skills.contains_key(&skill) }

    /// Returns the level of the skill
    pub fn skill_level(&self, skill: Skill) -> Result<Option<u16>, SkillError> {
        if let Some(level) = self.skills.get(&skill).copied() {
            Ok(level)
        } else {
            Err(SkillError::MissingSkill)
        }
    }

    /// Returns the level of the skill or passed value as default
    pub fn skill_level_or(&self, skill: Skill, default: u16) -> u16 {
        if let Ok(Some(level)) = self.skill_level(skill) {
            level
        } else {
            default
        }
    }

    /// Checks the next level of a skill
    fn next_skill_level(&self, skill: Skill) -> Option<u16> {
        if let Ok(level) = self.skill_level(skill) {
            level.map(|l| l + 1)
        } else {
            skill.max_level().map(|_| 1)
        }
    }
}

impl Skill {
    /// Returns a vec of prerequisite skills (it should only be necessary to
    /// note direct prerequisites)
    pub fn prerequisite_skills(&self) -> impl Iterator<Item = (Skill, Option<u16>)> {
        SKILL_PREREQUISITES
            .get(self)
            .into_iter()
            .flatten()
            .map(|(skill, level)| (*skill, *level))
    }

    /// Returns the cost in skill points of unlocking a particular skill
    pub fn skill_cost(&self, level: Option<u16>) -> u16 {
        // TODO: Better balance the costs later
        level.unwrap_or(1)
    }

    /// Returns the maximum level a skill can reach, returns None if the skill
    /// doesn't level
    pub fn max_level(&self) -> Option<u16> { SKILL_MAX_LEVEL.get(self).copied().flatten() }

    /// Returns the skill group type for a skill from the static skill group
    /// definitions.
    pub fn skill_group_kind(&self) -> Option<SkillGroupKind> {
        SKILL_GROUP_LOOKUP.get(self).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refund_skill() {
        let mut skillset = SkillSet::default();
        skillset.unlock_skill_group(SkillGroupKind::Weapon(ToolKind::Axe));
        skillset.add_skill_points(SkillGroupKind::Weapon(ToolKind::Axe), 1);
        skillset.unlock_skill(Skill::Axe(AxeSkill::UnlockLeap));

        assert_eq!(skillset.skill_groups[2].available_sp, 0);
        assert_eq!(skillset.skills.len(), 1);
        assert!(skillset.has_skill(Skill::Axe(AxeSkill::UnlockLeap)));

        skillset.refund_skill(Skill::Axe(AxeSkill::UnlockLeap));

        assert_eq!(skillset.skill_groups[2].available_sp, 1);
        assert_eq!(skillset.skills.get(&Skill::Axe(AxeSkill::UnlockLeap)), None);
    }

    #[test]
    fn test_unlock_skillgroup() {
        let mut skillset = SkillSet::default();
        skillset.unlock_skill_group(SkillGroupKind::Weapon(ToolKind::Axe));

        assert_eq!(skillset.skill_groups.len(), 3);
        assert_eq!(
            skillset.skill_groups[2],
            SkillGroup::new(SkillGroupKind::Weapon(ToolKind::Axe))
        );
    }

    #[test]
    fn test_unlock_skill() {
        let mut skillset = SkillSet::default();

        skillset.unlock_skill_group(SkillGroupKind::Weapon(ToolKind::Axe));
        skillset.add_skill_points(SkillGroupKind::Weapon(ToolKind::Axe), 1);

        assert_eq!(skillset.skill_groups[2].available_sp, 1);
        assert_eq!(skillset.skills.len(), 0);

        // Try unlocking a skill with enough skill points
        skillset.unlock_skill(Skill::Axe(AxeSkill::UnlockLeap));

        assert_eq!(skillset.skill_groups[2].available_sp, 0);
        assert_eq!(skillset.skills.len(), 1);
        assert!(skillset.has_skill(Skill::Axe(AxeSkill::UnlockLeap)));

        // Try unlocking a skill without enough skill points
        skillset.unlock_skill(Skill::Axe(AxeSkill::DsCombo));

        assert_eq!(skillset.skills.len(), 1);
        assert_eq!(skillset.skills.get(&Skill::Axe(AxeSkill::DsCombo)), None);
    }

    #[test]
    fn test_add_skill_points() {
        let mut skillset = SkillSet::default();
        skillset.unlock_skill_group(SkillGroupKind::Weapon(ToolKind::Axe));
        skillset.add_skill_points(SkillGroupKind::Weapon(ToolKind::Axe), 1);

        assert_eq!(skillset.skill_groups[2].available_sp, 1);
    }
}
