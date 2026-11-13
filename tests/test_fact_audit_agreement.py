# Copyright 2026 Ryan Duguid
# SPDX-License-Identifier: FSL-1.1-ALv2

import io
import json
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stdout
from pathlib import Path
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "evals"))

import fact_audit_agreement as faa  # noqa: E402


def unit(group, kept):
    return {"group": group, "surface": "x", "kept": kept}


def judged(tool, sample, units, cycle="2026-10", kind="commercial"):
    return {"set": kind, "tool": tool, "aihb_cycle": cycle, "sample_id": sample, "units": units}


class FactAuditAgreementTest(unittest.TestCase):
    def test_flagged_reads_the_paper_kinds_and_meaning_words(self):
        report = (
            "dropped number: $1,200\ndropped quotation: we will not proceed\n"
            "dropped meaning word: 'not' (1 before, 0 after)\ncheck name: Priya Nair\n"
            "added number: 4\ncheck added name: Jane Smith\nlength: 9 to 5 words (56% of the original)\n"
        )
        self.assertEqual({"num", "quote", "meaning", "ent"}, faa.flagged(report))
        self.assertEqual({"num"}, faa.flagged("dropped quantity: 25 °C\n"))
        self.assertEqual(set(), faa.flagged("No tracked items dropped; meaning not checked.\nadded number: 4\n"))

    def test_judged_documents_keep_only_the_cycle_commercial_rows(self):
        rows = [
            judged("a", "s01", [unit("num", False), unit("quote", True)]),
            judged("a", "s02", []),
            judged("a", "s03", [unit("num", False)], cycle="2026-09"),
            judged(None, "p1", [unit("num", False)], kind="open-12b"),
        ]
        self.assertEqual({("a", "s01"): {"num"}, ("a", "s02"): set()}, faa.judged_documents(rows))

    def test_tally_counts_catches_and_failures_on_kept_documents(self):
        counts = faa.tally(
            [
                ({"num", "ent"}, True, {"num", "meaning"}),
                ({"quote"}, True, {"meaning"}),
                (set(), False, set()),
                (set(), True, {"meaning"}),
                (set(), True, {"quote", "meaning"}),
            ]
        )
        self.assertEqual(2, counts["altered"])
        self.assertEqual(2, counts["altered failed"])
        self.assertEqual((1, 1), (counts["num altered"], counts["num flagged"]))
        self.assertEqual((1, 0), (counts["quote altered"], counts["quote flagged"]))
        self.assertEqual((1, 0), (counts["ent altered"], counts["ent flagged"]))
        self.assertEqual(3, counts["kept"])
        self.assertEqual(1, counts["kept passed"])
        self.assertEqual(1, counts["kept failed on meaning words only"])
        self.assertEqual(1, counts["kept failed on numbers or quotations"])
        text = faa.table(counts)
        self.assertIn("| Any unit altered | 2 | 2 fail |", text)
        self.assertIn("| Numbers altered | 1 | 1 report a dropped number or quantity |", text)
        self.assertIn("| Names altered | 1 | 0 list a name to check |", text)
        self.assertIn("| Every unit kept | 3 | 1 pass; 1 fail only on meaning words; 1 fail on a number", text)

    def test_run_compares_each_pair_and_lists_the_differences(self):
        with tempfile.TemporaryDirectory() as tmp:
            cache = Path(tmp)
            (cache / "fact_units.jsonl").write_text(
                "\n".join(
                    json.dumps(row)
                    for row in [
                        judged("t", "s01", [unit("num", False)]),
                        judged("t", "s02", [unit("num", True)]),
                    ]
                ),
                encoding="utf-8",
            )
            (cache / "2026-10").mkdir()
            (cache / "2026-10" / "t.jsonl").write_text(
                "\n".join(
                    json.dumps({"humanizer": "t", "sample_id": sample, "input": source, "output": rewrite})
                    for sample, source, rewrite in [
                        ("s01", "Pay $500.", "Pay $400."),
                        ("s02", "It may rain on 3 May.", "Rain on 3 May."),
                    ]
                ),
                encoding="utf-8",
            )
            seen = []

            def check(binary, source, rewrite):
                seen.append((binary, source, rewrite))
                return (True, {"num"}) if "$" in source else (True, {"quote"})

            counts, differences = faa.run("lc", cache, check)
        self.assertEqual(
            [("lc", "Pay $500.", "Pay $400."), ("lc", "It may rain on 3 May.", "Rain on 3 May.")], seen
        )
        self.assertEqual((1, 1), (counts["num altered"], counts["num flagged"]))
        self.assertEqual(["t s02: missed [], extra ['quote']"], differences)

    def test_run_refuses_a_judged_document_without_a_rewrite(self):
        with tempfile.TemporaryDirectory() as tmp:
            cache = Path(tmp)
            (cache / "fact_units.jsonl").write_text(json.dumps(judged("t", "s01", [])), encoding="utf-8")
            (cache / "2026-10").mkdir()
            (cache / "2026-10" / "t.jsonl").write_text("", encoding="utf-8")
            with self.assertRaisesRegex(RuntimeError, "no benchmark rewrite"):
                faa.run("lc", cache, lambda *args: (False, set()))

    def test_preserve_reads_the_report_and_refuses_a_usage_error(self):
        dropped = subprocess.CompletedProcess(args=["x"], returncode=1, stdout="dropped number: -3%\n", stderr="")
        with mock.patch.object(faa.subprocess, "run", return_value=dropped) as called:
            self.assertEqual((True, {"num"}), faa.preserve("lc", "Growth was -3%.", "Growth was 3%."))
        self.assertEqual(["lc", "preserve"], called.call_args.args[0][:2])
        broken = subprocess.CompletedProcess(args=["x"], returncode=2, stdout="", stderr="usage")
        with mock.patch.object(faa.subprocess, "run", return_value=broken):
            with self.assertRaisesRegex(RuntimeError, "exited 2: usage"):
                faa.preserve("lc", "a", "b")

    def test_fetch_downloads_once_into_the_cache(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp, "sub", "file.jsonl")
            response = mock.MagicMock()
            response.__enter__.return_value.read.return_value = b"{}\n"
            with mock.patch.object(faa.urllib.request, "urlopen", return_value=response) as opened:
                faa.fetch("https://example.com/f", path)
                faa.fetch("https://example.com/f", path)
            self.assertEqual(1, opened.call_count)
            self.assertEqual("{}\n", path.read_text(encoding="utf-8"))

    def test_main_prints_the_table_and_the_differences(self):
        counts = faa.tally([({"num"}, True, {"num"})])
        with mock.patch.object(faa, "run", return_value=(counts, ["t s01: missed ['num'], extra []"])):
            out = io.StringIO()
            with redirect_stdout(out):
                self.assertEqual(0, faa.main(["--laconic-check", "lc", "--cache", "c"]))
        self.assertIn("| Any unit altered | 1 | 1 fail |", out.getvalue())
        self.assertIn("differ on numbers or quotations: 1\n- t s01: missed ['num'], extra []", out.getvalue())

    def test_the_pins_are_full_commits(self):
        for pin in (faa.AIHB_REVISION, faa.FACT_AUDIT_COMMIT):
            self.assertRegex(pin, r"^[0-9a-f]{40}$")


if __name__ == "__main__":
    unittest.main()
