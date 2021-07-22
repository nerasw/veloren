use super::{tool, ItemKind, ItemTag, Quality, RawItemDef, TagExampleInfo, ToolKind};
use crate::recipe::{RawRecipe, RawRecipeBook, RawRecipeInput};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModularComponentKind {
    Damage,
    Held,
}

impl ModularComponentKind {
    fn identifier_name(&self) -> &'static str {
        match self {
            ModularComponentKind::Damage => "damage",
            ModularComponentKind::Held => "held",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModularComponent {
    pub toolkind: ToolKind,
    pub modkind: ModularComponentKind,
    pub stats: tool::Stats,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModularComponentTag {
    toolkind: ToolKind,
    modkind: ModularComponentKind,
}

impl TagExampleInfo for ModularComponentTag {
    fn name(&self) -> &'static str {
        match self.modkind {
            ModularComponentKind::Damage => match self.toolkind {
                ToolKind::Sword => "sword blade",
                ToolKind::Axe => "axe head",
                ToolKind::Hammer => "hammer head",
                ToolKind::Bow => "bow limbs",
                ToolKind::Dagger => "dagger blade",
                ToolKind::Staff => "staff head",
                ToolKind::Sceptre => "sceptre head",
                // TODO: naming
                ToolKind::Spear => "spear damage component",
                ToolKind::Blowgun => "blowgun damage component",
                ToolKind::Shield => "shield damage component",
                ToolKind::Debug => "debug damage component",
                ToolKind::Farming => "farming damage component",
                ToolKind::Pick => "pickaxe head",
                ToolKind::Natural => "natural damage component",
                ToolKind::Empty => "empty damage component",
            },
            ModularComponentKind::Held => match self.toolkind {
                ToolKind::Sword => "sword hilt",
                ToolKind::Axe => "axe shaft",
                ToolKind::Hammer => "hammer shaft",
                ToolKind::Bow => "bow riser",
                ToolKind::Dagger => "dagger grip",
                ToolKind::Staff => "staff shaft",
                ToolKind::Sceptre => "sceptre shaft",
                // TODO: naming
                ToolKind::Spear => "spear held component",
                ToolKind::Blowgun => "blowgun held component",
                ToolKind::Shield => "shield held component",
                ToolKind::Natural => "natural held component",
                ToolKind::Debug => "debug held component",
                ToolKind::Farming => "farming held component",
                ToolKind::Pick => "pickaxe handle",
                ToolKind::Empty => "empty held component",
            },
        }
    }

    fn exemplar_identifier(&self) -> &'static str {
        match self.modkind {
            ModularComponentKind::Damage => match self.toolkind {
                ToolKind::Sword => "common.items.tag_examples.modular.damage.sword",
                ToolKind::Axe => "common.items.tag_examples.modular.damage.axe",
                ToolKind::Hammer => "common.items.tag_examples.modular.damage.hammer",
                ToolKind::Bow => "common.items.tag_examples.modular.damage.bow",
                ToolKind::Dagger => "common.items.tag_examples.modular.damage.dagger",
                ToolKind::Staff => "common.items.tag_examples.modular.damage.staff",
                ToolKind::Sceptre => "common.items.tag_examples.modular.damage.sceptre",
                ToolKind::Shield => "common.items.tag_examples.modular.damage.shield",
                ToolKind::Spear => "common.items.tag_examples.modular.damage.spear",
                ToolKind::Blowgun => "common.items.tag_examples.modular.damage.blowgun",
                ToolKind::Natural => "common.items.tag_examples.modular.damage.natural",
                ToolKind::Debug => "common.items.tag_examples.modular.damage.debug",
                ToolKind::Farming => "common.items.tag_examples.modular.damage.farming",
                ToolKind::Pick => "common.items.tag_examples.modular.damage.pick",
                ToolKind::Empty => "common.items.tag_examples.modular.damage.empty",
            },
            ModularComponentKind::Held => match self.toolkind {
                ToolKind::Sword => "common.items.tag_examples.modular.held.sword",
                ToolKind::Axe => "common.items.tag_examples.modular.held.axe",
                ToolKind::Hammer => "common.items.tag_examples.modular.held.hammer",
                ToolKind::Bow => "common.items.tag_examples.modular.held.bow",
                ToolKind::Dagger => "common.items.tag_examples.modular.held.dagger",
                ToolKind::Staff => "common.items.tag_examples.modular.held.staff",
                ToolKind::Sceptre => "common.items.tag_examples.modular.held.sceptre",
                ToolKind::Shield => "common.items.tag_examples.modular.held.shield",
                ToolKind::Spear => "common.items.tag_examples.modular.held.spear",
                ToolKind::Blowgun => "common.items.tag_examples.modular.held.blowgun",
                ToolKind::Natural => "common.items.tag_examples.modular.held.natural",
                ToolKind::Debug => "common.items.tag_examples.modular.held.debug",
                ToolKind::Farming => "common.items.tag_examples.modular.held.farming",
                ToolKind::Pick => "common.items.tag_examples.modular.held.pick",
                ToolKind::Empty => "common.items.tag_examples.modular.held.empty",
            },
        }
    }
}

const SUPPORTED_TOOLKINDS: [ToolKind; 6] = [
    ToolKind::Sword,
    ToolKind::Axe,
    ToolKind::Hammer,
    ToolKind::Bow,
    ToolKind::Staff,
    ToolKind::Sceptre,
];
const MODKINDS: [ModularComponentKind; 2] =
    [ModularComponentKind::Damage, ModularComponentKind::Held];

const WEAPON_PREFIX: &str = "common.items.weapons.modular";
const TAG_EXAMPLES_PREFIX: &str = "common.items.tag_examples.modular";

fn make_weapon_def(toolkind: ToolKind) -> (String, RawItemDef) {
    let identifier = format!("{}.{}", WEAPON_PREFIX, toolkind.identifier_name(),);
    let name = format!("Modular {}", toolkind.identifier_name());
    let description = format!("A {} made of components", toolkind.identifier_name());
    let tool = tool::Tool {
        kind: toolkind,
        hands: tool::Hands::Two,
        stats: tool::StatKind::Modular,
    };
    let kind = ItemKind::Tool(tool);
    let quality = Quality::Common;
    let item = RawItemDef {
        name,
        description,
        kind,
        quality,
        tags: Vec::new(),
        slots: 0,
        ability_spec: None,
    };
    (identifier, item)
}

fn make_recipe_def(identifier: String, toolkind: ToolKind) -> RawRecipe {
    let output = (identifier, 1);
    let mut inputs = Vec::new();
    for &modkind in &MODKINDS {
        let input = RawRecipeInput::Tag(ItemTag::ModularComponent(ModularComponentTag {
            toolkind,
            modkind,
        }));
        inputs.push((input, 1, true));
    }
    RawRecipe {
        output,
        inputs,
        craft_sprite: None,
    }
}

fn make_tagexample_def(toolkind: ToolKind, modkind: ModularComponentKind) -> (String, RawItemDef) {
    let identifier = format!(
        "{}.{}.{}",
        TAG_EXAMPLES_PREFIX,
        modkind.identifier_name(),
        toolkind.identifier_name(),
    );
    let tag = ModularComponentTag { toolkind, modkind };
    // TODO: i18n
    let name = format!("Any {}", tag.name());
    let description = format!(
        "{}s used to make {}s",
        tag.name(),
        toolkind.identifier_name()
    );
    let kind = ItemKind::TagExamples {
        // TODO: Iterate over components
        item_ids: Vec::new(),
    };
    let quality = Quality::Common;

    let item = RawItemDef {
        name,
        description,
        kind,
        quality,
        tags: vec![ItemTag::ModularComponent(tag)],
        slots: 0,
        ability_spec: None,
    };
    (identifier, item)
}

fn initialize_modular_assets() -> (HashMap<String, RawItemDef>, RawRecipeBook) {
    let mut itemdefs = HashMap::new();
    let mut recipes = HashMap::new();
    for &toolkind in &SUPPORTED_TOOLKINDS {
        let (identifier, item) = make_weapon_def(toolkind);
        itemdefs.insert(identifier.clone(), item);
        let recipe = make_recipe_def(identifier.clone(), toolkind);
        recipes.insert(identifier, recipe);
    }
    for &toolkind in &SUPPORTED_TOOLKINDS {
        for &modkind in &MODKINDS {
            let (identifier, item) = make_tagexample_def(toolkind, modkind);
            itemdefs.insert(identifier, item);
        }
    }
    (itemdefs, RawRecipeBook(recipes))
}

lazy_static! {
    static ref ITEM_DEFS_AND_RECIPES: (HashMap<String, RawItemDef>, RawRecipeBook) =
        initialize_modular_assets();
}

pub(crate) fn append_modular_recipes(recipes: &mut RawRecipeBook) {
    for (name, recipe) in ITEM_DEFS_AND_RECIPES.1.0.iter() {
        // avoid clobbering recipes from the filesystem, to allow overrides
        if !recipes.0.contains_key(name) {
            recipes.0.insert(name.clone(), recipe.clone());
        }
    }
}

/// Synthesize modular assets programmatically, to allow for the following:
/// - Allow the modular tag_examples to auto-update with the list of applicable
///   components
pub(super) fn synthesize_modular_asset(specifier: &str) -> Option<RawItemDef> {
    let ret = ITEM_DEFS_AND_RECIPES.0.get(specifier).cloned();
    tracing::trace!("synthesize_modular_asset({:?}) -> {:?}", specifier, ret);
    ret
}
