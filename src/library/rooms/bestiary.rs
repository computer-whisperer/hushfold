//! The Bestiary: department 3, ground floor, west side.

use super::*;

pub(super) fn furnish(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    classic(lib, frame, rng, titles);
}
