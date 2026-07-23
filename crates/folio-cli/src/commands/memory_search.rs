use anyhow::Result;
use folio_core::memory::page::read_dir_pages;
use folio_core::memory::types::MemoryKind;

use crate::cli::MemorySearchArgs;

pub fn run(args: MemorySearchArgs) -> Result<()> {
    let pages = read_dir_pages(&args.dir);
    let needle = args.query.to_lowercase();
    let kind_filter = args
        .kind
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(MemoryKind::parse);

    let mut matches: Vec<_> = pages
        .into_iter()
        .filter(|m| match kind_filter {
            Some(k) => m.kind == k,
            None => true,
        })
        .filter(|m| {
            if needle.is_empty() {
                return true;
            }
            let hay_content = m.content.to_lowercase();
            let hay_key = m.key.as_deref().unwrap_or("").to_lowercase();
            hay_content.contains(&needle) || hay_key.contains(&needle)
        })
        .collect();

    matches.sort_by_key(|m| std::cmp::Reverse(m.updated_at));
    if args.limit > 0 && matches.len() > args.limit {
        matches.truncate(args.limit);
    }

    if args.table {
        if matches.is_empty() {
            eprintln!("no matches under {}", args.dir.display());
            return Ok(());
        }
        for m in matches {
            let kind = m.kind.as_str();
            let key = m.key.as_deref().unwrap_or("-");
            println!(
                "{:<7}  {:<24}  {}",
                kind,
                truncate(key, 24),
                truncate(&m.content, 100)
            );
        }
        return Ok(());
    }

    for m in matches {
        println!("{}", serde_json::to_string(&m)?);
    }
    Ok(())
}

fn truncate(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(n.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}
