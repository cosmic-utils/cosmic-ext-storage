import io
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch

from run_coverage import unpack_lab, write_html


class ProfileArchiveTests(unittest.TestCase):
    def test_html_reports_the_union_and_escapes_source_text(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "src/lib.rs"
            source.parent.mkdir()
            source.write_text('let text = "<script>alert(1)</script>";\nuntested();\n')
            output = root / "report"
            output.mkdir()
            with patch("run_coverage.ROOT", root):
                write_html(output, {"src/lib.rs": {1: 2, 2: 0}}, {"src/lib.rs": {(1, 2, "1:1"): 1}})
            index = (output / "html/index.html").read_text()
            page = (output / "html/source-0.html").read_text()
            self.assertIn("1/2", index)
            self.assertIn('id="L1" class="hit"', page)
            self.assertIn('id="L2" class="miss"', page)
            self.assertIn("&lt;script&gt;", page)
            self.assertNotIn("<script>", page)

    def archive(self, root, entries):
        archive = root / "archive.tar.gz"
        with tarfile.open(archive, "w:gz") as target:
            for name, kind in entries:
                info = tarfile.TarInfo(name)
                info.type = kind
                info.size = 4 if kind == tarfile.REGTYPE else 0
                target.addfile(info, io.BytesIO(b"data") if info.size else None)
        return archive

    def test_profiles_require_the_matching_baked_executable(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            archive = self.archive(root, [("tmp/storage-lab-profiles/test.profraw", tarfile.REGTYPE),
                                          ("opt/storage-lab/bin/storage-lab-images", tarfile.REGTYPE)])
            profiles, executable = unpack_lab(archive, root / "result", "images")
            self.assertEqual([path.read_bytes() for path in profiles], [b"data"])
            self.assertEqual(executable.read_bytes(), b"data")

    def test_archive_cannot_escape_or_smuggle_links_or_wrong_objects(self):
        for entries in [[("../escape", tarfile.REGTYPE)], [("/absolute", tarfile.REGTYPE)],
                        [("tmp/storage-lab-profiles/test.profraw", tarfile.SYMTYPE)],
                        [("opt/storage-lab/bin/storage-lab-wrong", tarfile.REGTYPE)],
                        [("tmp/storage-lab-profiles/test.profraw", tarfile.REGTYPE)]]:
            with self.subTest(entries=entries), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                archive = self.archive(root, entries)
                with self.assertRaises(ValueError):
                    unpack_lab(archive, root / "result", "images")
                self.assertFalse((root.parent / "escape").exists())


if __name__ == "__main__":
    unittest.main()
