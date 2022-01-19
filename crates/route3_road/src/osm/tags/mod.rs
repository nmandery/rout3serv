pub mod maxspeed;
pub mod sidewalk;

/// the string should be in lowercase
fn str_to_bool(instr: &str) -> Option<bool> {
    match instr {
        "y" | "yes" | "true" => Some(true),
        "n" | "no" | "false" => Some(false),
        _ => None,
    }
}
