use super::database::*;
use super::dolphin::{RVZ_BLOCK_SIZE_RANGE, RVZ_COMPRESSION_LEVEL_RANGE};
use super::sevenzip::{SEVENZIP_COMPRESSION_LEVEL_RANGE, ZIP_COMPRESSION_LEVEL_RANGE};
use super::util::*;
use super::SimpleResult;
use async_std::path::{Path, PathBuf};
use cfg_if::cfg_if;
use clap::{Arg, ArgAction, ArgMatches, Command};
use indicatif::ProgressBar;
use phf::phf_map;
use sqlx::sqlite::SqliteConnection;
use std::str::FromStr;
use strum::{Display, EnumString, EnumVariantNames, VariantNames};

cfg_if! {
    if #[cfg(test)] {
        use async_std::sync::Mutex;

        static mut ROM_DIRECTORY: Option<PathBuf> = None;
        static mut TMP_DIRECTORY: Option<PathBuf> = None;

        lazy_static! {
            pub static ref MUTEX: Mutex<i32> = Mutex::new(0);
        }
    } else {
        use once_cell::sync::OnceCell;
        use std::env;

        static ROM_DIRECTORY: OnceCell<PathBuf> = OnceCell::new();
        static TMP_DIRECTORY: OnceCell<PathBuf> = OnceCell::new();
    }
}

#[derive(PartialEq, EnumString, EnumVariantNames)]
#[strum(serialize_all = "lowercase")]
pub enum HashAlgorithm {
    Crc,
    Md5,
    Sha1,
}

#[derive(Display, PartialEq, EnumString, EnumVariantNames)]
#[strum(serialize_all = "lowercase")]
pub enum RvzCompressionAlgorithm {
    None,
    Zstd,
    Bzip,
    Lzma,
    Lzma2,
}

#[derive(PartialEq, EnumString, EnumVariantNames)]
#[strum(serialize_all = "lowercase")]
pub enum SubfolderScheme {
    None,
    Alpha,
}

#[derive(PartialEq, EnumString, EnumVariantNames)]
#[strum(serialize_all = "lowercase")]
pub enum PreferredVersion {
    None,
    New,
    Old,
}

#[derive(PartialEq, EnumString, EnumVariantNames)]
#[strum(serialize_all = "lowercase")]
pub enum PreferredRegion {
    None,
    Broad,
    Narrow,
}

const BOOLEANS: &[&str] = &[
    "GROUP_SUBSYSTEMS",
    "PREFER_PARENTS",
    "REGIONS_ONE_STRICT",
    "SEVENZIP_SOLID_COMPRESSION",
];
const CHOICES: phf::Map<&str, &[&str]> = phf_map! {
    "HASH_ALGORITHM" => HashAlgorithm::VARIANTS,
    "PREFER_REGIONS" => PreferredRegion::VARIANTS,
    "PREFER_VERSIONS" => PreferredVersion::VARIANTS,
    "REGIONS_ALL_SUBFOLDERS" => SubfolderScheme::VARIANTS,
    "REGIONS_ONE_SUBFOLDERS" => SubfolderScheme::VARIANTS,
    "RVZ_COMPRESSION_ALGORITHM" => RvzCompressionAlgorithm::VARIANTS,
};
const INTEGERS: phf::Map<&str, &[usize; 2]> = phf_map! {
    "RVZ_BLOCK_SIZE" => &RVZ_BLOCK_SIZE_RANGE,
    "RVZ_COMPRESSION_LEVEL" => &RVZ_COMPRESSION_LEVEL_RANGE,
    "SEVENZIP_COMPRESSION_LEVEL" => &SEVENZIP_COMPRESSION_LEVEL_RANGE,
    "ZIP_COMPRESSION_LEVEL" => &ZIP_COMPRESSION_LEVEL_RANGE,
};
const LISTS: &[&str] = &[
    "DISCARD_FLAGS",
    "DISCARD_RELEASES",
    "LANGUAGES",
    "PREFER_FLAGS",
    "REGIONS_ALL",
    "REGIONS_ONE",
];
const PATHS: &[&str] = &["ROM_DIRECTORY", "TMP_DIRECTORY"];

const LIST_SEPARATOR: &str = "|";

#[cfg(feature = "chd")]
pub static BIN_EXTENSION: &str = "bin";
pub static CHD_EXTENSION: &str = "chd";
pub static CSO_EXTENSION: &str = "cso";
pub static CUE_EXTENSION: &str = "cue";
pub static ISO_EXTENSION: &str = "iso";
pub static M3U_EXTENSION: &str = "m3u";
pub static NSP_EXTENSION: &str = "nsp";
pub static NSZ_EXTENSION: &str = "nsz";
pub static PKG_EXTENSION: &str = "pkg";
pub static PUP_EXTENSION: &str = "pup";
pub static RAP_EXTENSION: &str = "rap";
pub static RVZ_EXTENSION: &str = "rvz";
pub static SEVENZIP_EXTENSION: &str = "7z";
pub static ZIP_EXTENSION: &str = "zip";

pub static ARCHIVE_EXTENSIONS: [&str; 2] = [SEVENZIP_EXTENSION, ZIP_EXTENSION];
pub static PS3_EXTENSIONS: [&str; 3] = [PKG_EXTENSION, PUP_EXTENSION, RAP_EXTENSION];

#[cfg(feature = "ird")]
pub static PS3_DISC_SFB: &str = "PS3_DISC.SFB";

pub fn subcommand() -> Command {
    Command::new("config")
        .about("Query and modify the oxyromon settings")
        .arg(
            Arg::new("LIST")
                .short('l')
                .long("list")
                .help("Print the whole configuration")
                .required(false)
                .action(ArgAction::SetTrue)
                .exclusive(true),
        )
        .arg(
            Arg::new("GET")
                .short('g')
                .long("get")
                .help("Print a single setting")
                .required(false)
                .num_args(1)
                .value_name("KEY")
                .exclusive(true),
        )
        .arg(
            Arg::new("SET")
                .short('s')
                .long("set")
                .help("Configure a single setting")
                .required(false)
                .num_args(2)
                .value_names(["KEY", "VALUE"])
                .exclusive(true),
        )
        .arg(
            Arg::new("ADD")
                .short('a')
                .long("add")
                .help("Add an entry to a list")
                .required(false)
                .num_args(2)
                .value_names(["KEY", "VALUE"])
                .exclusive(true),
        )
        .arg(
            Arg::new("REMOVE")
                .short('r')
                .long("remove")
                .help("Remove an entry from a list")
                .required(false)
                .num_args(2)
                .value_names(["KEY", "VALUE"])
                .exclusive(true),
        )
}

pub async fn main(
    connection: &mut SqliteConnection,
    matches: &ArgMatches,
    progress_bar: &ProgressBar,
) -> SimpleResult<()> {
    if matches.get_flag("LIST") {
        list_settings(connection).await;
    } else if matches.contains_id("GET") {
        get_setting(connection, matches.get_one::<String>("GET").unwrap()).await;
    } else if matches.contains_id("SET") {
        if let [key, value] = matches
            .get_many::<String>("SET")
            .unwrap()
            .collect::<Vec<_>>()
            .as_slice()
        {
            set_setting(connection, progress_bar, key, value).await?;
        };
    } else if matches.contains_id("ADD") {
        if let [key, value] = matches
            .get_many::<String>("ADD")
            .unwrap()
            .collect::<Vec<_>>()
            .as_slice()
        {
            add_to_list(connection, key, value).await;
        };
    } else if matches.contains_id("REMOVE") {
        if let [key, value] = matches
            .get_many::<String>("REMOVE")
            .unwrap()
            .collect::<Vec<_>>()
            .as_slice()
        {
            remove_from_list(connection, key, value).await;
        };
    }

    Ok(())
}

async fn list_settings(connection: &mut SqliteConnection) {
    for setting in find_settings(connection).await {
        println!("{} = {}", setting.key, setting.value.unwrap_or_default());
    }
}

pub async fn get_setting(connection: &mut SqliteConnection, key: &str) {
    let setting = find_setting_by_key(connection, key).await.unwrap();
    println!("{} = {}", setting.key, setting.value.unwrap_or_default());
}

async fn set_setting(
    connection: &mut SqliteConnection,
    progress_bar: &ProgressBar,
    key: &str,
    value: &str,
) -> SimpleResult<()> {
    if PATHS.contains(&key) {
        let p = get_canonicalized_path(&value.to_owned()).await?;
        create_directory(progress_bar, &p, false).await?;
        set_directory(connection, key, &p).await;
    } else if BOOLEANS.contains(&key) {
        let b: bool = try_with!(FromStr::from_str(value), "Failed to parse bool");
        set_bool(connection, key, b).await;
    } else if CHOICES.keys().any(|&s| s == key) {
        if CHOICES.get(key).unwrap().contains(&value) {
            set_string(connection, key, value).await;
        } else {
            println!("Valid choices: {:?}", CHOICES.get(key).unwrap());
        }
    } else if INTEGERS.keys().any(|&i| i == key) {
        let i: usize = try_with!(FromStr::from_str(value), "Failed to parse integer");
        if INTEGERS.get(key).unwrap()[0] <= i && i <= INTEGERS.get(key).unwrap()[1] {
            set_integer(connection, key, i).await;
        } else {
            println!("Valid range: {:?}", INTEGERS.get(key).unwrap());
        }
    } else if LISTS.contains(&key) {
        println!("Lists can't be set directly, please use ADD or REMOVE instead");
    } else {
        println!("Unsupported setting");
    }
    Ok(())
}

pub async fn get_bool(connection: &mut SqliteConnection, key: &str) -> bool {
    find_setting_by_key(connection, key)
        .await
        .unwrap()
        .value
        .unwrap()
        .parse()
        .unwrap()
}

pub async fn set_bool(connection: &mut SqliteConnection, key: &str, value: bool) {
    let setting = find_setting_by_key(connection, key).await;
    let value = value.to_string();
    match setting {
        Some(setting) => update_setting(connection, setting.id, Some(value)).await,
        None => create_setting(connection, key, Some(value)).await,
    };
}

pub async fn get_integer(connection: &mut SqliteConnection, key: &str) -> usize {
    find_setting_by_key(connection, key)
        .await
        .unwrap()
        .value
        .unwrap()
        .parse()
        .unwrap()
}

async fn set_integer(connection: &mut SqliteConnection, key: &str, value: usize) {
    let setting = find_setting_by_key(connection, key).await;
    let value = value.to_string();
    match setting {
        Some(setting) => update_setting(connection, setting.id, Some(value)).await,
        None => create_setting(connection, key, Some(value)).await,
    };
}

pub async fn get_list(connection: &mut SqliteConnection, key: &str) -> Vec<String> {
    match find_setting_by_key(connection, key).await {
        Some(setting) => match setting.value {
            Some(value) => value.split(LIST_SEPARATOR).map(|s| s.to_owned()).collect(),
            None => Vec::new(),
        },
        None => Vec::new(),
    }
}

pub async fn add_to_list(connection: &mut SqliteConnection, key: &str, value: &str) {
    if LISTS.contains(&key) {
        let mut list = get_list(connection, key).await;
        if !list.contains(&String::from(value)) {
            list.push(value.to_owned());
            if key != "REGIONS_ONE" {
                list.sort();
            }
            set_list(connection, key, &list).await;
        } else {
            println!("Value already in list");
        }
    } else {
        println!("Only list settings are supported");
    }
}

pub async fn remove_from_list(connection: &mut SqliteConnection, key: &str, value: &str) {
    if LISTS.contains(&key) {
        let mut list = get_list(connection, key).await;
        if list.contains(&String::from(value)) {
            list.remove(list.iter().position(|v| v == value).unwrap());
            set_list(connection, key, &list).await;
        } else {
            println!("Value not in list");
        }
    } else {
        println!("Only list settings are supported");
    }
}

async fn set_list(connection: &mut SqliteConnection, key: &str, value: &[String]) {
    let setting = find_setting_by_key(connection, key).await;
    let value = if value.is_empty() {
        None
    } else {
        Some(value.join(LIST_SEPARATOR))
    };
    match setting {
        Some(setting) => update_setting(connection, setting.id, value).await,
        None => create_setting(connection, key, value).await,
    };
}

pub async fn get_directory(connection: &mut SqliteConnection, key: &str) -> Option<PathBuf> {
    match find_setting_by_key(connection, key).await {
        Some(p) => match get_canonicalized_path(&p.value.unwrap()).await {
            Ok(path) => Some(path),
            Err(_) => None,
        },
        None => None,
    }
}

pub async fn set_directory<P: AsRef<Path>>(
    connection: &mut SqliteConnection,
    key: &str,
    value: &P,
) {
    let setting = find_setting_by_key(connection, key).await;
    let value = value.as_ref().as_os_str().to_str().unwrap().to_owned();
    match setting {
        Some(setting) => update_setting(connection, setting.id, Some(value)).await,
        None => create_setting(connection, key, Some(value)).await,
    };
}

pub async fn get_string(connection: &mut SqliteConnection, key: &str) -> String {
    find_setting_by_key(connection, key)
        .await
        .unwrap()
        .value
        .unwrap()
}

pub async fn set_string(connection: &mut SqliteConnection, key: &str, value: &str) {
    let setting = find_setting_by_key(connection, key).await;
    match setting {
        Some(setting) => update_setting(connection, setting.id, Some(value.to_string())).await,
        None => create_setting(connection, key, Some(value.to_string())).await,
    };
}

cfg_if::cfg_if! {
    if #[cfg(test)] {
        pub async fn get_rom_directory(_: &mut SqliteConnection) -> &'static PathBuf {
            unsafe {
                ROM_DIRECTORY.as_ref().unwrap()
            }
        }

        pub fn set_rom_directory(rom_directory: PathBuf) -> &'static PathBuf {
            unsafe {
                ROM_DIRECTORY.replace(rom_directory);
                ROM_DIRECTORY.as_ref().unwrap()
            }
        }

        pub async fn get_tmp_directory(_: &mut SqliteConnection) -> &'static PathBuf {
            unsafe {
                TMP_DIRECTORY.as_ref().unwrap()
            }
        }

        pub fn set_tmp_directory(tmp_directory: PathBuf) -> &'static PathBuf {
            unsafe {
                TMP_DIRECTORY.replace(tmp_directory);
                TMP_DIRECTORY.as_ref().unwrap()
            }
        }
    } else {
        pub async fn get_rom_directory(connection: &mut SqliteConnection) -> &'static PathBuf {
            match ROM_DIRECTORY.get() {
                Some(rom_directory) => rom_directory,
                None => {
                    let rom_directory = match get_directory(connection, "ROM_DIRECTORY").await {
                        Some(rom_directory) => rom_directory,
                        None => {
                            let rom_directory = match env::var("OXYROMON_ROM_DIRECTORY") {
                                Ok(rom_directory) => PathBuf::from(rom_directory),
                                Err(_) => dirs::home_dir().map(PathBuf::from).unwrap().join("Emulation")
                            };
                            set_directory(connection, "ROM_DIRECTORY", &rom_directory).await;
                            rom_directory
                        }
                    };
                    ROM_DIRECTORY
                        .set(rom_directory)
                        .expect("Failed to set rom directory");
                    ROM_DIRECTORY.get().unwrap()
                }
            }
        }

        pub async fn get_tmp_directory(connection: &mut SqliteConnection) -> &'static PathBuf {
            match TMP_DIRECTORY.get() {
                Some(tmp_directory) => tmp_directory,
                None => {
                    let tmp_directory = match get_directory(connection, "TMP_DIRECTORY").await {
                        Some(tmp_directory) => tmp_directory,
                        None => {
                            let tmp_directory = match env::var("OXYROMON_TMP_DIRECTORY") {
                                Ok(tmp_directory) => PathBuf::from(tmp_directory),
                                Err(_) => PathBuf::from(env::temp_dir())
                            };
                            set_directory(connection, "TMP_DIRECTORY", &tmp_directory).await;
                            tmp_directory
                        }
                    };
                    TMP_DIRECTORY
                        .set(tmp_directory)
                        .expect("Failed to set tmp directory");
                    TMP_DIRECTORY.get().unwrap()
                }
            }
        }
    }
}

#[cfg(test)]
mod test_add_to_list;
#[cfg(test)]
mod test_add_to_list_already_exists;
#[cfg(test)]
mod test_bool;
#[cfg(test)]
mod test_directory;
#[cfg(test)]
mod test_list;
#[cfg(test)]
mod test_remove_from_list;
#[cfg(test)]
mod test_remove_from_list_does_not_exist;
#[cfg(test)]
mod test_set_new_directory_when_old_is_missing;
