use keyring::{Entry, Error as KeyringError};

const SERVICE_NAME: &str = "NetworkViaShell";

/// Retrieves saved password from Windows Credential Manager / system keyring.
pub fn get_saved_password(profile_name: &str) -> Option<String> {
    let entry = Entry::new(SERVICE_NAME, profile_name).ok()?;
    entry.get_password().ok()
}

/// Securely stores the password in Windows Credential Manager / system keyring.
pub fn save_password(profile_name: &str, password: &str) -> Result<(), String> {
    let entry = Entry::new(SERVICE_NAME, profile_name)
        .map_err(|e| format!("Could not initialize credential entry: {e}"))?;
    entry
        .set_password(password)
        .map_err(|e| format!("Could not save password to keyring: {e}"))
}

/// Deletes the saved credential when "Remember password" is unchecked.
/// Returns Ok(()) if removed or not found, Err if Windows rejects the deletion.
pub fn delete_password(profile_name: &str) -> Result<(), String> {
    let entry = Entry::new(SERVICE_NAME, profile_name)
        .map_err(|e| format!("Could not initialize credential entry: {e}"))?;
    match entry.delete_password() {
        Ok(()) => Ok(()),
        Err(KeyringError::NoEntry) => Ok(()),
        Err(e) => Err(format!("Windows Credential Manager rejected deletion: {e}")),
    }
}
