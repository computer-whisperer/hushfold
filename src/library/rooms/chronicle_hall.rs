//! Chronicle Hall: department 5, first floor, east side.

use super::*;

pub(super) fn furnish(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    classic(lib, frame, rng, titles);
}
