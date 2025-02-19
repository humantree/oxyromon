#[cfg(feature = "server")]
use async_graphql::{Enum, SimpleObject};
use num_derive::FromPrimitive;
use serde::Deserialize;
use sqlx::{FromRow, Type};
#[cfg(feature = "ird")]
use std::collections::HashMap;

#[derive(Clone, Copy, FromPrimitive, Type, Eq, PartialEq)]
#[cfg_attr(feature = "server", derive(Enum))]
#[repr(i8)]
pub enum Merging {
    Split = 0,
    NonMerged = 1,
    FullNonMerged = 2,
    Merged = 3,
    FullMerged = 4,
}

#[derive(FromRow)]
#[cfg_attr(feature = "server", derive(Clone, SimpleObject))]
#[cfg_attr(feature = "server", graphql(complex))]
pub struct System {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub version: String,
    pub url: Option<String>,
    pub complete: bool,
    pub arcade: bool,
    pub merging: i64,
}

#[cfg_attr(feature = "server", derive(Clone, SimpleObject))]
pub struct Header {
    pub id: i64,
    pub name: String,
    pub version: String,
    pub size: i64,
    pub system_id: i64,
}

#[cfg_attr(feature = "server", derive(Clone, SimpleObject))]
pub struct Rule {
    pub id: i64,
    pub start_byte: i64,
    pub hex_value: String,
    pub header_id: i64,
}

#[derive(FromPrimitive, Type)]
#[cfg_attr(feature = "server", derive(Clone, Copy, Enum, Eq, PartialEq))]
#[repr(i8)]
pub enum Sorting {
    AllRegions = 0,
    OneRegion = 1,
    Ignored = 2,
}

#[derive(FromRow)]
#[cfg_attr(feature = "server", derive(Clone, SimpleObject))]
#[cfg_attr(feature = "server", graphql(complex))]
pub struct Game {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub comment: Option<String>,
    pub external_id: Option<String>,
    pub bios: bool,
    pub jbfolder: bool,
    pub regions: String,
    pub sorting: i64,
    pub complete: bool,
    pub system_id: i64,
    pub parent_id: Option<i64>,
    pub bios_id: Option<i64>,
    pub playlist_id: Option<i64>,
}

#[derive(FromRow)]
#[cfg_attr(feature = "server", derive(Clone, SimpleObject))]
pub struct GameInformation {
    pub title: String,
    pub regions: Vec<String>,
    pub languages: Vec<String>,
    pub release: Option<String>,
    pub flags: Vec<String>,
}

#[derive(FromRow)]
#[cfg_attr(feature = "server", derive(Clone, SimpleObject))]
#[cfg_attr(feature = "server", graphql(complex))]
pub struct Rom {
    pub id: i64,
    pub name: String,
    pub bios: bool,
    pub size: i64,
    pub crc: Option<String>,
    pub md5: Option<String>,
    pub sha1: Option<String>,
    pub rom_status: Option<String>,
    pub game_id: i64,
    pub romfile_id: Option<i64>,
    pub parent_id: Option<i64>,
}

#[derive(FromRow, PartialEq, Eq)]
#[cfg_attr(feature = "server", derive(Clone, SimpleObject))]
pub struct Romfile {
    pub id: i64,
    pub path: String,
    pub size: i64,
}

#[cfg_attr(feature = "server", derive(Clone, SimpleObject))]
pub struct Setting {
    pub id: i64,
    pub key: String,
    pub value: Option<String>,
}

#[derive(Deserialize)]
pub struct ProfileXml {
    #[serde(alias = "datfile")]
    pub systems: Vec<SystemXml>,
}

#[derive(Deserialize)]
pub struct DatfileXml {
    #[serde(alias = "header")]
    pub system: SystemXml,
    #[serde(alias = "game", alias = "machine")]
    pub games: Vec<GameXml>,
}

#[derive(Deserialize)]
pub struct SystemXml {
    pub name: String,
    pub description: String,
    pub version: String,
    #[serde(alias = "clrmamepro", default)]
    pub clrmamepros: Vec<ClrMameProXml>,
    pub url: Option<String>,
}

#[derive(Deserialize)]
pub struct ClrMameProXml {
    #[serde(rename = "@header")]
    pub header: Option<String>,
}

#[derive(Deserialize)]
pub struct GameXml {
    #[serde(rename = "@name")]
    pub name: String,
    pub description: String,
    pub comment: Option<String>,
    #[serde(rename = "@cloneof")]
    pub cloneof: Option<String>,
    #[serde(rename = "@romof")]
    pub romof: Option<String>,
    #[serde(rename = "@isbios")]
    pub isbios: Option<String>,
    #[serde(alias = "rom", default)]
    pub roms: Vec<RomXml>,
}

#[derive(Deserialize)]
pub struct RomXml {
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(rename = "@merge")]
    pub merge: Option<String>,
    #[serde(rename = "@size")]
    pub size: i64,
    #[serde(rename = "@crc")]
    pub crc: Option<String>,
    #[serde(rename = "@md5")]
    pub md5: Option<String>,
    #[serde(rename = "@sha1")]
    pub sha1: Option<String>,
    #[serde(rename = "@status")]
    pub status: Option<String>,
}

#[derive(Deserialize)]
pub struct DetectorXml {
    pub name: String,
    pub version: String,
    pub rule: RuleXml,
}

#[derive(Deserialize)]
pub struct RuleXml {
    #[serde(rename = "@start_offset")]
    pub start_offset: String,
    pub data: Vec<DataXml>,
}

#[derive(Deserialize)]
pub struct DataXml {
    #[serde(rename = "@offset")]
    pub offset: String,
    #[serde(rename = "@value")]
    pub value: String,
}

#[cfg(feature = "ird")]
pub struct Irdfile {
    pub version: u8,
    pub game_id: String,
    pub game_name: String,
    pub update_version: String,
    pub game_version: String,
    pub app_version: String,
    pub regions_count: usize,
    pub regions_hashes: Vec<String>,
    pub files_count: usize,
    pub files_hashes: HashMap<u64, String>,
}
