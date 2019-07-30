mod manifest;
use common::assets::{load_from_path, read_from_assets};
use manifest::encode::{calc_hash, BlockManifest};
use ron::ser::{to_string_pretty, PrettyConfig};
use std::collections::BTreeMap;
use std::fs::File;
use std::prelude::*;

fn main() -> std::io::Result<()> {
    //let mut manifesto = File::create("Manifest.ron")?;
    let files = load_assets("world/tree");
    for (file, entry) in &files {
        let data = BlockManifest {
            id: file.to_string(),
            block_type: "tree".to_string(),
            asset_dir: entry.to_string(),
            map: assets_map(&entry),
            sfx_dir: "null".to_string(),
            hash_val: calc_hash(&file),
        };
        let pretty = PrettyConfig::default();
        let s = to_string_pretty(&data, pretty).expect("Serialization failed");
        println!("{}", s)
        //println!("{:?}", assets_map(&file))
    }
    Ok(())
}

fn load_assets(dir: &str) -> Vec<(String, String)> {
    let target_dir = read_from_assets(dir).expect("cannot find the folder.");

    target_dir
        .filter_map(|entry| {
            entry.ok().map(|f| {
                let path = f.path();
                let file_name = path
                    .file_name()
                    .expect("cannot display filename.")
                    .to_str()
                    .expect("cannot convert &OsStr into &str.")
                    .into();
                let dir_name = (*path.into_os_string().to_string_lossy()).to_owned();
                (file_name, dir_name)
            })
        })
        .collect::<Vec<(String, String)>>()
}

fn assets_map(dir: &str) -> BTreeMap<u8, String> {
    let files = load_assets(dir);
    let mut number = 0;
    let mut assets = BTreeMap::new();
    for (_file, _entry) in files {
        number += 1;
        assets.insert(number, String::from(number.to_string() + ".vox"));
    }
    assets
}
