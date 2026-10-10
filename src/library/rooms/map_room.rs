//! The Map Room: department 4, first floor, north side.

use super::*;

pub(super) fn furnish(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    classic(lib, frame, rng, titles);
}
