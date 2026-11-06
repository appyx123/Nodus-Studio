use crate::engine::{CircuitEdit, CircuitSnapshot, CircuitState};
use crate::sandbox::pkg_loader::RegistryCatalog;
use crate::NodusError;
use std::sync::{Arc, Mutex, MutexGuard};

pub type ManagedCircuit = Arc<Mutex<CircuitState>>;

fn lock(state: &ManagedCircuit) -> Result<MutexGuard<'_, CircuitState>, NodusError> {
    state
        .lock()
        .map_err(|_| NodusError::Circuit("Circuit state lock is poisoned".into()))
}

pub fn snapshot(state: &ManagedCircuit) -> Result<CircuitSnapshot, NodusError> {
    Ok(lock(state)?.snapshot())
}

pub fn edit(state: &ManagedCircuit, edit: CircuitEdit) -> Result<CircuitSnapshot, NodusError> {
    lock(state)?.edit(edit)
}

pub fn register_scan(
    state: &ManagedCircuit,
    result: Result<RegistryCatalog, NodusError>,
) -> Result<RegistryCatalog, NodusError> {
    let catalog = result?;
    lock(state)?.register_catalog(&catalog);
    Ok(catalog)
}

#[cfg(test)]
#[path = "circuit_tests.rs"]
mod tests;
