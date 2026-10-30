//! Keyboard shortcut representation and string conversion.
//!
//! Ported and enhanced from:
//! - `PinWin/BusinessLayer/KeysStringConverter.cs`
//! - `PinWin/BusinessLayer/HotKey.cs`
//! - `PinWin/WinApi/ApiHotKey.cs`

use std::fmt;

/// Win32 RegisterHotKey modifier flags.
pub const MOD_ALT: u32 = 0x0001;
pub const MOD_CONTROL: u32 = 0x0002;
pub const MOD_SHIFT: u32 = 0x0004;
pub const MOD_WIN: u32 = 0x0008;
pub const MOD_NOREPEAT: u32 = 0x4000;

/// Win32 Virtual-Key codes commonly used in shortcuts.
pub const VK_NONE: u32 = 0x00;
pub const VK_BACK: u32 = 0x08;
pub const VK_TAB: u32 = 0x09;
pub const VK_RETURN: u32 = 0x0D;
pub const VK_SHIFT: u32 = 0x10;
pub const VK_CONTROL: u32 = 0x11;
pub const VK_MENU: u32 = 0x12; // ALT
pub const VK_PAUSE: u32 = 0x13;
pub const VK_CAPITAL: u32 = 0x14;
pub const VK_ESCAPE: u32 = 0x1B;
pub const VK_SPACE: u32 = 0x20;
pub const VK_PRIOR: u32 = 0x21; // PageUp
pub const VK_NEXT: u32 = 0x22; // PageDown
pub const VK_END: u32 = 0x23;
pub const VK_HOME: u32 = 0x24;
pub const VK_LEFT: u32 = 0x25;
pub const VK_UP: u32 = 0x26;
pub const VK_RIGHT: u32 = 0x27;
pub const VK_DOWN: u32 = 0x28;
pub const VK_INSERT: u32 = 0x2D;
pub const VK_DELETE: u32 = 0x2E;
pub const VK_LWIN: u32 = 0x5B;
pub const VK_RWIN: u32 = 0x5C;
pub const VK_NUMPAD0: u32 = 0x60;
pub const VK_F1: u32 = 0x70;
pub const VK_F11: u32 = 0x7A;
pub const VK_F12: u32 = 0x7B;
pub const VK_F24: u32 = 0x87;
pub const VK_LSHIFT: u32 = 0xA0;
pub const VK_RSHIFT: u32 = 0xA1;
pub const VK_LCONTROL: u32 = 0xA2;
pub const VK_RCONTROL: u32 = 0xA3;
pub const VK_LMENU: u32 = 0xA4;
pub const VK_RMENU: u32 = 0xA5;
pub const VK_OEM_PLUS: u32 = 0xBB;
pub const VK_OEM_COMMA: u32 = 0xBC;
pub const VK_OEM_MINUS: u32 = 0xBD;
pub const VK_OEM_PERIOD: u32 = 0xBE;
pub const VK_OEM_3: u32 = 0xC0; // `~`

/// Represents a keyboard shortcut combination (modifiers + virtual key code).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct KeyCombo {
    /// Bitmask of `MOD_SHIFT`, `MOD_CONTROL`, `MOD_ALT`, `MOD_WIN`.
    pub modifiers: u32,
    /// Win32 Virtual-Key code (`VK_*`), or `0` for `Keys.None`.
    pub vk: u32,
}

impl KeyCombo {
    pub const NONE: Self = Self {
        modifiers: 0,
        vk: 0,
    };

    /// Default shortcut for selecting and pinning a window (`CTRL + F11`).
    pub const DEFAULT_PIN_PROMPT: Self = Self {
        modifiers: MOD_CONTROL,
        vk: VK_F11,
    };

    /// Default shortcut for pinning the window under cursor (`CTRL + F12`).
    pub const DEFAULT_PIN_UNDER_CURSOR: Self = Self {
        modifiers: MOD_CONTROL,
        vk: VK_F12,
    };

    pub const fn new(modifiers: u32, vk: u32) -> Self {
        Self { modifiers, vk }
    }

    pub const fn is_none(&self) -> bool {
        self.vk == 0 || self.modifiers == 0 || is_modifier_only_vk(self.vk)
    }

    pub const fn has_shift(&self) -> bool {
        (self.modifiers & MOD_SHIFT) != 0
    }

    pub const fn has_ctrl(&self) -> bool {
        (self.modifiers & MOD_CONTROL) != 0
    }

    pub const fn has_alt(&self) -> bool {
        (self.modifiers & MOD_ALT) != 0
    }

    pub const fn has_win(&self) -> bool {
        (self.modifiers & MOD_WIN) != 0
    }

    /// Convert to user-friendly shortcut string compatible with C# `KeysStringConverter.ToString`.
    /// Returns `None` if there is no modifier or if only modifier keys are pressed.
    pub fn to_shortcut_string(&self) -> Option<String> {
        if self.is_none() {
            return None;
        }
        let key_name = vk_to_key_name(self.vk)?;
        let mut parts = Vec::with_capacity(5);
        // Preserve original C# KeysStringConverter order: SHIFT, CTRL, ALT (plus WIN if set)
        if self.has_shift() {
            parts.push("SHIFT".to_string());
        }
        if self.has_ctrl() {
            parts.push("CTRL".to_string());
        }
        if self.has_alt() {
            parts.push("ALT".to_string());
        }
        if self.has_win() {
            parts.push("WIN".to_string());
        }
        if parts.is_empty() {
            return None;
        }
        parts.push(key_name);
        Some(parts.join(" + "))
    }

    /// Parse a user-friendly shortcut string (e.g. `"CTRL + F11"`, `"SHIFT + CTRL + A"`).
    /// Returns `KeyCombo::NONE` if invalid or empty, matching `KeysStringConverter.FromString`.
    pub fn from_shortcut_string(input: &str) -> Self {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Self::NONE;
        }

        let parts: Vec<&str> = trimmed.split('+').collect();
        if parts.len() < 2 {
            return Self::NONE;
        }

        let mut modifiers = 0u32;
        for &mod_part in &parts[..parts.len() - 1] {
            match parse_modifier(mod_part) {
                Some(m) => modifiers |= m,
                None => return Self::NONE,
            }
        }

        if modifiers == 0 {
            return Self::NONE;
        }

        let last_part = parts[parts.len() - 1].trim();
        match key_name_to_vk(last_part) {
            Some(vk) if !is_modifier_only_vk(vk) && vk != 0 => Self { modifiers, vk },
            _ => Self::NONE,
        }
    }
}

impl fmt::Debug for KeyCombo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.to_shortcut_string() {
            Some(s) => write!(f, "KeyCombo(\"{}\")", s),
            None => write!(f, "KeyCombo(None)"),
        }
    }
}

impl fmt::Display for KeyCombo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(s) = self.to_shortcut_string() {
            f.write_str(&s)
        } else {
            Ok(())
        }
    }
}

/// Returns `true` if `vk` is purely a modifier key (Shift, Ctrl, Alt, Win).
pub const fn is_modifier_only_vk(vk: u32) -> bool {
    matches!(
        vk,
        VK_SHIFT
            | VK_CONTROL
            | VK_MENU
            | VK_LWIN
            | VK_RWIN
            | VK_LSHIFT
            | VK_RSHIFT
            | VK_LCONTROL
            | VK_RCONTROL
            | VK_LMENU
            | VK_RMENU
    )
}

fn parse_modifier(raw: &str) -> Option<u32> {
    match raw.trim().to_ascii_uppercase().as_str() {
        "SHIFT" => Some(MOD_SHIFT),
        "CTRL" | "CONTROL" => Some(MOD_CONTROL),
        "ALT" | "MENU" => Some(MOD_ALT),
        "WIN" | "WINDOWS" | "SUPER" => Some(MOD_WIN),
        _ => None,
    }
}

/// Convert a Win32 Virtual-Key code to its canonical C# `Keys` enum name.
pub fn vk_to_key_name(vk: u32) -> Option<String> {
    if is_modifier_only_vk(vk) || vk == 0 {
        return None;
    }
    if (VK_F1..=VK_F24).contains(&vk) {
        return Some(format!("F{}", vk - VK_F1 + 1));
    }
    if (0x41..=0x5A).contains(&vk) {
        return Some((vk as u8 as char).to_string());
    }
    if (0x30..=0x39).contains(&vk) {
        return Some(format!("D{}", vk - 0x30));
    }
    if (VK_NUMPAD0..=VK_NUMPAD0 + 9).contains(&vk) {
        return Some(format!("NumPad{}", vk - VK_NUMPAD0));
    }

    let named = match vk {
        VK_SPACE => "Space",
        VK_TAB => "Tab",
        VK_RETURN => "Return",
        VK_ESCAPE => "Escape",
        VK_BACK => "Back",
        VK_INSERT => "Insert",
        VK_DELETE => "Delete",
        VK_HOME => "Home",
        VK_END => "End",
        VK_PRIOR => "PageUp",
        VK_NEXT => "Next",
        VK_LEFT => "Left",
        VK_UP => "Up",
        VK_RIGHT => "Right",
        VK_DOWN => "Down",
        VK_PAUSE => "Pause",
        VK_CAPITAL => "Capital",
        VK_OEM_PLUS => "Oemplus",
        VK_OEM_COMMA => "Oemcomma",
        VK_OEM_MINUS => "OemMinus",
        VK_OEM_PERIOD => "OemPeriod",
        VK_OEM_3 => "Oemtilde",
        _ => return None,
    };
    Some(named.to_string())
}

/// Convert a key name (case-insensitive, compatible with C# `Keys` enum) to a Win32 Virtual-Key code.
pub fn key_name_to_vk(name: &str) -> Option<u32> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    let upper = trimmed.to_ascii_uppercase();

    if upper.len() == 1 {
        let b = upper.as_bytes()[0];
        if b.is_ascii_uppercase() {
            return Some(b as u32);
        }
        if b.is_ascii_digit() {
            return Some(0x30 + (b - b'0') as u32);
        }
    }

    if let Some(rest) = upper.strip_prefix('D')
        && rest.len() == 1
        && rest.as_bytes()[0].is_ascii_digit()
    {
        return Some(0x30 + (rest.as_bytes()[0] - b'0') as u32);
    }

    if let Some(rest) = upper.strip_prefix('F')
        && let Ok(num) = rest.parse::<u32>()
        && (1..=24).contains(&num)
    {
        return Some(VK_F1 + num - 1);
    }

    if let Some(rest) = upper.strip_prefix("NUMPAD")
        && rest.len() == 1
        && rest.as_bytes()[0].is_ascii_digit()
    {
        return Some(VK_NUMPAD0 + (rest.as_bytes()[0] - b'0') as u32);
    }

    match upper.as_str() {
        "SPACE" => Some(VK_SPACE),
        "TAB" => Some(VK_TAB),
        "RETURN" | "ENTER" => Some(VK_RETURN),
        "ESCAPE" | "ESC" => Some(VK_ESCAPE),
        "BACK" | "BACKSPACE" => Some(VK_BACK),
        "INSERT" | "INS" => Some(VK_INSERT),
        "DELETE" | "DEL" => Some(VK_DELETE),
        "HOME" => Some(VK_HOME),
        "END" => Some(VK_END),
        "PAGEUP" | "PRIOR" => Some(VK_PRIOR),
        "PAGEDOWN" | "NEXT" => Some(VK_NEXT),
        "LEFT" => Some(VK_LEFT),
        "UP" => Some(VK_UP),
        "RIGHT" => Some(VK_RIGHT),
        "DOWN" => Some(VK_DOWN),
        "PAUSE" => Some(VK_PAUSE),
        "CAPITAL" | "CAPSLOCK" => Some(VK_CAPITAL),
        "OEMPLUS" | "PLUS" => Some(VK_OEM_PLUS),
        "OEMCOMMA" | "COMMA" => Some(VK_OEM_COMMA),
        "OEMMINUS" | "MINUS" => Some(VK_OEM_MINUS),
        "OEMPERIOD" | "PERIOD" => Some(VK_OEM_PERIOD),
        "OEMTILDE" | "TILDE" | "OEM3" => Some(VK_OEM_3),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_shortcuts_format_and_parse() {
        let p1 = KeyCombo::DEFAULT_PIN_PROMPT;
        assert_eq!(p1.to_shortcut_string().as_deref(), Some("CTRL + F11"));
        assert_eq!(KeyCombo::from_shortcut_string("CTRL + F11"), p1);

        let p2 = KeyCombo::DEFAULT_PIN_UNDER_CURSOR;
        assert_eq!(p2.to_shortcut_string().as_deref(), Some("CTRL + F12"));
        assert_eq!(KeyCombo::from_shortcut_string("CTRL + F12"), p2);
    }

    #[test]
    fn test_modifier_ordering_matches_csharp_converter() {
        let combo = KeyCombo::new(MOD_SHIFT | MOD_CONTROL | MOD_ALT, b'P' as u32);
        assert_eq!(
            combo.to_shortcut_string().as_deref(),
            Some("SHIFT + CTRL + ALT + P")
        );
        assert_eq!(
            KeyCombo::from_shortcut_string("ALT + CTRL + SHIFT + p"),
            combo
        );
    }

    #[test]
    fn test_rejects_unmodified_or_modifier_only_keys() {
        let no_mod = KeyCombo::new(0, VK_F11);
        assert_eq!(no_mod.to_shortcut_string(), None);
        assert_eq!(KeyCombo::from_shortcut_string("F11"), KeyCombo::NONE);
        assert_eq!(KeyCombo::from_shortcut_string("CTRL + "), KeyCombo::NONE);
        assert_eq!(
            KeyCombo::from_shortcut_string("CTRL + SHIFT"),
            KeyCombo::NONE
        );
        assert_eq!(KeyCombo::from_shortcut_string(""), KeyCombo::NONE);
    }

    #[test]
    fn test_digits_and_special_keys() {
        let d5 = KeyCombo::from_shortcut_string("CTRL + D5");
        assert_eq!(d5.to_shortcut_string().as_deref(), Some("CTRL + D5"));
        let num0 = KeyCombo::from_shortcut_string("ALT + NumPad0");
        assert_eq!(num0.to_shortcut_string().as_deref(), Some("ALT + NumPad0"));
    }
}
