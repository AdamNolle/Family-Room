use family_core::FamilyCore;
use std::io::{self, BufRead, Write};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::args()
        .nth(1)
        .ok_or("usage: family-core <vault-directory>; set FAMILY_ROOM_KEY securely")?;
    let key = std::env::var("FAMILY_ROOM_KEY")
        .map_err(|_| "Set FAMILY_ROOM_KEY to your recovery key; never pass it in argv")?;
    let core = FamilyCore::open(root, key)?;
    for line in io::stdin().lock().lines() {
        let response = match core.command(line?) {
            Ok(result) => {
                serde_json::json!({"ok":true,"result":serde_json::from_str::<serde_json::Value>(&result)?})
            }
            Err(e) => serde_json::json!({"ok":false,"error":e.to_string()}),
        };
        writeln!(io::stdout().lock(), "{response}")?;
    }
    Ok(())
}
