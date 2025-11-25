#![allow(unused)]

use std::collections::HashMap;
use std::io::Read;

use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::wfinfo::item_data::{DucatItem, FilteredItems};
use crate::wfinfo::price_data::PriceItem;

pub mod price_data {
    use palette::num::MinMax;
    use serde::Serialize;

    use super::*;

    #[derive(Clone, Debug, Serialize, Deserialize)]
    pub struct PriceItem {
        pub name: String,
        #[serde(deserialize_with = "serde_aux::prelude::deserialize_number_from_string")]
        pub custom_avg: f32,
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
        pub rare1: Option<String>,
        pub uncommon1: Option<String>,
        pub uncommon2: Option<String>,
        pub common1: Option<String>,
        pub common2: Option<String>,
        pub common3: Option<String>,
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

pub fn load_from_str<T: DeserializeOwned>(json: &str) -> crate::Result<T> {
    serde_json::from_str::<T>(json).map_err(Into::into)
}

pub fn load_from_reader<T: DeserializeOwned>(json: impl Read) -> crate::Result<T> {
    serde_json::from_reader::<_, T>(json).map_err(Into::into)
}

#[derive(Debug, Clone)]
pub struct Item {
    tokens: Vec<String>,
    pub name: String,
    pub platinum: Option<f32>,
    pub ducats: Option<usize>,
    pub ignored: bool,
    pub vaulted: bool,
}

impl Item {
    pub fn new(
        name: String,
        platinum: Option<f32>,
        ducats: Option<usize>,
        ignored: bool,
        vaulted: bool,
    ) -> Self {
        Self {
            tokens: name.split_ascii_whitespace().map(str::to_owned).collect(),
            name,
            platinum,
            ducats,
            ignored,
            vaulted,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Items {
    items: Vec<Item>,
    min_len: usize,
    max_len: usize,
}

impl Items {
    pub fn new(price_items: Vec<PriceItem>, filtered_items: FilteredItems) -> Self {
        if price_items.is_empty() {
            return Self {
                items: vec![],
                min_len: 0,
                max_len: 0,
            };
        }

        let mut items = vec![];

        let FilteredItems {
            eqmt,
            ignored_items,
            ..
        } = filtered_items;

        items.extend(
            ignored_items.into_keys().map(|name| Item::new(name, None, None, true, false)),
        );

        let eqmt = eqmt
            .into_iter()
            .flat_map(|(_, e)| e.parts.into_iter().map(move |item| (e.vaulted, item)));

        for (vaulted, (name, item)) in eqmt {
            let platinum = price_items
                .iter() //
                .filter(|item| !item.name.ends_with("Set"))
                .find(|item| {
                    item.name
                        // some reason there is a single typo in prices api
                        // "Kompressa Prime Receiver" turns into "Kompressa Prime Reciever"
                        .replace("Reciever", "Receiver")
                        .starts_with(&name)
                })
                .map(|item| item.custom_avg);

            let item = Item::new(name, platinum, Some(item.ducats), false, vaulted);
            items.push(item);
        }

        let min_len = items.iter().map(|item| item.name.len()).min().unwrap_or(0);
        let max_len = items.iter().map(|item| item.name.len()).max().unwrap_or(0);

        Self {
            items,
            min_len,
            max_len,
        }
    }
}

impl Items {
    pub fn find_item(&self, item_name: &str) -> Option<Item> {
        let item_name = item_name.trim();

        if !(self.min_len..=self.max_len).contains(&item_name.len()) {
            return None;
        }

        let mut current_matches = self.items.clone();

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

            current_matches.retain(|item| item.tokens[i] == best_matched_token);
        }

        current_matches.into_iter().next()
    }
}
