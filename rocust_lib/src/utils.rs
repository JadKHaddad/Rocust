use std::{
    ffi::OsStr,
    path::Path,
    time::{SystemTime, SystemTimeError, UNIX_EPOCH},
};

pub(crate) fn get_timestamp_as_millis() -> Result<u128, SystemTimeError> {
    let now = SystemTime::now();
    let since_the_epoch = now.duration_since(UNIX_EPOCH)?;
    Ok(since_the_epoch.as_millis())
}

pub(crate) fn get_timestamp_as_millis_as_string() -> Result<String, SystemTimeError> {
    Ok(get_timestamp_as_millis()?.to_string())
}

pub(crate) fn get_extension_from_filename(filename: &Path) -> Option<&str> {
    filename.extension().and_then(OsStr::to_str)
}
