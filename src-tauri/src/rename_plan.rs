use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    time::{Instant, SystemTime},
};

static COUNTER: AtomicU64 = AtomicU64::new(1);
static PLANS: Mutex<VecDeque<Plan>> = Mutex::new(VecDeque::new());
struct Item {
    path: String,
    target: PathBuf,
    size: u64,
    modified: Option<SystemTime>,
}
struct Plan {
    token: String,
    template: String,
    created: Instant,
    items: Vec<Item>,
    associated: Option<crate::associated_files::Plan>,
}

#[cfg(test)]
pub fn record(paths: &[String], template: &str, targets: Vec<PathBuf>) -> Result<String, String> {
    record_associated(paths, template, targets, None)
}

pub fn record_associated(
    paths: &[String],
    template: &str,
    targets: Vec<PathBuf>,
    associated: Option<crate::associated_files::Plan>,
) -> Result<String, String> {
    if paths.len() != targets.len() {
        return Err("Incomplete rename preview".into());
    }
    let items = paths
        .iter()
        .zip(targets)
        .map(|(path, target)| {
            let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
            Ok(Item {
                path: path.clone(),
                target,
                size: meta.len(),
                modified: meta.modified().ok(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let token = COUNTER.fetch_add(1, Ordering::Relaxed).to_string();
    let mut plans = PLANS.lock().map_err(|_| "Rename plan lock failed")?;
    plans.retain(|plan| plan.created.elapsed().as_secs() < 300);
    while plans.len() >= 32 {
        plans.pop_front();
    }
    plans.push_back(Plan {
        token: token.clone(),
        template: template.into(),
        created: Instant::now(),
        items,
        associated,
    });
    Ok(token)
}
#[cfg(test)]
pub fn consume(
    token: &str,
    paths: &[String],
    template: &str,
) -> Result<Vec<(String, PathBuf)>, String> {
    consume_associated(token, paths, template).map(|(targets, _)| targets)
}
type ReviewedPlan = (
    Vec<(String, PathBuf)>,
    Option<crate::associated_files::Plan>,
);
pub fn consume_associated(
    token: &str,
    paths: &[String],
    template: &str,
) -> Result<ReviewedPlan, String> {
    let mut plans = PLANS.lock().map_err(|_| "Rename plan lock failed")?;
    let index = plans
        .iter()
        .position(|p| p.token == token)
        .ok_or("Rename preview is no longer available; preview again")?;
    let plan = plans.remove(index).ok_or("Rename plan disappeared")?;
    if plan.created.elapsed().as_secs() >= 300
        || plan.template != template
        || plan.items.iter().map(|i| &i.path).ne(paths.iter())
    {
        return Err("The rename request changed; preview again".into());
    }
    for item in &plan.items {
        let meta = std::fs::metadata(&item.path).map_err(|e| e.to_string())?;
        if meta.len() != item.size || meta.modified().ok() != item.modified {
            return Err("A file changed after preview; preview again".into());
        }
    }
    if let Some(associated) = &plan.associated {
        associated.validate()?;
    }
    Ok((
        plan.items.into_iter().map(|i| (i.path, i.target)).collect(),
        plan.associated,
    ))
}

/// Read frozen companion sources for the initial undo capture without consuming
/// the receipt. Apply still consumes and validates it after the capture.
pub fn associated_sources(
    token: &str,
    paths: &[String],
    template: &str,
) -> Result<Vec<String>, String> {
    let plans = PLANS.lock().map_err(|_| "Rename plan lock failed")?;
    let plan = plans
        .iter()
        .find(|p| p.token == token)
        .ok_or("Rename preview is no longer available; preview again")?;
    if plan.created.elapsed().as_secs() >= 300
        || plan.template != template
        || plan.items.iter().map(|i| &i.path).ne(paths.iter())
    {
        return Err("The rename request changed; preview again".into());
    }
    if let Some(associated) = &plan.associated {
        associated.validate()?;
        Ok(associated
            .groups
            .iter()
            .flat_map(|g| g.associated.iter().map(|i| i.source.clone()))
            .collect())
    } else {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reviewed_targets_survive_dynamic_templates_and_can_only_apply_once() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        std::fs::write(&source, "original").unwrap();
        let paths = vec![source.to_string_lossy().into_owned()];
        let target = dir.path().join("chosen-uuid.wav");
        let token = record(&paths, "$uuid()", vec![target.clone()]).unwrap();
        assert_eq!(consume(&token, &paths, "$uuid()").unwrap()[0].1, target);
        assert!(consume(&token, &paths, "$uuid()").is_err());
    }
    #[test]
    fn changed_sources_and_selection_are_rejected_before_renaming() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        std::fs::write(&source, "original").unwrap();
        let paths = vec![source.to_string_lossy().into_owned()];
        let token = record(&paths, "%title%", vec![dir.path().join("target")]).unwrap();
        std::fs::write(&source, "changed-size").unwrap();
        assert!(consume(&token, &paths, "%title%").is_err());
        assert!(!dir.path().join("target").exists());
    }
    #[test]
    fn companion_receipts_reject_changed_selection_and_expiry() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.wav");
        let lyric = dir.path().join("source.lrc");
        std::fs::write(&source, b"audio").unwrap();
        std::fs::write(&lyric, b"lyrics").unwrap();
        let paths = vec![source.to_string_lossy().into_owned()];
        let target = dir.path().join("target.wav");
        let plan = || {
            crate::associated_files::Plan::new(vec![(paths[0].clone(), target.clone())], true)
                .unwrap()
        };
        let token = record_associated(&paths, "new", vec![target.clone()], Some(plan())).unwrap();
        assert_eq!(
            associated_sources(&token, &paths, "new").unwrap(),
            vec![lyric.to_string_lossy().into_owned()]
        );
        assert!(consume_associated(&token, &[], "new").is_err());
        assert!(source.exists() && lyric.exists());
        let token = record_associated(&paths, "new", vec![target.clone()], Some(plan())).unwrap();
        PLANS
            .lock()
            .unwrap()
            .iter_mut()
            .find(|p| p.token == token)
            .unwrap()
            .created -= std::time::Duration::from_secs(301);
        assert!(associated_sources(&token, &paths, "new").is_err());
        assert!(consume_associated(&token, &paths, "new").is_err());
        assert!(source.exists() && lyric.exists());
    }
}
