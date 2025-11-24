#![allow(unused)]

use crate::wfinfo::price_data::PriceItem;
use serde::Deserialize;
use std::io::Read;

pub mod price_data {
    use super::*;
    use palette::num::MinMax;
    use serde::Serialize;

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct PriceItem {
        pub name: String,
        #[serde(deserialize_with = "serde_aux::prelude::deserialize_number_from_string")]
        pub yesterday_vol: u32,
        #[serde(deserialize_with = "serde_aux::prelude::deserialize_number_from_string")]
        pub today_vol: u32,
        #[serde(deserialize_with = "serde_aux::prelude::deserialize_number_from_string")]
        pub custom_avg: f32,
    }

    impl PriceItem {
        pub fn get_price(&self) -> u32 {
            let (min, max) = self.yesterday_vol.min_max(self.today_vol);

            min + ((max - min) / 2)
        }
    }
}

pub mod item_data {
    use std::collections::HashMap;

    use super::*;

    #[derive(Clone, Debug, Deserialize)]
    pub struct DucatItem {
        #[serde(default)]
        pub ducats: usize,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub enum EquipmentType {
        Warframes,
        Primary,
        Secondary,
        Melee,
        Sentinels,
        Archwing,
        #[serde(rename = "Arch-Gun")]
        ArchGun,
        Skins,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct EquipmentItem {
        #[serde(rename = "type")]
        pub item_type: EquipmentType,
        pub vaulted: bool,
        pub parts: HashMap<String, DucatItem>,
    }

    #[derive(Copy, Clone, Debug)]
    pub enum Refinement {
        Intact,
        Exceptional,
        Flawless,
        Radiant,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct Relic {
        pub vaulted: bool,
        pub rare1: String,
        pub uncommon1: String,
        pub uncommon2: String,
        pub common1: String,
        pub common2: String,
        pub common3: String,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct Relics {
        #[serde(rename = "Lith")]
        pub lith: HashMap<String, Relic>,
        #[serde(rename = "Neo")]
        pub neo: HashMap<String, Relic>,
        #[serde(rename = "Meso")]
        pub meso: HashMap<String, Relic>,
        #[serde(rename = "Axi")]
        pub axi: HashMap<String, Relic>,
    }

    #[derive(Clone, Debug, Deserialize)]
    pub struct FilteredItems {
        pub errors: Vec<String>,
        pub relics: Relics,
        pub eqmt: HashMap<String, EquipmentItem>,
        pub ignored_items: HashMap<String, DucatItem>,
    }

    impl Refinement {
        pub fn common_chance(&self) -> f32 {
            match self {
                Refinement::Intact => 0.2533,
                Refinement::Exceptional => 0.2333,
                Refinement::Flawless => 0.2,
                Refinement::Radiant => 0.1667,
            }
        }

        pub fn uncommon_chance(&self) -> f32 {
            match self {
                Refinement::Intact => 0.11,
                Refinement::Exceptional => 0.13,
                Refinement::Flawless => 0.17,
                Refinement::Radiant => 0.20,
            }
        }

        pub fn rare_chance(&self) -> f32 {
            match self {
                Refinement::Intact => 0.02,
                Refinement::Exceptional => 0.04,
                Refinement::Flawless => 0.06,
                Refinement::Radiant => 0.1,
            }
        }
    }
}

pub fn load_price_data_from_str(json: &str) -> crate::Result<Vec<PriceItem>> {
    serde_json::from_str::<Vec<PriceItem>>(json).map_err(Into::into)
}

pub fn load_price_data_from_reader(json: impl Read) -> crate::Result<Vec<PriceItem>> {
    serde_json::from_reader::<_, Vec<PriceItem>>(json).map_err(Into::into)
}

#[derive(Debug, Clone)]
pub struct Item {
    tokens: Vec<String>,
    item: PriceItem,
    len: usize,
}

impl Item {
    pub fn new(item: PriceItem) -> Self {
        Self {
            tokens: item
                .name
                .split_ascii_whitespace()
                .map(str::to_owned)
                .collect(),
            len: item.name.len(),
            item,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Items {
    tokens: Vec<Item>,
    min_len: usize,
    max_len: usize,
}

impl Items {
    pub fn new(items: Vec<PriceItem>) -> Self {
        if items.is_empty() {
            return Self {
                tokens: vec![],
                min_len: 0,
                max_len: 0,
            };
        }

        let tokens = items
            .into_iter()
            .filter(|item| !item.name.ends_with("Set"))
            .map(Item::new)
            .collect::<Vec<_>>();

        let min_len = tokens.iter().map(|item| item.len).min().unwrap_or(0);
        let max_len = tokens.iter().map(|item| item.len).max().unwrap_or(0);

        Self {
            tokens,
            min_len,
            max_len,
        }
    }
}

impl Items {
    pub fn find_item(&self, item_name: &str) -> Option<PriceItem> {
        let item_name = item_name.trim();

        if !(self.min_len..=self.max_len).contains(&item_name.len()) {
            return None;
        }

        let mut current_matches = self.tokens.clone();

        for (i, token) in item_name.split_ascii_whitespace().enumerate() {
            let best_match = current_matches
                .iter()
                .filter(|item| i < item.tokens.len())
                .map(|item| (item, levenshtein::levenshtein(&item.tokens[i], token)))
                // .inspect(|(item, score)| println!("{} {score}", item.tokens[i]))
                .filter(|(item, score)| item.tokens[i].len() / 3 >= *score)
                .min_by_key(|(_, score)| *score);

            let Some((best_match, score)) = best_match else {
                continue;
            };

            let best_matched_token = best_match.tokens[i].clone();

            current_matches = current_matches
                .into_iter()
                .filter(|item| item.tokens[i] == best_matched_token)
                .collect();
        }

        current_matches.into_iter().map(|item| item.item).next()
    }
}
