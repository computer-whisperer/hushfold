//! Light and fog place by place: each department's room a mood of its own, the Hall
//! and the atrium the library's, eased between as the player walks.

use super::sound::smoothstep;
use super::*;

/// How far in from its doorway a room's mood has wholly taken over, and how far out
/// into the atrium it carries.
const INTO_ROOM: f32 = 4.0;
const OUT_OF_ROOM: f32 = 3.0;

/// The light the eye is in: the fog's colour and density (w), the ambient light from
/// above and from below, and a tint on the library's lights.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Mood {
    pub fog: [f32; 4],
    pub top: [f32; 3],
    pub bottom: [f32; 3],
    pub light: [f32; 3],
}

impl Mood {
    /// Moonlit blue overhead, candlelit warmth underfoot.
    pub const LIBRARY: Mood = Mood {
        fog: [0.035, 0.03, 0.04, 0.022],
        top: [0.19, 0.18, 0.24],
        bottom: [0.09, 0.065, 0.05],
        light: [1.0; 3],
    };

    /// A department's, by its name: each a shade off the library's, about as bright.
    fn department(name: &str) -> Mood {
        let m = |fog: [f32; 4], top, bottom, light| Mood {
            fog,
            top,
            bottom,
            light,
        };
        match name {
            // A clear night, starlit blue.
            "Observatory" => m(
                [0.02, 0.03, 0.065, 0.022],
                [0.14, 0.17, 0.32],
                [0.06, 0.06, 0.09],
                [0.92, 0.97, 1.12],
            ),
            // The athanor's amber, a green tinge off the glassware.
            "Alchemical Wing" => m(
                [0.045, 0.038, 0.022, 0.026],
                [0.18, 0.2, 0.16],
                [0.13, 0.08, 0.04],
                [1.06, 1.02, 0.86],
            ),
            // Moonlight through leaves.
            "Herbarium" => m(
                [0.02, 0.04, 0.034, 0.022],
                [0.12, 0.22, 0.19],
                [0.06, 0.085, 0.05],
                [0.92, 1.05, 0.96],
            ),
            // Earthy and close.
            "Bestiary" => m(
                [0.04, 0.03, 0.022, 0.026],
                [0.15, 0.13, 0.14],
                [0.11, 0.075, 0.04],
                [1.05, 0.98, 0.88],
            ),
            // Sepia, like old paper.
            "Map Room" => m(
                [0.045, 0.036, 0.026, 0.022],
                [0.21, 0.18, 0.17],
                [0.11, 0.08, 0.05],
                [1.06, 1.0, 0.88],
            ),
            // Grave, grey stone.
            "Chronicle Hall" => m(
                [0.034, 0.034, 0.038, 0.022],
                [0.18, 0.185, 0.21],
                [0.08, 0.072, 0.065],
                [0.98, 0.99, 1.04],
            ),
            // Rose and lilac.
            "Verse Gallery" => m(
                [0.045, 0.028, 0.045, 0.022],
                [0.23, 0.16, 0.24],
                [0.11, 0.06, 0.07],
                [1.07, 0.94, 1.02],
            ),
            // Violet, a little hazy with magic.
            "Arcanum" => m(
                [0.038, 0.022, 0.06, 0.028],
                [0.2, 0.13, 0.32],
                [0.08, 0.05, 0.09],
                [1.0, 0.9, 1.15],
            ),
            // Brass and lamplight.
            "Clockworks" => m(
                [0.045, 0.034, 0.022, 0.022],
                [0.2, 0.17, 0.16],
                [0.12, 0.08, 0.035],
                [1.08, 1.0, 0.86],
            ),
            // Crimson velvet.
            "Music Room" => m(
                [0.048, 0.026, 0.03, 0.022],
                [0.21, 0.14, 0.18],
                [0.12, 0.055, 0.05],
                [1.1, 0.95, 0.92],
            ),
            // A grey-blue storm, mistier.
            "Weatherhouse" => m(
                [0.03, 0.038, 0.05, 0.034],
                [0.15, 0.19, 0.27],
                [0.065, 0.07, 0.08],
                [0.92, 0.98, 1.08],
            ),
            // Firelit.
            "Hearth Library" => m(
                [0.05, 0.034, 0.022, 0.022],
                [0.2, 0.16, 0.15],
                [0.15, 0.085, 0.04],
                [1.1, 0.98, 0.84],
            ),
            _ => Mood::LIBRARY,
        }
    }

    /// `self` with `w` of `other` added in.
    fn add(mut self, other: Mood, w: f32) -> Mood {
        let mix = |a: &mut [f32], b: &[f32]| a.iter_mut().zip(b).for_each(|(a, b)| *a += b * w);
        mix(&mut self.fog, &other.fog);
        mix(&mut self.top, &other.top);
        mix(&mut self.bottom, &other.bottom);
        mix(&mut self.light, &other.light);
        self
    }
}

impl Game {
    /// The light where the player stands: a room's mood fills it from a little way in
    /// from its doorway and carries a little way out into the atrium (as its sound
    /// does), so it changes smoothly walking in and out.
    pub(super) fn mood(&self) -> Mood {
        if self.layout == Layout::Hall {
            return Mood::LIBRARY;
        }
        let feet = self.player.pos;
        let (level, inside) = (self.lib.world.level_at(feet), self.lib.room_at(feet));
        let none = Mood {
            fog: [0.0; 4],
            top: [0.0; 3],
            bottom: [0.0; 3],
            light: [0.0; 3],
        };
        let mut mood = none;
        let mut atrium = 1.0;
        for (d, room) in self.lib.rooms.iter().enumerate() {
            let from_door = Vec2::new(feet.x - room.door.x, feet.z - room.door.z).length();
            let w = match inside {
                Some(i) if i == d => 0.5 + 0.5 * smoothstep(from_door / INTO_ROOM),
                None if room.level == level => 0.5 * (1.0 - smoothstep(from_door / OUT_OF_ROOM)),
                _ => 0.0,
            };
            if w > 0.0 {
                mood = mood.add(Mood::department(&self.lib.catalog.departments[d].name), w);
                atrium -= w;
            }
        }
        mood.add(Mood::LIBRARY, atrium)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_great_department_has_its_own_mood() {
        let g = Game::new(Layout::Great);
        for d in &g.lib.catalog.departments {
            assert_ne!(Mood::department(&d.name), Mood::LIBRARY, "{}", d.name);
        }
    }

    #[test]
    fn a_room_mood_fades_in_from_its_doorway() {
        let mut g = Game::new(Layout::Great);
        let room = &g.lib.rooms[0];
        let mid = (room.min + room.max) * 0.5;
        let into = (Vec3::new(mid.x, room.door.y, mid.y) - room.door).normalize();
        let (door, own) = (room.door, Mood::department(&g.lib.catalog.departments[0].name));
        let mut at = |p: Vec3| {
            g.player.pos = p;
            g.mood()
        };
        let (deep, at_door, outside) = (at(door + into * 6.0), at(door + into * 0.05), at(door - into * 4.0));
        assert_eq!(deep, own);
        assert_eq!(outside, Mood::LIBRARY);
        for k in 0..3 {
            let half = (own.top[k] + Mood::LIBRARY.top[k]) * 0.5;
            assert!((at_door.top[k] - half).abs() < 0.01, "{at_door:?}");
        }
    }
}
