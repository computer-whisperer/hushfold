//! The Hearth Library: department 11, top floor, west side.

use super::*;

pub(super) fn furnish(lib: &mut Library, frame: &RoomFrame, rng: &mut StdRng, titles: &mut HashSet<String>) {
    classic(lib, frame, rng, titles);
}
