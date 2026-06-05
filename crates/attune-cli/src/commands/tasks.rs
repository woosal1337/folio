use anyhow::Result;
use attune_core::storage::TaskStore;

use crate::cli::TasksArgs;

pub fn run(args: TasksArgs) -> Result<()> {
    let store = TaskStore::new(args.path.clone());
    let tasks = store.list();

    let normalised = args.status.as_deref().map(str::trim).map(str::to_lowercase);
    let filtered: Vec<_> = tasks
        .into_iter()
        .filter(|t| match &normalised {
            None => true,
            Some(s) if s.is_empty() => true,
            Some(s) => serde_json::to_string(&t.status)
                .map(|v| v.trim_matches('"').eq_ignore_ascii_case(s))
                .unwrap_or(false),
        })
        .collect();

    if args.table {
        if filtered.is_empty() {
            eprintln!("no tasks");
            return Ok(());
        }
        for t in filtered {
            let owner = t.owner.as_deref().unwrap_or("-");
            let due = t.due.as_deref().unwrap_or("-");
            let status = serde_json::to_string(&t.status)
                .map(|s| s.trim_matches('"').to_string())
                .unwrap_or_else(|_| "?".to_string());
            println!("{:<6}  {:<14}  {:<10}  {}", status, owner, due, t.title);
        }
        return Ok(());
    }

    for t in filtered {
        println!("{}", serde_json::to_string(&t)?);
    }
    Ok(())
}
