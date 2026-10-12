#!/bin/sh
# Packs the built characters (out/<id>/) for the store, in store/: the scenes once each
# (escena-<scene>.iris), and per character its body piece (<id>-body.iris) and its buddy
# (<id>.iris), whose recipe uses them as `escena-<scene>@1` and `<id>-body@1`. Upload order:
# the scene (if it isn't up yet), the body, the buddy (the store takes a buddy whose pieces are
# yours and still in review, and approves them together).
#   ./pack-store.sh            (all built ones)   ./pack-store.sh pikachu slime
set -eu
cd "$(dirname "$0")"
mkdir -p store
ids=${*:-$(for d in out/*/; do b=$(basename "$d"); [ "$b" != _scenes ] && [ -f "$d/recipe.toml" ] && echo "$b"; done)}
python3 - store $ids <<'PY'
import io, os, re, sys, zipfile
dest, ids = sys.argv[1], sys.argv[2:]
SCENES = {
    "dreamland": ("escena-dreamland", "Dream Land: estrellas en órbita y cielo que titila"),
    "meadow": ("escena-pradera", "Una pradera: pétalos que caen, órbita de estrellas"),
    "ocean": ("escena-oceano", "El fondo del mar: burbujas que suben, órbita de anillos"),
    "snow": ("escena-nieve", "Nieve que cae despacio, órbita de copos"),
    "volcano": ("escena-volcan", "Un volcán: brasas que suben, órbita de diamantes"),
    "space": ("escena-espacio", "El espacio: estrellas que titilan, órbita de anillos"),
    "forest": ("escena-bosque", "Un bosque: hojas que caen balanceándose"),
    "city": ("escena-ciudad", "Una ciudad de neón bajo la lluvia"),
    "code": ("escena-codigo", "Código que cae como lluvia verde"),
    "candy": ("escena-dulces", "Un mundo de dulces: destellos de colores, órbita de corazones"),
    "desert": ("escena-desierto", "Un desierto: arena que vuela baja"),
    "cave": ("escena-cueva", "Una cueva de cristales con luciérnagas"),
    "spooky": ("escena-tenebroso", "Un lugar tenebroso con fuegos fatuos"),
    "sakura": ("escena-sakura", "Cerezos en flor: pétalos rosados, órbita de corazones"),
}
def manifest(name, kind, desc, fmt=None):
    return "[package]\n" + f'name = "{name}"\nkind = "{kind}"\n' + (f'format = "{fmt}"\n' if fmt else "") + f'version = "1.0.0"\ndescription = "{desc}"\nlicense = "MIT"\n'
def write(path, files):
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as z:
        for n, b in files.items():
            z.writestr(n, b)
for id_ in ids:
    d = f"out/{id_}"
    recipe = open(f"{d}/recipe.toml").read()
    scene = re.search(r'in its\n# scene \(stage\.wasm, "([a-z]+)"\)', recipe).group(1)
    name = re.search(r"^# (.+?): a pixel-art character", recipe, re.M).group(1)
    sname, sdesc = SCENES[scene]
    os.makedirs(f"{dest}/{id_}", exist_ok=True)
    spath = f"{dest}/{sname}.iris"
    if not os.path.exists(spath):
        write(spath, {"iris.toml": manifest(sname, "piece", sdesc, "wasm"), "piece.wasm": open(f"{d}/stage.wasm", "rb").read()})
    body = f"{id_}-body"
    write(f"{dest}/{id_}/2-{body}.iris", {"iris.toml": manifest(body, "piece", f"El cuerpo de {name}, en pixel art y en código", "wasm"), "piece.wasm": open(f"{d}/sprite.wasm", "rb").read()})
    r = recipe.replace('use = "file:sprite.wasm"', f'use = "{body}@1"').replace('use = "file:stage.wasm"', f'use = "{sname}@1"')
    desc = re.search(r'^description = "(.*)"$', recipe, re.M).group(1)
    write(f"{dest}/{id_}/3-{id_}.iris", {"iris.toml": manifest(id_, "buddy", desc), "recipe.toml": r})
    open(f"{dest}/{id_}/1-escena.txt", "w").write(f"Escenario: {sname} (store/{sname}.iris) — súbelo primero si todavía no está en la tienda.\n")
    print(f"{id_}: {sname} → {body} → {id_}")
PY
