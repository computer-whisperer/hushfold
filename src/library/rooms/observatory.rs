//! The Observatory: department 0, ground floor, north side.

use super::*;

pub(super) fn furnish(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    classic(lib, frame, rng, titles);
}
