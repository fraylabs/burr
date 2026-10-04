"""Exercise evidence refusals and byte preservation through the actual CLIs."""
import copy
import json
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import unittest

import cadquery as cq


SCRIPTS = Path(__file__).resolve().parent
MARKER = "Burr Caf\u00e9".encode('latin1')


def run_script(name, *args):
    return subprocess.run([sys.executable, str(SCRIPTS / name), *map(str, args)],
                          capture_output=True, text=True)


class ImportEvidence(unittest.TestCase):
    def test_missing_boundary_nearest_cannot_certify_a_carrier(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            reference = dict(model='authored', source_sha256='fixture', face=1,
                             surface=2, area=1, samples=[dict(normal=[0, 0, 1])],
                             boundary=[[0, 0, 0]])
            actual = dict(model='authored', face=1, losses=0, mesh=None,
                          samples=[dict(normal=[0, 0, 1], point_error=0,
                                        inverse_error=0, nearest_error=0)],
                          boundary=[dict(inverse_error=0, nearest_error=0)])
            (root / 'reference.json').write_text(json.dumps([reference]))
            (root / 'look.json').write_text(json.dumps([actual]))
            complete = run_script('compare_tori.py', root)
            self.assertEqual(complete.returncode, 0, complete.stderr)
            missing = copy.deepcopy(actual)
            missing['boundary'][0]['nearest_error'] = None
            (root / 'look.json').write_text(json.dumps([missing]))
            refused = run_script('compare_tori.py', root)
            self.assertNotEqual(refused.returncode, 0)
            row = json.loads((root / 'summary.json').read_text())[0]
            self.assertEqual(row['missing_boundary_nearest'], 1)
            self.assertFalse(row['analytic_agrees'])

    def source(self, shape, path):
        cq.exporters.export(shape, str(path), exportType='STEP')
        text = path.read_text(encoding='latin1')
        text = text.replace("FILE_DESCRIPTION(('", "FILE_DESCRIPTION(('Burr Caf\u00e9 ", 1)
        self.assertIn('Burr Caf\u00e9', text)
        return text

    def check_header(self, source, reductions):
        self.assertTrue(reductions, 'The CLI did not produce a face reduction')
        header = source.read_bytes().split(b'DATA;', 1)[0]
        self.assertIn(MARKER, header)
        for path in reductions:
            self.assertEqual(path.read_bytes().split(b'DATA;', 1)[0], header)

    def test_loop_reduction_preserves_latin1_source_header(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / 'box.step'
            text = self.source(cq.Solid.makeBox(1, 2, 3), source)
            def disorder(match):
                uses = re.findall(r'#\d+', match[2])
                self.assertGreaterEqual(len(uses), 3)
                uses[1], uses[2] = uses[2], uses[1]
                return match[1] + ','.join(uses) + match[3]
            text = re.sub(r"(EDGE_LOOP\('',\()(.*?)(\)\);)", disorder, text, count=1)
            source.write_text(text, encoding='latin1')
            output = root / 'loops'
            result = run_script('inspect_step_loops.py', '--model', source, '--output', output)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.check_header(source, list(output.glob('face-*.step')))

    def test_torus_reduction_preserves_latin1_source_header(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            models = root / 'models'
            models.mkdir()
            source = models / 'openamr-authored.STEP'
            text = self.source(cq.Solid.makeTorus(10, 2), source)
            text, count = re.subn(r"(TOROIDAL_SURFACE\([^;]*?,)\s*10\.,", r'\g<1>-10.,', text)
            self.assertGreater(count, 0)
            source.write_text(text, encoding='latin1')
            output = root / 'tori'
            result = run_script('validate_tori.py', '--corpus', root, '--output', output)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.check_header(source, list(output.glob('*-face-*.step')))


if __name__ == '__main__':
    unittest.main()
