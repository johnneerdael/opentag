//! Durable provider-backed working copies. Call under FILE_OPERATIONS, on a
//! blocking worker. Mutation closures must perform pure tag work only: reentering
//! these functions from a closure would try to lock the store recursively.
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use tag_core::{AudioEntry, Fields, ScanResult};
#[cfg(any(target_os = "android", test))]
mod engine;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DocumentState {
    pub uri: String,
    pub name: String,
    pub writable: bool,
    pub persisted: bool,
    pub state: String,
    pub error: Option<String>,
}
#[cfg(target_os = "android")]
mod android {
    use super::*;
    use std::sync::{Mutex, OnceLock};
    use tauri_plugin_document_access::{DocumentInfo, DocumentSnapshot, DocumentsExt};
    pub(super) struct Bridge(tauri::AppHandle);
    impl engine::Provider for Bridge {
        fn snapshot(&self, uri: &str, output: &Path) -> Result<DocumentSnapshot, String> {
            self.0.documents().snapshot(uri, output)
        }
        fn publish(
            &self,
            uri: &str,
            input: &Path,
            expected: &str,
        ) -> Result<DocumentSnapshot, String> {
            self.0.documents().publish(uri, input, expected)
        }
        fn create(&self, name: &str, mime: Option<&str>) -> Result<Option<DocumentInfo>, String> {
            self.0.documents().create_document(name, mime)
        }
    }
    static STORE: OnceLock<Mutex<engine::Store<Bridge>>> = OnceLock::new();
    pub(super) fn initialize(app: &tauri::AppHandle) -> Result<(), String> {
        if STORE.get().is_none() {
            // Tauri app_data_dir is Android dataDir, not Context.filesDir.
            // Resolve the actual allowed persistent root through the bridge.
            let root = app.documents().storage_root()?.join("documents-v1");
            let store =
                engine::Store::open(&root, Bridge(app.clone()), crate::settings::tag_options)?;
            STORE
                .set(Mutex::new(store))
                .map_err(|_| "Document store initialized concurrently")?;
        }
        Ok(())
    }
    pub(super) fn with_store<T>(
        work: impl FnOnce(&mut engine::Store<Bridge>) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut store = STORE
            .get()
            .ok_or("Document store is not initialized")?
            .lock()
            .map_err(|_| "Document store lock failed")?;
        work(&mut store)
    }
    pub(super) fn pick(app: &tauri::AppHandle) -> Result<ScanResult, String> {
        let documents = app.documents().pick_documents(true)?;
        with_store(|store| {
            let mut result = ScanResult::default();
            for document in documents {
                let uri = document.uri.clone();
                match store.import(document) {
                    Ok(entry) => result.entries.push(entry),
                    Err(error) => result.errors.push(tag_core::FileError { path: uri, error }),
                }
            }
            Ok(result)
        })
    }
}

pub fn initialize(app: &tauri::AppHandle) -> Result<(), String> {
    #[cfg(target_os = "android")]
    return android::initialize(app);
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Ok(())
    }
}
pub fn pick(app: &tauri::AppHandle) -> Result<ScanResult, String> {
    initialize(app)?;
    #[cfg(target_os = "android")]
    return android::pick(app);
    #[cfg(not(target_os = "android"))]
    Err("Provider document selection is supported only on Android".into())
}
pub fn states() -> Result<BTreeMap<String, DocumentState>, String> {
    #[cfg(test)]
    {
        let stores = command_fixture::STORES.lock().unwrap();
        Ok(stores.iter().flat_map(|s| s.states()).collect())
    }
    #[cfg(all(target_os = "android", not(test)))]
    return android::with_store(|store| Ok(store.states()));
    #[cfg(not(any(target_os = "android", test)))]
    Ok(BTreeMap::new())
}
pub fn restore_recovery() -> Result<ScanResult, String> {
    #[cfg(test)]
    {
        let mut result = ScanResult::default();
        for store in command_fixture::STORES.lock().unwrap().iter_mut() {
            let recovered = store.restore_recovery();
            result.entries.extend(recovered.entries);
            result.errors.extend(recovered.errors);
        }
        Ok(result)
    }
    #[cfg(all(target_os = "android", not(test)))]
    return android::with_store(|store| Ok(store.restore_recovery()));
    #[cfg(not(any(target_os = "android", test)))]
    Ok(ScanResult::default())
}
pub fn is_managed(path: &Path) -> bool {
    #[cfg(test)]
    if command_fixture::STORES
        .lock()
        .unwrap()
        .iter()
        .any(|s| s.is_managed(path))
    {
        return true;
    }
    #[cfg(target_os = "android")]
    return android::with_store(|store| Ok(store.is_managed(path))).unwrap_or(false);
    #[cfg(not(target_os = "android"))]
    {
        let _ = path;
        false
    }
}
pub fn mutate(
    path: &Path,
    work: impl FnOnce(&Path) -> Result<AudioEntry, String>,
) -> Result<AudioEntry, String> {
    #[cfg(test)]
    if is_managed(path) {
        return command_fixture::STORES
            .lock()
            .unwrap()
            .iter_mut()
            .find(|s| s.is_managed(path))
            .unwrap()
            .mutate(path, work);
    }
    #[cfg(target_os = "android")]
    return android::with_store(|store| store.mutate(path, work));
    #[cfg(not(target_os = "android"))]
    work(path)
}
pub fn read_working(path: &Path) -> Result<AudioEntry, String> {
    #[cfg(test)]
    {
        return command_fixture::STORES
            .lock()
            .unwrap()
            .iter()
            .find(|store| store.is_managed(path))
            .ok_or_else(|| "Unknown provider document".to_string())?
            .read_working(path);
    }
    #[cfg(all(target_os = "android", not(test)))]
    return android::with_store(|store| store.read_working(path));
    #[cfg(not(any(target_os = "android", test)))]
    {
        let _ = path;
        Err("Unknown provider document".into())
    }
}
pub fn reload(path: &Path) -> Result<AudioEntry, String> {
    #[cfg(test)]
    if is_managed(path) {
        return command_fixture::STORES
            .lock()
            .unwrap()
            .iter_mut()
            .find(|s| s.is_managed(path))
            .unwrap()
            .reload(path);
    }
    #[cfg(target_os = "android")]
    return android::with_store(|store| store.reload(path));
    #[cfg(not(target_os = "android"))]
    tag_core::read_entry(path)
}
pub fn project(entry: &mut AudioEntry) {
    #[cfg(test)]
    for store in command_fixture::STORES.lock().unwrap().iter() {
        store.project(entry);
    }
    #[cfg(target_os = "android")]
    {
        let _ = android::with_store(|store| {
            store.project(entry);
            Ok(())
        });
    }
    #[cfg(not(target_os = "android"))]
    let _ = entry;
}
pub fn snapshot_input(app: &tauri::AppHandle, path: &str) -> Result<PathBuf, String> {
    initialize(app)?;
    #[cfg(target_os = "android")]
    return android::with_store(|store| store.snapshot_input(path));
    #[cfg(not(target_os = "android"))]
    Ok(PathBuf::from(path))
}
pub(crate) fn snapshot_input_with_name(
    app: &tauri::AppHandle,
    path: &str,
) -> Result<(PathBuf, Option<String>), String> {
    initialize(app)?;
    #[cfg(target_os = "android")]
    return android::with_store(|store| {
        store
            .snapshot_input_with_name(path)
            .map(|(path, name)| (path, Some(name)))
    });
    #[cfg(not(target_os = "android"))]
    Ok((PathBuf::from(path), None))
}
pub fn save_copy(
    app: &tauri::AppHandle,
    path: &str,
    updates: &Fields,
    version: &str,
) -> Result<ScanResult, String> {
    initialize(app)?;
    #[cfg(target_os = "android")]
    return android::with_store(|store| store.save_copy(Path::new(path), updates, version));
    #[cfg(not(target_os = "android"))]
    {
        let _ = (path, updates, version);
        Err("Provider Save a copy is supported only on Android".into())
    }
}

#[cfg(test)]
pub(crate) mod command_fixture;
