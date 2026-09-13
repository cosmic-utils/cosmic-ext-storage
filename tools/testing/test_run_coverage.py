import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch

from run_coverage import collect_ui, digest, export_reports, gate, scope_export, unpack_lab, write_html


class ProfileArchiveTests(unittest.TestCase):
    def test_scope_classification_is_cached_per_filename_not_per_instantiation(self):
        dependency = dict(filenames=["/cargo/registry/dep/src/lib.rs"], count=1,
                          regions=[[1, 1, 2, 1, 1, 0, 0, 0]])
        ours = dependency | {"filenames": ["/repo/src/lib.rs"], "count": 0}
        report = dict(data=[dict(functions=[dependency] * 10_000 + [ours] * 100)])
        with patch.object(gate, "source_path", wraps=gate.source_path) as classify:
            scoped = scope_export(report, Path("/repo"))
        self.assertEqual(classify.call_count, 2)
        self.assertEqual(scoped["data"][0]["functions"], [ours] * 100)

    def test_scoped_export_preserves_exact_first_party_function_coverage(self):
        root = Path("/repo")
        files = ["/repo/src/app.rs", "/workspace/crates/test-backend/src/lib.rs",
                 "/opt/ui-test/source/crates/ui-e2e-runner/src/main.rs",
                 "/cargo/registry/dependency/src/lib.rs", "/repo/tests/unit.rs",
                 "/repo-other/src/lib.rs", "/repo/crates/storage-types/tests/contract.rs"]

        def function(filename, count, file_id=0):
            names = ["/cargo/registry/macro/src/lib.rs", filename] if file_id else [filename]
            return dict(name="same_generic_symbol", filenames=names, count=count,
                        regions=[[1, 2, 5, 8, count, file_id, 0, 0]])

        records = [function(filename, index % 2) for index, filename in enumerate(files)]
        # First-party code can be at a nonzero file ID. Conversely, merely
        # mentioning our file does not make a dependency definition ours.
        records += [function(files[0], 0, 1),
                    dict(name="dependency", filenames=[files[0], files[3]], count=999,
                         regions=[[1, 1, 3, 4, 999, 1, 0, 0]]),
                    dict(name="unmapped", filenames=[files[0]], count=0, regions=[])]
        document = dict(type="llvm.coverage.json.export", version="2.0.1",
                        data=[dict(files=[dict(filename=name, segments=[[1, 1, 0, True, True, False]])
                                         for name in files], functions=records)])
        original = json.dumps(document)
        scoped = scope_export(document, root)
        self.assertEqual(json.dumps(document), original)
        self.assertEqual(gate.read_functions(document, root), gate.read_functions(scoped, root))
        self.assertEqual([entry["filename"] for entry in scoped["data"][0]["files"]], files[:3])
        self.assertEqual(scoped["data"][0]["functions"], records[:3] + [records[7]])
        self.assertTrue(any(entry["count"] == 0 for entry in scoped["data"][0]["functions"]))
        self.assertEqual(scope_export(scoped, root), scoped)

    def test_export_persists_only_scoped_json_and_leaves_lcov_unchanged(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            output, run = root / "output", root / "run"
            output.mkdir()
            run.mkdir()
            source = root / "src/lib.rs"
            source.parent.mkdir()
            source.write_text("fn untested() {}\n")
            report = dict(data=[dict(files=[dict(filename=str(source))], functions=[
                dict(name="untested", filenames=[str(source)], count=0,
                     regions=[[1, 1, 1, 17, 0, 0, 0, 0]]),
                dict(name="dependency", filenames=["/cargo/dep/src/lib.rs"], count=999,
                     regions=[[1, 1, 1, 17, 999, 0, 0, 0]])])])
            lcov = f"SF:{source}\nDA:1,0\nend_of_record\n"

            def command(args, *, output=None, **kwargs):
                if output:
                    output.write_text(lcov if "-format=lcov" in args else json.dumps(report))
                return 0

            with patch("run_coverage.ROOT", root), patch("run_coverage.command", side_effect=command), \
                 patch.object(gate, "workspace_sources", return_value={"src/lib.rs": source}):
                export_reports(output, run, root / "llvm", [dict(profiles=[], objects=[])])
            expected = scope_export(report, root)["data"]
            for path in (run / "group-0.json", output / "summary.json"):
                self.assertEqual(json.loads(path.read_text())["data"], expected)
            self.assertEqual((output / "lcov.info").read_text(), lcov)
            self.assertIn('class="miss"', (output / "html/source-0.html").read_text())

    def test_ui_profiles_require_executed_steps_checkpoint_and_matching_elf(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            case = root / "case.toml"
            case.write_text('id = "live_scenario_reload"\n[[step]]\nid = "assertion"\n')
            source = root / "ui"
            (source / "profiles").mkdir(parents=True)
            (source / "objects").mkdir()
            for name in ("app-1.profraw", "runner-1.profraw"):
                (source / "profiles" / name).write_bytes(b"profile")
            for name in ("application", "runner"):
                (source / "objects" / name).write_bytes(name.encode())
            for name in ("Cargo.lock", "tools/ui-testing/environment.lock.toml", "tools/ui-testing/debug-app.py"):
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(name)
            result = dict(schema_version=2, functional_status="passed", status="semantic_passed", coverage_enabled=True,
                          case="live_scenario_reload", case_sha256=digest(case), completed_steps=["assertion"],
                          executable_sha256=digest(source / "objects/application"), runner_executable_sha256=digest(source / "objects/runner"),
                          cargo_lock_sha256=digest(root / "Cargo.lock"), environment_lock_sha256=digest(root / "tools/ui-testing/environment.lock.toml"),
                          debugger_script_sha256=digest(root / "tools/ui-testing/debug-app.py"))
            report = source / "execution.json"
            report.write_text(json.dumps(result))
            control = source / "control.json"
            control.write_text(json.dumps([dict(command="flush_coverage", response=dict(ok=dict(kind="coverage_flushed")))]))
            with patch("run_coverage.ROOT", root):
                group, records = collect_ui(source, root / "result", case)
                self.assertEqual(len(group["objects"]), 2)
                self.assertEqual(len(records), 2)
                self.assertTrue(all(entry["tests"] == ["live_scenario_reload"] for entry in records))
                for change in [dict(coverage_enabled=False), dict(completed_steps=[]), dict(functional_status="failed"),
                               dict(executable_sha256="wrong"), dict(runner_executable_sha256="wrong"), dict(cargo_lock_sha256="old")]:
                    report.write_text(json.dumps(result | change))
                    with self.subTest(change=change), self.assertRaises(ValueError):
                        collect_ui(source, root / "rejected", case)
                report.write_text(json.dumps(result))
                control.write_text("[]")
                with self.assertRaisesRegex(ValueError, "checkpoint"):
                    collect_ui(source, root / "rejected", case)

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
