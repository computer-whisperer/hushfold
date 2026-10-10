//! What the player hears from where they stand, told to `Audio` each frame: the
//! listener's pose, how much of each place's ambience reaches them, the fires in
//! earshot and the familiars in flight.

use super::*;
use crate::audio::{Place, Scene};

/// How far in from its doorway a room's sound has wholly taken over, and how far
/// out into the atrium it carries.
const INTO_ROOM: f32 = 5.0;
const OUT_OF_ROOM: f32 = 6.0;
/// A fire is a flickering light at least this bright (candles and lamps are dimmer)
/// and at most this high above its floor (chandeliers hang higher).
const FIRE_LIGHT: f32 = 2.0;
const FIRE_HIGH: f32 = 1.5;
/// Height of the atrium's roof, for how lofty its sound grows.
const ROOF: f32 = 18.0;
/// A posted familiar further than this from its post is on the wing (its perch is
/// lower).
const PERCHED: f32 = 1.5;
/// Familiars further off than this go unheard.
const WINGS_REACH: f32 = 25.0;

fn smoothstep(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

impl Game {
    /// The ambience a department's room has.
    fn room_place(&self, d: usize) -> Place {
        match self.layout {
            Layout::Hall => Place::Hall,
            Layout::Great => Place::department(&self.lib.catalog.departments[d].name),
        }
    }

    /// Where the library's fires burn: by their light.
    fn fires(&self) -> impl Iterator<Item = Vec3> + '_ {
        let world = &self.lib.world;
        (self.lib.lights.iter())
            .filter(move |l| {
                let floor = world.levels[world.level_at(l.pos)].y;
                l.kind == LightKind::Flicker && l.intensity >= FIRE_LIGHT && l.pos.y - floor <= FIRE_HIGH
            })
            .map(|l| l.pos)
    }

    /// Tell `audio` where the player is and what is around them.
    pub fn update_sound(&self, audio: &mut Audio) {
        audio.set_scene(self.scene());
    }

    /// Where the player is and what is around them, as `Audio` hears it.
    fn scene(&self) -> Scene {
        let (ears, feet) = (self.player.eye(), self.player.pos);
        let mut scene = Scene {
            ears,
            right: self.player.right(),
            ..Scene::default()
        };
        match self.layout {
            Layout::Hall => scene.places[Place::Hall as usize] = 1.0,
            Layout::Great => {
                // A room's sound fills it from a little way in, and spills out of its
                // doorway into the atrium.
                let (level, inside) = (self.lib.world.level_at(feet), self.lib.room_at(feet));
                let mut atrium = 1.0;
                for (d, room) in self.lib.rooms.iter().enumerate() {
                    let from_door = Vec2::new(feet.x - room.door.x, feet.z - room.door.z).length();
                    let w = match inside {
                        Some(i) if i == d => 0.5 + 0.5 * smoothstep(from_door / INTO_ROOM),
                        None if room.level == level => 0.5 * (1.0 - smoothstep(from_door / OUT_OF_ROOM)),
                        _ => 0.0,
                    };
                    scene.places[self.room_place(d) as usize] += w;
                    atrium -= w;
                }
                scene.places[Place::Atrium as usize] = atrium;
                scene.loft = (ears.y / ROOF).clamp(0.0, 1.0);
            }
        }
        // Fires, nearest first, each as loud as the place it burns in is heard.
        let mut fires: Vec<(f32, Vec3, f32)> = (self.fires())
            .map(|at| {
                let place = match self.layout {
                    Layout::Hall => Place::Hall,
                    Layout::Great => self.lib.room_at(at).map_or(Place::Atrium, |d| self.room_place(d)),
                };
                (at.distance(ears), at, scene.places[place as usize])
            })
            .filter(|f| f.2 > 0.0)
            .collect();
        fires.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (slot, (_, at, heard)) in scene.fires.iter_mut().zip(fires) {
            *slot = (at, heard);
        }
        // Familiars on the wing, nearest first.
        let mut wings: Vec<(f32, Vec3)> = (self.familiars.iter())
            .filter(|f| f.post.as_ref().is_some_and(|p| f.pos.distance(p.at) > PERCHED))
            .map(|f| (f.pos.distance(ears), f.pos))
            .filter(|w| w.0 < WINGS_REACH)
            .collect();
        wings.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (slot, (_, at)) in scene.wings.iter_mut().zip(wings) {
            *slot = Some(at);
        }
        scene
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_great_department_has_its_own_ambience() {
        let g = Game::new(Layout::Great);
        let mut places: Vec<Place> = (0..g.lib.rooms.len()).map(|d| g.room_place(d)).collect();
        assert!(!places.contains(&Place::Room), "{places:?}");
        places.sort_by_key(|p| *p as usize);
        places.dedup();
        assert_eq!(places.len(), g.lib.rooms.len());
    }

    #[test]
    fn fires_are_found_where_they_burn() {
        let fires = |g: &Game| -> Vec<Option<usize>> { g.fires().map(|at| g.lib.room_at(at)).collect() };
        let hall = Game::new(Layout::Hall);
        assert_eq!(fires(&hall).len(), 1, "the Hall's hearth");
        let great = Game::new(Layout::Great);
        let mut rooms: Vec<Place> = fires(&great)
            .iter()
            .map(|d| great.room_place(d.expect("a fire in a room")))
            .collect();
        rooms.sort_by_key(|p| *p as usize);
        assert_eq!(rooms, [Place::AlchemicalWing, Place::HearthLibrary]);
    }

    #[test]
    fn a_room_fades_in_from_its_doorway() {
        let mut g = Game::new(Layout::Great);
        let room = &g.lib.rooms[0];
        let mid = (room.min + room.max) * 0.5;
        let into = (Vec3::new(mid.x, room.door.y, mid.y) - room.door).normalize();
        let (door, place) = (room.door, g.room_place(0));
        let weight = |g: &mut Game, p: Vec3| {
            g.player.pos = p;
            let s = g.scene();
            (s.places[place as usize], s.places[Place::Atrium as usize])
        };
        let (deep, at_door, outside) = (
            weight(&mut g, door + into * 8.0),
            weight(&mut g, door + into * 0.3),
            weight(&mut g, door - into * 3.0),
        );
        assert!(deep.0 > 0.99 && deep.1 < 0.01, "{deep:?}");
        assert!((at_door.0 - 0.5).abs() < 0.1, "{at_door:?}");
        assert!(outside.0 > 0.05 && outside.0 < 0.5 && outside.1 > 0.5, "{outside:?}");
    }
}
