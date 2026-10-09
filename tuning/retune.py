"""Fit a library's skill costs (upg_costs in src/game.rs) to a purchase schedule.

A schedule (tuning/<library>.schedule) lists every purchase in the order the
cheapest-first pacing bot should make it, with the minute it should come: each
skill's levels in order. Costs must rise along the schedule for the bot to keep its
order. Fitting is a few rounds of: run the bot, then scale each cost by (target
minute / the minute the bot bought it); one never bought counts as bought at 1.3x
the run's end. The bot's buying feeds back into its earning, so a round can
overshoot; it settles in two or three.

  python3 tuning/retune.py compare <hall|great> <bot output>
  python3 tuning/retune.py scale   <hall|great> <bot output> [beta=1]
  python3 tuning/retune.py sorted  <hall|great>   (re-deal the current costs, sorted,
                                                  along a reordered schedule)

The bot's output comes from
  cargo test --release pacing_<hall|great> -- --ignored --nocapture > out.txt
"""
import os
import re
import sys
from collections import OrderedDict

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
GAME = os.path.join(ROOT, "src", "game.rs")
ARM = {"hall": "Hall", "great": "Great"}


def schedule(lib):
    path = os.path.join(ROOT, "tuning", f"{lib}.schedule")
    out = []
    for line in open(path):
        line = line.split("#")[0].strip()
        if line:
            u, m = line.split()
            out.append((u, float(m)))
    return out


def arm_span(src, lib):
    m = re.search(r"(        Layout::%s => match u \{\n)(.*?)(\n        \},)" % ARM[lib], src, re.S)
    assert m, f"no arm for {lib} in upg_costs"
    return m


def read_costs(lib):
    body = arm_span(open(GAME).read(), lib).group(2)
    out = OrderedDict()
    for m in re.finditer(r"Upg::(\w+) => &\[([\d, ]+)\]", body):
        out[m.group(1)] = [int(x) for x in m.group(2).split(",")]
    return out


def write_costs(lib, sched, cs):
    """Costs `cs` in schedule order, kept rising and rounded to two figures."""
    by = OrderedDict()
    prev = 0
    for (u, _), c in zip(sched, cs):
        c = max(int(round(c)), prev + 1)
        mag = 10 ** max(len(str(c)) - 2, 0)
        c = max(int(round(c / mag)) * mag, prev + 1)
        by.setdefault(u, []).append(c)
        prev = c
    src = open(GAME).read()
    m = arm_span(src, lib)
    body = m.group(2)
    # Keep the arms for skills this library doesn't offer (empty costs) as they are.
    kept = [l for l in body.splitlines() if "=> &[]" in l or l.strip().startswith("//")]
    lines = [f"            Upg::{u} => &{v}," for u, v in by.items()] + kept
    src = src[: m.start(2)] + "\n".join(lines) + src[m.end(2):]
    open(GAME, "w").write(src)
    print(f"{ARM[lib]}: {len(sched)} purchases, costing {sum(sum(v) for v in by.values())} in all")


def bought(run):
    text = open(run).read()
    got = [(m.group(2), float(m.group(1))) for m in re.finditer(r"([\d.]+) min  bought (\w+) ->", text)]
    end = re.search(r"done at ([\d.]+) h", text)
    return got, float(end.group(1)) * 60 if end else None


def in_schedule_order(sched, costs):
    seen = {}
    out = []
    for u, _ in sched:
        i = seen.get(u, 0)
        seen[u] = i + 1
        out.append(costs[u][i])
    return out


def main():
    cmd, lib = sys.argv[1], sys.argv[2]
    sched = schedule(lib)
    if cmd == "sorted":
        cs = sorted(c for v in read_costs(lib).values() for c in v)
        assert len(cs) == len(sched), f"{len(cs)} costs for {len(sched)} purchases"
        write_costs(lib, sched, cs)
        return
    got, end = bought(sys.argv[3])
    if cmd == "compare":
        print(" ".join(f"{u}:{t:g}/{g[1]:.0f}" for (u, t), g in zip(sched, got)))
        same = [u for u, _ in sched][: len(got)] == [u for u, _ in got]
        print(f"{len(got)} of {len(sched)} bought, in the schedule's order: {same}; done at {end and end / 60:.2f} h")
        return
    if cmd == "scale":
        beta = float(sys.argv[4]) if len(sys.argv) > 4 else 1.0
        cs = in_schedule_order(sched, read_costs(lib))
        new = []
        for k, ((u, target), c) in enumerate(zip(sched, cs)):
            t = got[k][1] if k < len(got) else (end or target) * 1.3
            new.append(c * (target / max(t, 0.5)) ** beta)
        write_costs(lib, sched, new)
        return
    sys.exit(__doc__)


main()
