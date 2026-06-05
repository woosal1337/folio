use anyhow::Result;
use folio_core::storage::scan_recordings;

use crate::cli::SessionsArgs;

pub fn run(args: SessionsArgs) -> Result<()> {
    let mut summaries = scan_recordings(&args.output);
    if args.limit > 0 && summaries.len() > args.limit {
        summaries.truncate(args.limit);
    }

    if args.table {
        if summaries.is_empty() {
            eprintln!("no recordings under {}", args.output.display());
            return Ok(());
        }
        for s in summaries {
            let dur = s.duration_seconds;
            let mic = s.mic_bytes.unwrap_or(0);
            let sys = s.system_bytes.unwrap_or(0);
            println!(
                "{:<30}  {:>4}s  mic={:>10}B  sys={:>10}B  transcript={}",
                s.label,
                dur,
                mic,
                sys,
                if s.has_transcript { "yes" } else { "no" }
            );
        }
        return Ok(());
    }

    for s in summaries {
        let line = serde_json::to_string(&s)?;
        println!("{line}");
    }
    Ok(())
}
