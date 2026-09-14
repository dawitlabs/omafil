use crate::error::DirectoryError;
use serde::Serialize;
use trash::os_limited::{list, purge_all, restore_all};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecycleItem {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) original_path: String,
    pub(crate) deleted_at: i64,
}

pub(crate) fn list_recycle_bin() -> Result<Vec<RecycleItem>, DirectoryError> {
    let mut items = list()
        .map_err(|_| DirectoryError::unavailable())?
        .into_iter()
        .map(|item| RecycleItem {
            id: item.id.to_string_lossy().into_owned(),
            name: item.name.to_string_lossy().into_owned(),
            original_path: item.original_path().to_string_lossy().into_owned(),
            deleted_at: item.time_deleted,
        })
        .collect::<Vec<_>>();

    items.sort_by(|left, right| right.deleted_at.cmp(&left.deleted_at));
    Ok(items)
}

/// The ids currently in the trash, used to tell apart what a delete just added.
pub(crate) fn recycle_item_ids() -> Result<std::collections::HashSet<String>, DirectoryError> {
    Ok(list()
        .map_err(|_| DirectoryError::unavailable())?
        .into_iter()
        .map(|item| item.id.to_string_lossy().into_owned())
        .collect())
}

fn selected(ids: &[String]) -> Result<Vec<trash::TrashItem>, DirectoryError> {
    let requested = ids.iter().collect::<std::collections::HashSet<_>>();
    let items = list()
        .map_err(|_| DirectoryError::unavailable())?
        .into_iter()
        .filter(|item| requested.contains(&item.id.to_string_lossy().into_owned()))
        .collect::<Vec<_>>();

    (items.len() == ids.len())
        .then_some(items)
        .ok_or_else(DirectoryError::unavailable)
}

pub(crate) fn restore_recycle_items(ids: Vec<String>) -> Result<(), DirectoryError> {
    restore_all(selected(&ids)?).map_err(|_| DirectoryError::operation_failed())
}

pub(crate) fn empty_recycle_bin() -> Result<(), DirectoryError> {
    let items = list().map_err(|_| DirectoryError::unavailable())?;
    purge_all(items).map_err(|_| DirectoryError::operation_failed())
}
