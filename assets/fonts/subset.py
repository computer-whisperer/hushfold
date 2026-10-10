"""Build the game's fonts (assets/fonts/<Family>/) from their files in google/fonts.

Each family's font and licence come from google/fonts at REV. Fonts that reserve a
font name (in the licence's copyright statement, or in the font's own) are copied as
they are. The rest are instanced (variable fonts, at the axes below), subset to the
characters the game draws with them, and their GPOS pair kerning is written as a
`kern` table, the only kerning ab_glyph (lettering) and egui (the HUD) read.
Lettering faces drop their hinting (ab_glyph ignores it); the HUD's keeps it. Each
family's folder gets its licence and a SOURCE.txt saying where it came from and what
was done to it. Reruns with the same fontTools (4.63 made these) give the same bytes.

  python3 -I assets/fonts/subset.py <download dir, empty the first time>

Needs fontTools. Rerun it when text::CHARS changes (`every_face_has_every_glyph`
fails until then).
"""
import os
import shutil
import sys
import urllib.parse
import urllib.request

from fontTools import subset
from fontTools.ttLib import TTFont, newTable
from fontTools.ttLib.tables._k_e_r_n import KernTable_format_0
from fontTools.varLib import instancer

REV = "bd8f81ddb5c74d5c8897b36ad88b440266245103"
RAW = f"https://raw.githubusercontent.com/google/fonts/{REV}/"
BLOB = f"https://github.com/google/fonts/blob/{REV}/"
OUT = os.path.dirname(os.path.abspath(__file__))

# What a face is drawn with, and so which characters it keeps: lettering's; the HUD's
# (which it may share with lettering, whose characters it holds too); symbols.
LETTERING = "lettering"
HUD = "hud"
SYMBOLS = "symbols"

# text::CHARS: lettering in the world (signs, labels, spines).
CHARS = " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~–—‘’“”…·"
# The HUD: Latin-1 and the common punctuation (anything else falls back to egui's own
# fonts); its symbols come from the symbol font.
HUD_CHARS = "".join(map(chr, [*range(0x20, 0x7F), *range(0xA0, 0x100), *range(0x2010, 0x2028), *range(0x2030, 0x203B)]))
SYMBOL_CHARS = "✦▼"

# (folder, google/fonts directory, file, axes to instance at, use, upstream repository)
FACES = [
    ("EBGaramond", "ofl/ebgaramond", "EBGaramond[wght].ttf", {"wght": 400}, LETTERING,
     "https://github.com/octaviopardo/EBGaramond12"),
    ("Cardo", "ofl/cardo", "Cardo-Regular.ttf", None, LETTERING, "https://github.com/googlefonts/CardoFont"),
    ("IMFellEnglish", "ofl/imfellenglish", "IMFeENrm28P.ttf", None, LETTERING,
     "https://github.com/librefonts/imfellenglish"),
    ("LibreCaslonText", "ofl/librecaslontext", "LibreCaslonText[wght].ttf", {"wght": 400}, HUD,
     "https://github.com/thundernixon/Libre-Caslon"),
    ("IbarraRealNova", "ofl/ibarrarealnova", "IbarraRealNova[wght].ttf", {"wght": 400}, LETTERING,
     "https://github.com/googlefonts/ibarrareal"),
    ("CastoroTitling", "ofl/castorotitling", "CastoroTitling-Regular.ttf", None, LETTERING,
     "https://github.com/TiroTypeworks/Castoro"),
    ("Baskervville", "ofl/baskervville", "Baskervville[wght].ttf", {"wght": 400}, LETTERING,
     "https://github.com/anrt-type/ANRT-Baskervville"),
    ("BodoniModa", "ofl/bodonimoda", "BodoniModa[opsz,wght].ttf", {"opsz": 11, "wght": 400}, LETTERING,
     "https://github.com/indestructible-type/Bodoni"),
    ("OldStandardTT", "ofl/oldstandardtt", "OldStandard-Regular.ttf", None, LETTERING,
     "https://git.sr.ht/~ralessi/oldstandard"),
    ("Coustard", "ofl/coustard", "Coustard-Regular.ttf", None, LETTERING, "https://github.com/googlefonts/coustardFont"),
    ("SortsMillGoudy", "ofl/sortsmillgoudy", "SortsMillGoudy-Regular.ttf", None, LETTERING,
     "https://github.com/theleagueof/sorts-mill-goudy"),
    ("GoudyBookletter1911", "ofl/goudybookletter1911", "GoudyBookletter1911.ttf", None, LETTERING,
     "https://github.com/theleagueof/goudy-bookletter-1911"),
    ("Texturina", "ofl/texturina", "Texturina[opsz,wght].ttf", {"opsz": 12, "wght": 500}, LETTERING,
     "https://github.com/Omnibus-Type/Texturina"),
    ("UnifrakturMaguntia", "ofl/unifrakturmaguntia", "UnifrakturMaguntia-Book.ttf", None, LETTERING,
     "https://github.com/googlefonts/googlefontdirectory-hg"),
    ("UncialAntiqua", "ofl/uncialantiqua", "UncialAntiqua-Regular.ttf", None, LETTERING,
     "https://github.com/googlefonts/googlefontdirectory-hg"),
    ("Cinzel", "ofl/cinzel", "Cinzel[wght].ttf", {"wght": 400}, LETTERING, "https://github.com/NDISCOVER/Cinzel"),
    ("NotoSansSymbols2", "ofl/notosanssymbols2", "NotoSansSymbols2-Regular.ttf", None, SYMBOLS,
     "https://github.com/notofonts/symbols"),
]


def fetch(url, path):
    if not os.path.exists(path):
        with urllib.request.urlopen(url, timeout=60) as r:
            data = r.read()
        with open(path, "wb") as f:
            f.write(data)
    return path


def reserved_names(licence, font):
    """The copyright statement, heading the licence or in the font itself, that
    reserves a font name, if one does (the OFL's terms, which follow the heading,
    define the phrase in any case)."""
    head = " ".join(licence.split("This Font Software is licensed under")[0].split())
    texts = [("its licence", head)] + [("the font", str(n)) for n in font["name"].names if n.nameID == 0]
    return next((f'{where}: "{t}"' for where, t in texts if "reserved font name" in t.lower()), None)


def kern_lookups(gpos):
    """The lookups of the `kern` feature for Latin (else the default script)."""
    scripts = {s.ScriptTag: s.Script for s in gpos.ScriptList.ScriptRecord}
    script = scripts.get("latn") or scripts.get("DFLT") or next(iter(scripts.values()), None)
    if script is None or script.DefaultLangSys is None:
        return []
    feats = gpos.FeatureList.FeatureRecord
    found = set()
    for i in script.DefaultLangSys.FeatureIndex:
        if feats[i].FeatureTag == "kern":
            found.update(feats[i].Feature.LookupListIndex)
    return [gpos.LookupList.Lookup[i] for i in sorted(found)]


def pair_kerning(font, glyphs):
    """GPOS pair kerning between `glyphs`, as a shaper applies it: in each lookup the
    first subtable that takes a pair gives its x advance; lookups add."""
    if "GPOS" not in font:
        return {}
    subtables = []
    for lookup in kern_lookups(font["GPOS"].table):
        tables = []
        for st in lookup.SubTable:
            if lookup.LookupType == 9:
                st = st.ExtSubTable
            if getattr(st, "LookupType", 2) == 2 and st.Format in (1, 2):
                tables.append(st)
        subtables.append(tables)
    adv = lambda v: (getattr(v, "XAdvance", 0) or 0) if v is not None else 0  # noqa: E731
    out = {}
    for left in glyphs:
        for right in glyphs:
            total = 0
            for tables in subtables:
                for st in tables:
                    cov = st.Coverage.glyphs
                    if left not in cov:
                        continue
                    if st.Format == 1:
                        ps = st.PairSet[cov.index(left)]
                        rec = next((r for r in ps.PairValueRecord if r.SecondGlyph == right), None)
                        if rec is None:
                            continue
                        total += adv(rec.Value1)
                    else:
                        c1 = st.ClassDef1.classDefs.get(left, 0)
                        c2 = st.ClassDef2.classDefs.get(right, 0)
                        total += adv(st.Class1Record[c1].Class2Record[c2].Value1)
                    break
            if total:
                out[(left, right)] = total
    return out


def write_kern(font, pairs):
    """`pairs` as a version 0 `kern` table, in subtables of at most 10000 pairs (a
    format 0 subtable's length is 16 bits); readers take a pair from the first
    subtable holding it."""
    items = sorted(pairs.items())
    tables = [dict(items[i : i + 10000]) for i in range(0, len(items), 10000)]
    kern = newTable("kern")
    kern.version = 0
    kern.kernTables = []
    for t in tables:
        st = KernTable_format_0()
        st.version, st.coverage, st.format = 0, 1, 0
        st.kernTable = t
        kern.kernTables.append(st)
    font["kern"] = kern


def build(folder, gdir, name, axes, use, upstream, downloads):
    d = os.path.join(downloads, gdir.replace("/", "_"))
    os.makedirs(d, exist_ok=True)
    lic_name = "LICENSE.txt" if gdir.startswith("apache/") else "OFL.txt"
    lic_path = fetch(RAW + gdir + "/" + lic_name, os.path.join(d, lic_name))
    src = fetch(RAW + gdir + "/" + urllib.parse.quote(name, safe=","), os.path.join(d, name))
    licence = open(lic_path, encoding="utf-8").read()
    font = TTFont(src, recalcTimestamp=False)
    family = font["name"].getBestFamilyName()
    designer = font["name"].getDebugName(9) or ""
    out = os.path.join(OUT, folder)
    shutil.rmtree(out, ignore_errors=True)
    os.makedirs(out)
    shutil.copy(lic_path, os.path.join(out, lic_name))
    stem = os.path.splitext(name)[0].split("[")[0]
    changes = []
    reserved = reserved_names(licence, font)
    if reserved:
        dest = name
        shutil.copy(src, os.path.join(out, dest))
        changes.append(f"none, as it reserves its name ({reserved})")
    else:
        if axes:
            font = instancer.instantiateVariableFont(font, axes)
            changes.append("instanced at " + ", ".join(f"{k} {v}" for k, v in axes.items()))
        chars = {LETTERING: CHARS, HUD: HUD_CHARS, SYMBOLS: SYMBOL_CHARS}[use]
        cmap = font.getBestCmap()
        missing = [c for c in (SYMBOL_CHARS if use == SYMBOLS else CHARS) if ord(c) not in cmap]
        if missing:
            sys.exit(f"{folder}: no glyph for {''.join(missing)!r}")
        glyphs = sorted({cmap[ord(c)] for c in chars if ord(c) in cmap})
        pairs = pair_kerning(font, glyphs) if use != SYMBOLS else {}
        opts = subset.Options()
        opts.layout_features = []
        opts.drop_tables += ["GSUB", "GPOS", "GDEF", "BASE", "JSTF", "DSIG", "STAT"]
        opts.name_IDs = ["*"]
        opts.name_languages = ["*"]
        opts.glyph_names = False
        opts.hinting = use == HUD
        opts.legacy_kern = True
        sub = subset.Subsetter(opts)
        sub.populate(unicodes=[ord(c) for c in chars])
        sub.subset(font)
        if pairs:
            write_kern(font, pairs)
        what = {LETTERING: "the characters the game letters (text::CHARS)",
                HUD: "Latin-1 and common punctuation, for the HUD and lettering",
                SYMBOLS: " ".join(SYMBOL_CHARS)}[use]
        changes.append("subset to " + what)
        if use != HUD:
            changes.append("hinting dropped")
        if pairs:
            changes.append(f"its GPOS kerning written as a kern table ({len(pairs)} pairs)")
        dest = f"{stem.replace('-Regular', '')}-Subset.ttf"
        font.save(os.path.join(out, dest))
    with open(os.path.join(out, "SOURCE.txt"), "w", encoding="utf-8") as f:
        f.write(f"{family}{', by ' + designer if designer else ''}.\n")
        f.write(f"Licence: {'Apache License 2.0' if lic_name == 'LICENSE.txt' else 'SIL Open Font License 1.1'}"
                f" ({lic_name}).\n")
        f.write(f"From: {BLOB}{gdir}/{urllib.parse.quote(name, safe=',')}\n")
        f.write(f"Upstream: {upstream}\n")
        f.write(f"Changes: {'; '.join(changes)}.\n")
        f.write("Made by assets/fonts/subset.py.\n")
    size = os.path.getsize(os.path.join(out, dest))
    print(f"{folder:20} {dest:34} {size:8} bytes  {'; '.join(changes)}")


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    for face in FACES:
        build(*face, sys.argv[1])


main()
