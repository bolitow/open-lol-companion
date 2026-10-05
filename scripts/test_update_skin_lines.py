"""Contrats du catalogue : identité, traduction complète et remplacement sûr."""

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest


SPEC = importlib.util.spec_from_file_location(
    "update_skin_lines", Path(__file__).with_name("update-skin-lines.py")
)
catalog = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(catalog)


def payload(entries):
    return json.dumps(entries, ensure_ascii=False).encode("utf-8")


class SkinLinesTests(unittest.TestCase):
    def test_parse_excludes_empty_riot_sentinel_and_preserves_names(self):
        result = catalog.parse_lines(payload([
            {"id": 0, "name": "", "description": ""},
            {"id": 20, "name": "Gardiens des étoiles (saison 2)"},
        ]))
        self.assertEqual(result, {20: "Gardiens des étoiles (saison 2)"})

    def test_refuses_duplicates_even_when_identical(self):
        for line_id in (0, 20):
            line = {"id": line_id, "name": "" if line_id == 0 else "Arcade"}
            with self.subTest(line_id=line_id), self.assertRaises(ValueError):
                catalog.parse_lines(payload([line, line]))

    def test_rejects_bad_ids_names_shapes_and_unbounded_input(self):
        invalid = [[], {}, [None], [{"id": -1, "name": "A"}],
                   [{"id": True, "name": "A"}], [{"id": "20", "name": "A"}],
                   [{"id": 1.5, "name": "A"}], [{"id": 2**32, "name": "A"}],
                   [{"id": 0, "name": "Invented"}], [{"id": 20, "name": ""}],
                   [{"id": 20, "name": "   "}], [{"id": 20, "name": "A\nB"}],
                   [{"id": 20, "name": "A" * 513}], [{"id": 20}],
                   [{"id": i + 1, "name": "A"} for i in range(1001)]]
        for value in invalid:
            with self.subTest(value=str(value)[:80]), self.assertRaises(ValueError):
                catalog.parse_lines(payload(value))
        for value in (b"x" * 524289, b"not json", b"\xff"):
            with self.assertRaises(ValueError):
                catalog.parse_lines(value)

    def test_merge_uses_ids_and_sorts_without_name_fallback(self):
        self.assertEqual(catalog.merge_lines({20: "Stars", 10: "Arcade"},
                                             {10: "Arcade FR", 20: "Étoiles"}), [
            {"id": 10, "names": {"fr": "Arcade FR", "en": "Arcade"}},
            {"id": 20, "names": {"fr": "Étoiles", "en": "Stars"}},
        ])
        with self.assertRaises(ValueError):
            catalog.merge_lines({10: "Arcade", 20: "Stars"}, {10: "Arcade FR"})

    def test_update_replaces_only_after_both_sources_validate(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "catalog.json"
            output.write_text("existing", encoding="utf-8")
            def broken(url):
                return payload([{"id": 10, "name": "Arcade"}]) if "/default/" in url else b"broken"
            with self.assertRaises(ValueError):
                catalog.update_catalog(output, "16.19", broken)
            self.assertEqual(output.read_text(encoding="utf-8"), "existing")
            self.assertEqual([p.name for p in Path(directory).iterdir()], ["catalog.json"])
            def valid(url):
                return payload([{"id": 10, "name": "Arcade FR" if "/fr_fr/" in url else "Arcade"}])
            catalog.update_catalog(output, "16.19", valid)
            result = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(result["patch"], "16.19")
            self.assertEqual(result["entries"], [{"id": 10, "names": {"fr": "Arcade FR", "en": "Arcade"}}])
            self.assertEqual(len(result["sources"]), 2)
            self.assertTrue(all("/16.19/" in source for source in result["sources"]))

    def test_rejects_unversioned_or_path_like_patch_before_fetching(self):
        def forbidden(_):
            self.fail("Une version invalide ne doit déclencher aucun téléchargement")
        for patch in ("latest", "../16.19", "16.19/", "16", "016.19"):
            with self.subTest(patch=patch), self.assertRaises(ValueError):
                catalog.update_catalog(Path("unused"), patch, forbidden)


if __name__ == "__main__":
    unittest.main()
