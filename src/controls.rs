//! The controls, by what they do, and their names: on the keyboard and mouse, and on
//! each kind of gamepad, by the labels on its buttons. Text shown to the player names
//! a control by a token in brackets (`[seek]`), filled in as it is drawn (`fill`), so
//! a hint given while the player was on the keyboard reads right once they take up a
//! pad.

/// Whose names the prompts give.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Prompts {
    #[default]
    Keys,
    /// Xbox's labels, which Steam's and most other pads share.
    Xbox,
    PlayStation,
    /// Nintendo's: A and B, X and Y, each where the other is on an Xbox pad.
    Nintendo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Walk,
    Hurry,
    Jump,
    Look,
    /// Take, scoop and file.
    Take,
    Toss,
    /// Toss the whole armful.
    Armful,
    Seek,
    SortBy,
    /// Cast a spell: on the keys, its number; on a pad, the one chosen.
    Cast,
    /// Choose a spell to cast (a pad's; the keys cast one directly).
    Choose,
    Post,
    Cycle,
    Grimoire,
    Pause,
}

const TOKENS: [(&str, Act); 15] = [
    ("walk", Act::Walk),
    ("hurry", Act::Hurry),
    ("jump", Act::Jump),
    ("look", Act::Look),
    ("take", Act::Take),
    ("toss", Act::Toss),
    ("armful", Act::Armful),
    ("seek", Act::Seek),
    ("sortby", Act::SortBy),
    ("cast", Act::Cast),
    ("choose", Act::Choose),
    ("post", Act::Post),
    ("cycle", Act::Cycle),
    ("grimoire", Act::Grimoire),
    ("pause", Act::Pause),
];

impl Prompts {
    pub fn pad(self) -> bool {
        self != Prompts::Keys
    }

    /// The control that does `act`.
    pub fn name(self, act: Act) -> &'static str {
        use Act::*;
        use Prompts::*;
        match (self, act) {
            (Keys, Walk) => "WASD",
            (Keys, Hurry) => "Shift",
            (Keys, Jump) => "Space",
            (Keys, Look) => "Mouse",
            (Keys, Take) => "Left click",
            (Keys, Toss) => "Right mouse",
            (Keys, Armful) => "X",
            (Keys, Seek) => "F",
            (Keys, SortBy) => "R",
            (Keys, Cast | Choose) => "1–5",
            (Keys, Post) => "G",
            (Keys, Cycle) => "Scroll, Q / E",
            (Keys, Grimoire) => "Tab",
            (Keys, Pause) => "Esc",
            (_, Walk) => "Left stick",
            (_, Look) => "Right stick",
            (_, Choose) => "D-pad ◀ ▶",
            (_, Cast) => "D-pad ▲",
            (_, Post) => "D-pad ▼",
            (PlayStation, Hurry) => "L3",
            (_, Hurry) => "LS",
            (Xbox, Jump) => "A",
            (PlayStation, Jump) => "✕",
            (Nintendo, Jump) => "B",
            (Xbox, Take) => "RT",
            (PlayStation, Take) => "R2",
            (Nintendo, Take) => "ZR",
            (Xbox, Toss) => "LT",
            (PlayStation, Toss) => "L2",
            (Nintendo, Toss) => "ZL",
            (Xbox, Armful) => "B",
            (PlayStation, Armful) => "○",
            (Nintendo, Armful) => "A",
            (Xbox, Seek) => "X",
            (PlayStation, Seek) => "□",
            (Nintendo, Seek) => "Y",
            (Xbox, SortBy) => "Y",
            (PlayStation, SortBy) => "△",
            (Nintendo, SortBy) => "X",
            (Xbox, Cycle) => "LB / RB",
            (PlayStation, Cycle) => "L1 / R1",
            (Nintendo, Cycle) => "L / R",
            (Xbox, Grimoire) => "View",
            (PlayStation, Grimoire) => "Create",
            // An en dash: the HUD's face has no minus sign.
            (Nintendo, Grimoire) => "–",
            (Xbox, Pause) => "Menu",
            (PlayStation, Pause) => "Options",
            (Nintendo, Pause) => "+",
        }
    }

    /// `text` with each control's token (`[seek]`) given its name.
    pub fn fill(self, text: &str) -> String {
        let mut out = String::with_capacity(text.len());
        let mut rest = text;
        while let Some(open) = rest.find('[') {
            out.push_str(&rest[..open]);
            let after = &rest[open + 1..];
            let act = after
                .find(']')
                .and_then(|close| Some((close, TOKENS.iter().find(|(t, _)| *t == &after[..close])?.1)));
            match act {
                Some((close, act)) => {
                    out.push_str(self.name(act));
                    rest = &after[close + 1..];
                }
                None => {
                    out.push('[');
                    rest = after;
                }
            }
        }
        out.push_str(rest);
        out
    }

    /// Every control and what it does, for the controls' list.
    pub fn list(self) -> Vec<(&'static str, &'static str)> {
        let n = |act| self.name(act);
        let mut rows = vec![
            (n(Act::Walk), "walk"),
            (n(Act::Hurry), "hurry"),
            (n(Act::Jump), "jump"),
            (n(Act::Look), "look"),
            (
                n(Act::Take),
                "tap a book to take it · hold on a pile to scoop · tap a run to file",
            ),
            (n(Act::Toss), "hold to toss a group, release to throw"),
            (n(Act::Armful), "the same for the whole armful"),
            (n(Act::Seek), "seek the kind of the book or shelf you aim at"),
            (n(Act::SortBy), "sort by department, category or series"),
        ];
        if self.pad() {
            rows.push((n(Act::Choose), "choose a spell"));
            rows.push((n(Act::Cast), "cast it"));
        } else {
            rows.push((n(Act::Cast), "cast the spells you have learned"));
        }
        rows.extend([
            (n(Act::Post), "post a familiar where you aim, or take one back"),
            (n(Act::Cycle), "cycle the active book"),
            (n(Act::Grimoire), "the Grimoire"),
        ]);
        if !self.pad() {
            rows.extend([
                ("M", "music on or off"),
                ("F3", "frame rate"),
                ("F11", "fullscreen"),
                ("F12", "screenshot"),
            ]);
        }
        rows.push((n(Act::Pause), "pause"));
        rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_filled_and_the_rest_kept() {
        let text = "Press [seek] at a book; [grimoire] opens it. [not a token] [";
        assert_eq!(
            Prompts::Keys.fill(text),
            "Press F at a book; Tab opens it. [not a token] ["
        );
        assert_eq!(
            Prompts::PlayStation.fill(text),
            "Press □ at a book; Create opens it. [not a token] ["
        );
    }

    /// Every name is drawn in the HUD's faces: Caslon, or the symbols beside it.
    #[test]
    fn every_name_has_its_glyphs() {
        use ab_glyph::Font;
        let faces = [crate::text::CASLON, crate::hud::SYMBOLS].map(|f| ab_glyph::FontRef::try_from_slice(f).unwrap());
        for p in [Prompts::Keys, Prompts::Xbox, Prompts::PlayStation, Prompts::Nintendo] {
            for (name, _) in p.list() {
                for c in name.chars() {
                    assert!(faces.iter().any(|f| f.glyph_id(c).0 != 0), "{c:?} in {name:?}");
                }
            }
        }
    }

    /// Every token is filled with its act's name, everywhere.
    #[test]
    fn every_act_is_named() {
        for p in [Prompts::Keys, Prompts::Xbox, Prompts::PlayStation, Prompts::Nintendo] {
            for (token, act) in TOKENS {
                assert!(!p.name(act).is_empty());
                assert_eq!(p.fill(&format!("[{token}]")), p.name(act));
            }
        }
    }
}
