#!/usr/bin/env python3
"""Actualise les noms FR/EN des séries Riot, sans dépendance Python externe.

Usage : python3 -B scripts/update-skin-lines.py --patch 16.19
Les données restent versionnées ; aucune substitution de traduction n'est faite.
"""

import argparse
import json
import os
from pathlib import Path
import re
import tempfile
import unicodedata
import urllib.request


MAX_BYTES = 512 * 1024
DEFAULT_OUTPUT = (Path(__file__).resolve().parents[1]
                  / "apps/desktop/src/app/collection/skinLines.json")


def parse_lines(data: bytes) -> dict[int, str]:
    """Valide les identifiants publics et ignore seulement la sentinelle Riot vide."""
    if len(data) > MAX_BYTES:
        raise ValueError("Catalogue de séries trop volumineux")
    entries = json.loads(data.decode("utf-8"))
    if not isinstance(entries, list) or not 1 <= len(entries) <= 1000:
        raise ValueError("Liste de séries invalide")
    names = {}
    seen = set()
    for entry in entries:
        if not isinstance(entry, dict):
            raise ValueError("Série invalide")
        line_id, name = entry.get("id"), entry.get("name")
        if type(line_id) is not int or not 0 <= line_id <= 2**32 - 1 or line_id in seen:
            raise ValueError("Identifiant de série invalide ou dupliqué")
        seen.add(line_id)
        if line_id == 0 and name == "":
            continue
        if (line_id == 0 or not isinstance(name, str) or not name.strip()
                or len(name) > 512
                or any(unicodedata.category(char) == "Cc" for char in name)):
            raise ValueError("Nom de série invalide")
        names[line_id] = name
    if not names:
        raise ValueError("Catalogue sans série nommée")
    return names


def merge_lines(english: dict[int, str], french: dict[int, str]) -> list[dict]:
    if not english or english.keys() != french.keys():
        raise ValueError("Les séries FR et EN ne couvrent pas les mêmes identifiants")
    return [{"id": line_id, "names": {"fr": french[line_id], "en": english[line_id]}}
            for line_id in sorted(english)]


def fetch_bytes(url: str) -> bytes:
    request = urllib.request.Request(url, headers={
        "User-Agent": "OpenLoLCompanion-Catalog/1.0", "Accept": "application/json",
    })
    with urllib.request.urlopen(request, timeout=30) as response:
        data = response.read(MAX_BYTES + 1)
    if len(data) > MAX_BYTES:
        raise ValueError("Réponse de séries trop volumineuse")
    return data


def update_catalog(output: Path, patch: str, fetch=fetch_bytes) -> None:
    if not re.fullmatch(r"[1-9][0-9]*\.(?:0|[1-9][0-9]*)", patch):
        raise ValueError("Utiliser une version explicite, par exemple 16.19")
    base = f"https://raw.communitydragon.org/{patch}/plugins/rcp-be-lol-game-data/global"
    sources = [f"{base}/default/v1/skinlines.json", f"{base}/fr_fr/v1/skinlines.json"]
    english, french = (parse_lines(fetch(source)) for source in sources)
    result = {"patch": patch, "sources": sources, "entries": merge_lines(english, french)}
    encoded = json.dumps(result, ensure_ascii=False, indent=2) + "\n"
    # Le temporaire est sur le même volume et n'est créé qu'après validation complète.
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = None
    try:
        with tempfile.NamedTemporaryFile(mode="w", encoding="utf-8", newline="\n",
                                         dir=output.parent, prefix=f".{output.name}.",
                                         suffix=".tmp", delete=False) as stream:
            temporary = Path(stream.name)
            stream.write(encoded)
            stream.flush()
            os.fsync(stream.fileno())
        temporary.chmod(0o644)
        os.replace(temporary, output)
    finally:
        if temporary is not None:
            temporary.unlink(missing_ok=True)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--patch", default="16.19")
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    options = parser.parse_args()
    try:
        update_catalog(options.output, options.patch)
    except (ValueError, OSError) as error:
        parser.exit(1, f"Catalogue inchangé : {error}\n")
    print(f"Catalogue FR/EN mis à jour : {options.output}")


if __name__ == "__main__":
    main()
