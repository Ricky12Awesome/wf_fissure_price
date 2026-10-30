#![allow(unused)]

use std::collections::HashMap;
use std::io::Read;
use std::time::Duration;

use palette::num::MinMax;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub type PriceItems = Vec<PriceItem>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PriceItem {
    pub name: String,
    #[serde(deserialize_with = "serde_aux::prelude::deserialize_number_from_string")]
    pub custom_avg: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DucatItem {
    #[serde(default)]
    pub ducats: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EquipmentType {
    Warframes,
    Primary,
    Secondary,
    Melee,
    Sentinels,
    Archwing,
    #[serde(rename = "Arch-Gun")]
    ArchGun,
    #[serde(rename = "Arch-Melee")]
    ArchMelee,
    Skins,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EquipmentItem {
    #[serde(rename = "type")]
    pub item_type: EquipmentType,
    pub vaulted: bool,
    pub parts: HashMap<String, DucatItem>,
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum Refinement {
    Intact,
    Exceptional,
    Flawless,
    Radiant,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Relic {
    pub vaulted: bool,
    pub rare1: Option<String>,
    pub uncommon1: Option<String>,
    pub uncommon2: Option<String>,
    pub common1: Option<String>,
    pub common2: Option<String>,
    pub common3: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Default, Debug, Clone)]
pub struct Items {
    items: Vec<Item>,
    min_len: usize,
    max_len: usize,
}

impl Items {
    pub fn new(price_items: PriceItems, filtered_items: FilteredItems) -> Self {
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
            ignored_items
                .into_keys()
                .map(|name| Item::new(name, None, None, true, false)),
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

    pub const fn min_len(&self) -> usize {
        self.min_len
    }

    pub const fn max_len(&self) -> usize {
        self.max_len
    }

    pub fn find_item(&self, item_name: &str) -> Option<Item> {
        fn normalize(name: &str) -> String {
            name.chars()
                .filter(char::is_ascii_alphanumeric)
                .map(|character| character.to_ascii_lowercase())
                .collect()
        }

        let item_name = normalize(item_name);
        if item_name.is_empty() {
            return None;
        }

        let mut best_match = None;
        let mut best_score = usize::MAX;
        let mut ambiguous = false;

        for item in &self.items {
            let Some(first_token) = item.tokens.first() else {
                continue;
            };
            let prefix = normalize(first_token);
            let prefix_errors = prefix.len() / 3;
            let min_prefix = prefix.len() - prefix_errors;
            let max_prefix = (prefix.len() + prefix_errors).min(item_name.len());

            // Generic words such as "Prime Systems" cannot identify the item's family.
            if !(min_prefix..=max_prefix).any(|length| {
                levenshtein::levenshtein(&prefix, &item_name[..length]) <= prefix_errors
            }) {
                continue;
            }

            let mut display_names = vec![normalize(&item.name)];
            if matches!(
                item.tokens.last().map(String::as_str),
                Some("Chassis" | "Neuroptics" | "Systems")
            ) {
                display_names.push(format!("{}blueprint", display_names[0]));
            }

            let score = display_names
                .iter()
                .map(|name| levenshtein::levenshtein(name, &item_name))
                .filter(|score| *score <= item_name.len() / 5)
                .min();

            let Some(score) = score else {
                continue;
            };
            if score < best_score {
                best_match = Some(item);
                best_score = score;
                ambiguous = false;
            } else if score == best_score {
                ambiguous = true;
            }
        }

        if ambiguous { None } else { best_match.cloned() }
    }
}

pub struct WfInfo {
    client: reqwest::Client,
}

impl WfInfo {
    pub fn new() -> crate::Result<Self> {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(5))
            .build()?;

        Ok(Self { client })
    }

    pub async fn fetch(&self) -> crate::Result<(PriceItems, FilteredItems)> {
        let prices = self.fetch_prices().await?;
        let filtered = self.fetch_filtered_items().await?;

        Ok((prices, filtered))
    }

    pub async fn fetch_prices(&self) -> crate::Result<PriceItems> {
        self.client
            .get("https://api.warframestat.us/wfinfo/prices")
            .send()
            .await?
            .json()
            .await
            .map_err(Into::into)
    }

    pub async fn fetch_filtered_items(&self) -> crate::Result<FilteredItems> {
        self.client
            .get("https://api.warframestat.us/wfinfo/filtered_items")
            .send()
            .await?
            .json()
            .await
            .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_arch_melee_parts_with_prices() {
        let filtered = load_from_str::<FilteredItems>(
            r#"{
                "errors": [],
                "relics": {"Lith": {}, "Neo": {}, "Meso": {}, "Axi": {}},
                "eqmt": {
                    "Test Arch-Melee": {
                        "type": "Arch-Melee",
                        "vaulted": false,
                        "parts": {"Test Arch-Melee Blade": {"ducats": 45}}
                    }
                },
                "ignored_items": {}
            }"#,
        )
        .expect("Arch-Melee equipment should load from the filtered items response");
        let items = Items::new(
            vec![PriceItem {
                name: "Test Arch-Melee Blade".to_owned(),
                custom_avg: 10.0,
            }],
            filtered,
        );

        let part = items
            .find_item("Test Arch-Melee Blade")
            .expect("Arch-Melee parts should be available for lookup");
        assert_eq!(part.platinum, Some(10.0));
        assert_eq!(part.ducats, Some(45));
        assert!(!part.vaulted);
        assert!(!part.ignored);
    }
}
