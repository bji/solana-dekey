mod usage;

use serde::Deserialize;
use std::io::BufRead;

const DEFAULT_MAINNET_RPC_URL : &str = "https://api.mainnet-beta.solana.com";
const DEFAULT_TESTNET_RPC_URL : &str = "https://api.testnet.solana.com";
const DEFAULT_DEVNET_RPC_URL : &str = "https://api.devnet.solana.com";
const DEFAULT_LOCALHOST_RPC_URL : &str = "http://localhost:8899";

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct VoteAccountDetails
{
    votePubkey : String,
    nodePubkey : String
}

enum Fit
{
    Key,
    Max
}

enum Duplicate
{
    Prefix,
    Suffix
}

fn error_exit(msg : String) -> !
{
    eprintln!("{}", msg);
    std::process::exit(-1);
}

fn jv(
    mut v : serde_json::Value,
    path : &str
) -> Result<serde_json::Value, String>
{
    for s in path.split(".") {
        v = match v {
            serde_json::Value::Object(m) => {
                m.get(s).ok_or(format!("Invalid response json, missing field {}", s))?.clone()
            },
            _ => return Err("Invalid response json, expected object".to_string())
        };
    }

    Ok(v)
}

fn decode_base58(
    r : &[u8],
    index : usize,
    len : usize
) -> Option<String>
{
    if (index + len) > r.len() {
        return None;
    }

    Some(bs58::encode(&r[index..(index + len)]).into_string())
}

// Returns validator identity pubkey and name
fn decode_validator_info(r : &[u8]) -> Option<(String, String)>
{
    // Read ShortU16 which is the number of { pubkey, is_signed } tuples
    if r.len() < 1 {
        return None;
    }

    let (count, mut index) = if r[0] > 0x7F {
        if r.len() < 2 {
            return None;
        }
        if r[1] > 0x7F {
            if r.len() < 3 {
                return None;
            }
            (((r[0] as u16) << 0) | ((r[1] as u16) << 7) | ((r[2] as u16) << 14), 3)
        }
        else {
            (((r[0] as u16) << 0) | ((r[1] as u16) << 7), 2)
        }
    }
    else {
        (r[0] as u16, 1)
    };

    if count < 2 {
        return None;
    }

    let program_pubkey = decode_base58(r, index, 32);
    if program_pubkey.is_none() {
        return None;
    }
    let program_pubkey = program_pubkey.unwrap();

    if program_pubkey != "Va1idator1nfo111111111111111111111111111111" {
        return None;
    }

    // Skip pubkey and boolean
    index += 33;

    let validator_pubkey = decode_base58(r, index, 32);
    if validator_pubkey.is_none() {
        return None;
    }
    let validator_pubkey = validator_pubkey.unwrap();

    // Skip pubkey and boolean and remaining keys and booleans, if any
    // Skip remaining keys and booleans, if any
    index += ((count as usize) - 1) * 33;

    // Read string length
    let length = bincode::deserialize::<u64>(&r[index..(index + 8)]).ok();
    if length.is_none() {
        return None;
    }
    let length = length.unwrap() as usize;

    // Skip u64
    index += 8;

    // Parse string as json
    match serde_json::from_slice(&r[index..(index + length)]) {
        Ok(serde_json::Value::Object(o)) => match o.get("name") {
            Some(serde_json::Value::String(name)) => Some((validator_pubkey, name.to_string())),
            _ => None
        },
        _ => None
    }
}

fn has_printable(s : &str) -> bool
{
    for c in s.chars() {
        if !char::is_whitespace(c) {
            return true;
        }
    }

    false
}

fn display_name(
    name : &str,
    ascii_only : bool,
    fit : &Option<Fit>,
    duplicate : &Option<Duplicate>,
    key : &str
) -> String
{
    let mut name =
        if ascii_only { name.chars().filter(|c| c.is_ascii()).collect::<String>() } else { name.to_string() };

    match fit {
        Some(Fit::Key) => {
            let mut v = name.chars().collect::<Vec<char>>();
            v.resize(key.len(), ' ');
            name = v.iter().collect::<String>()
        },
        Some(Fit::Max) => {
            let mut v = name.chars().collect::<Vec<char>>();
            v.resize(44, ' ');
            name = v.iter().collect::<String>()
        },
        None => ()
    }

    if name.len() == 0 || !has_printable(&name) {
        name = key.to_string()
    }

    match duplicate {
        Some(Duplicate::Prefix) => format!("{} ({})", name, key),
        Some(Duplicate::Suffix) => format!("{} ({})", key, name),
        None => name
    }
}

fn create_cache(
    rpc_url : &str,
    cache_file : &std::path::Path
) -> Result<(std::collections::HashMap<String, String>, std::collections::HashMap<String, String>), String>
{
    // Read vote account to validator id mapping
    let mut vote_to_validator_id = std::collections::HashMap::<String, String>::new();

    let resp = ureq::post(rpc_url)
        .set("Content-Type", "application/json")
        .send_string(&format!(
            "{}",
            serde_json::json!({
                "jsonrpc" : "2.0",
                "id" : 1,
                "method" : "getVoteAccounts",
                "params" : [ { "delinquentSlotDistance" : u64::MAX } ]
            })
        ))
        .unwrap_or_else(|e| error_exit(format!("ERROR: Failed to load vote accounts from {}: {}", rpc_url, e)));

    match jv(serde_json::from_reader(resp.into_reader()).map_err(|e| format!("{}", e))?, "result.current")? {
        serde_json::Value::Array(v) => {
            v.into_iter().for_each(|e| {
                //let  : Result<VoteAccountDetails, serde_json::Error> = serde_json::from_str(&e.to_string());
                //                match details {
                //                    Ok(mapping) => {
                //                    }
                match serde_json::from_str::<VoteAccountDetails>(&e.to_string()) {
                    Ok(details) => {
                        vote_to_validator_id.insert(details.votePubkey, details.nodePubkey);
                        ()
                    },
                    Err(e) => error_exit(format!("ERROR: Failed to load vote accounts from {}: {}", rpc_url, e))
                }
            });
        },
        _ => error_exit(format!("ERROR: Failed to parse vote accounts response from {}", rpc_url))
    }

    // Read validator id to name mapping
    let mut validator_id_to_name = std::collections::HashMap::<String, String>::new();

    let resp = ureq::post(rpc_url)
        .set("Content-Type", "application/json")
        .send_string(&format!(
            "{}",
            serde_json::json!({
                "jsonrpc" : "2.0",
                "id" : 1,
                "method" : "getProgramAccounts",
                "params" : [ "Config1111111111111111111111111111111111111", { "encoding" : "base64" } ]
            })
        ))
        .unwrap_or_else(|e| {
            error_exit(format!("ERROR: Failed to load validator info accounts from {}: {}", rpc_url, e))
        });

    match jv(serde_json::from_reader(resp.into_reader()).map_err(|e| format!("{}", e))?, "result")? {
        serde_json::Value::Array(v) => v.into_iter().for_each(|e| match jv(e, "account.data") {
            Ok(serde_json::Value::Array(v)) => match v.get(0) {
                Some(serde_json::Value::String(base64)) => {
                    base64::decode(base64)
                        .ok()
                        .and_then(|r| decode_validator_info(&r))
                        .and_then(|(validator_pubkey, name)| validator_id_to_name.insert(validator_pubkey, name));
                    ()
                },
                Some(_) | None => ()
            },
            _ => ()
        }),
        _ => {
            error_exit(format!("ERROR: Failed to parse validator info accounts response from {}", rpc_url));
        }
    }

    // Serialize the maps out to the cache file
    let maps = (vote_to_validator_id, validator_id_to_name);

    bincode::serialize_into(&std::fs::File::create(cache_file).map_err(|e| e.to_string())?, &maps)
        .map_err(|e| e.to_string())?;

    Ok(maps)
}

fn main()
{
    let mut rpc_url = DEFAULT_MAINNET_RPC_URL.to_string();
    let mut cache_file = None;
    let mut ascii_only = false;
    let mut fit = None;
    let mut duplicate = None;
    let mut dekey_identity = true;
    let mut dekey_vote = true;
    let mut delete_cache = false;
    let mut lookup_re = None;

    let mut args = std::env::args();
    args.nth(0);

    while let Some(arg) = args.nth(0) {
        match arg.as_str() {
            "--help" | "help" => {
                println!("{}", usage::USAGE_MESSAGE);
                return;
            },
            "-u" | "--url" => {
                rpc_url = match args.nth(0) {
                    None => error_exit(format!("ERROR: {} requires an argument", arg)),
                    Some(a) => match a.as_str() {
                        "l" | "localhost" => DEFAULT_LOCALHOST_RPC_URL.to_string(),
                        "d" | "devnet" => DEFAULT_DEVNET_RPC_URL.to_string(),
                        "t" | "testnet" => DEFAULT_TESTNET_RPC_URL.to_string(),
                        "m" | "mainnet" => DEFAULT_MAINNET_RPC_URL.to_string(),
                        url => url.to_string()
                    }
                }
            },
            "-c" | "--cache_file" => match args.nth(0) {
                None => error_exit(format!("ERROR: {} requires an argument", arg)),
                Some(a) => cache_file = Some(a.to_string())
            },
            "-a" | "--ascii" => ascii_only = true,
            "-f" | "--fit" => fit = Some(Fit::Key),
            "-m" | "--max" => fit = Some(Fit::Max),
            "-p" | "--prefix" => duplicate = Some(Duplicate::Prefix),
            "-s" | "--suffix" => duplicate = Some(Duplicate::Suffix),
            "-i" | "--identity" => dekey_vote = false,
            "-v" | "--vote" => dekey_identity = false,
            "-d" | "--delete_cache" => delete_cache = true,
            "-l" | "--lookup" => match args.nth(0) {
                None => error_exit(format!("ERROR: {} requires an argument", arg)),
                Some(a) => {
                    lookup_re = Some(
                        regex::Regex::new(&a)
                            .unwrap_or_else(|e| error_exit(format!("ERROR: Invalid regular expression {}: {}", a, e)))
                    )
                },
            },
            _ => error_exit(format!("ERROR: Unknown command: {}", arg))
        }
    }

    // Determine cache file location
    if cache_file.is_none() {
        cache_file = dirs::home_dir()
            .unwrap_or_else(|| error_exit("ERROR: Could not determine home directory".to_string()))
            .join(".solana-dekey-cache")
            .to_str()
            .map(|s| s.to_string());
    }

    let cache_file = cache_file.unwrap();

    let cache_file_path = std::path::Path::new(&cache_file);

    // Delete the cache if instructed to do so
    if delete_cache {
        std::fs::remove_file(&cache_file_path).unwrap_or_else(|e| {
            error_exit(format!("ERROR: Could not delete cache file {}: {}", cache_file_path.display(), e))
        });
    }

    // If the cache file exists, read it; else create it
    let (vote_to_validator_id, validator_id_to_name) = match std::fs::File::open(&cache_file_path) {
        Ok(file) => {
            let result : (std::collections::HashMap<String, String>, std::collections::HashMap<String, String>) =
                bincode::deserialize_from(&file).unwrap_or_else(|e| {
                    error_exit(format!("ERROR: Could not read cache file {}: {}", cache_file_path.display(), e))
                });
            result
        },
        Err(_) => create_cache(&rpc_url, &cache_file_path).unwrap()
    };

    // If looking up names, just do a name lookup, don't do dekeying
    match lookup_re {
        Some(lookup_re) => {
            // Invert the vote to validator id map, meanwhile checking to see if the regexp matches a vote
            // account or validator id
            // found is Option<(Option<vote_account>, Option<validator_id>)>
            let mut found_validator_ids = std::collections::HashSet::<String>::new();
            let mut validator_id_to_vote = std::collections::HashMap::<String, String>::new();
            for (key, value) in vote_to_validator_id {
                if lookup_re.is_match(&key) || lookup_re.is_match(&value) {
                    found_validator_ids.insert(value.clone());
                }
                validator_id_to_vote.insert(value, key);
            }
            // Check every name and validator id in the validator_id_to_name map
            for (key, value) in &validator_id_to_name {
                if lookup_re.is_match(&key) || lookup_re.is_match(&value) {
                    found_validator_ids.insert(key.clone());
                }
            }
            for validator in found_validator_ids {
                let vote_account = validator_id_to_vote.get(&validator);
                let name = match validator_id_to_name.get(&validator) {
                    Some(name) => name.clone(),
                    None => vote_account.unwrap_or(&validator).clone()
                };
                println!("{}", display_name(&name, ascii_only, &fit, &duplicate, &validator));
                println!("    Validator Identity: {}", validator);
                match vote_account {
                    Some(v) => println!("          Vote Account: {}", v),
                    None => ()
                }
            }
            return;
        },
        None => ()
    }

    // Now build up the regexps which will be used to replace in each line
    let mut regexps = vec![];

    if dekey_vote {
        for (vote_pubkey, validator_id) in vote_to_validator_id {
            if let Some(name) = validator_id_to_name.get(&validator_id) {
                regexps.push((
                    regex::Regex::new(&vote_pubkey).unwrap(),
                    display_name(&name, ascii_only, &fit, &duplicate, &vote_pubkey)
                ));
            }
        }
    }

    if dekey_identity {
        for (validator_id, name) in validator_id_to_name {
            regexps.push((
                regex::Regex::new(&validator_id).unwrap(),
                display_name(&name, ascii_only, &fit, &duplicate, &validator_id)
            ));
        }
    }

    // And do the replace of stdin to stdout, line by line
    std::io::stdin().lock().lines().for_each(|l| match l {
        Ok(mut l) => {
            for (re, replacement) in &regexps {
                l = re.replace_all(&l, replacement).to_string();
            }
            println!("{}", l);
        },
        _ => ()
    });
}
