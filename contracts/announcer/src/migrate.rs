use cosmwasm_std::{StdResult, StdError, Storage};
use cw2::get_contract_version;
use semver::Version;

fn from_semver(err: semver::Error) -> StdError {
    StdError::generic_err(format!("Semver: {}", err))
}

pub fn set_contract_version(storage: &mut dyn Storage, name: &str, version: &str) -> StdResult<()> {
    cw2::set_contract_version(storage, name, version)
}

pub fn ensure_from_older_version(
    storage: &mut dyn Storage,
    name: &str,
    new_version: &str,
) -> StdResult<Version> {
    let version: Version = new_version.parse().map_err(from_semver)?;
    let stored = get_contract_version(storage)?;
    let storage_version: Version = stored.version.parse().map_err(from_semver)?;

    if name != stored.contract {
        let msg = format!("Cannot migrate from {} to {}", stored.contract, name);
        return Err(StdError::generic_err(msg));
    }

    if storage_version > version {
        let msg = format!(
            "Cannot migrate from newer version ({}) to older ({})",
            stored.version, new_version
        );
        return Err(StdError::generic_err(msg));
    }
    if storage_version < version {
        // we don't need to save anything if migrating from the same version
        set_contract_version(storage, name, new_version)?;
    }

    Ok(storage_version)
}