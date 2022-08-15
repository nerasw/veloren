use hashbrown::HashMap;
use serde::{Serialize, Deserialize};
use slotmap::HopSlotMap;
use vek::*;
use std::ops::{Deref, DerefMut};
use common::{
    uid::Uid,
    store::Id,
    rtsim::FactionId,
};
pub use common::rtsim::SiteId;
use world::site::Site as WorldSite;

#[derive(Clone, Serialize, Deserialize)]
pub struct Site {
    pub wpos: Vec2<i32>,
    pub faction: Option<FactionId>,

    /// The site generated during initial worldgen that this site corresponds to.
    ///
    /// Eventually, rtsim should replace initial worldgen's site system and this will not be necessary.
    ///
    /// When setting up rtsim state, we try to 'link' these two definitions of a site: but if initial worldgen has
    /// changed, this might not be possible. We try to delete sites that no longer exist during setup, but this is an
    /// inherent fallible process. If linking fails, we try to delete the site in rtsim2 in order to avoid an
    /// 'orphaned' site. (TODO: create new sites for new initial worldgen sites that come into being too).
    #[serde(skip_serializing, skip_deserializing)]
    pub world_site: Option<Id<WorldSite>>,
}

impl Site {
    pub fn with_faction(mut self, faction: impl Into<Option<FactionId>>) -> Self {
        self.faction = faction.into();
        self
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Sites {
    pub sites: HopSlotMap<SiteId, Site>,
    
    #[serde(skip_serializing, skip_deserializing)]
    pub world_site_map: HashMap<Id<WorldSite>, SiteId>,
}

impl Sites {
    pub fn create(&mut self, site: Site) -> SiteId {
        let world_site = site.world_site;
        let key = self.sites.insert(site);
        if let Some(world_site) = world_site {
            self.world_site_map.insert(world_site, key);
        }
        key
    }
}

impl Deref for Sites {
    type Target = HopSlotMap<SiteId, Site>;
    fn deref(&self) -> &Self::Target { &self.sites }
}

impl DerefMut for Sites {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.sites }
}
