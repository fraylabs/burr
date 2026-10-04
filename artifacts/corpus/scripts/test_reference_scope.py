"""Ensure a completed subset scan cannot certify an unchecked Burr pair."""
import unittest
import cadquery as cq
from compare_pairs import source_bounds, validate_reference_scope


class ReferenceScope(unittest.TestCase):
    def setUp(self):
        self.reference = dict(pair_check_scope='reported_pairs', checked_pairs=[[1, 2], [3, 4]],
                              findings=[dict(pair=[1, 2])])

    def test_reported_subset(self):
        self.assertEqual(validate_reference_scope(self.reference, {(1, 2)}, True), 'reported_pairs')

    def test_subset_cannot_certify_full_scan(self):
        with self.assertRaisesRegex(ValueError, 'requires --reported-pairs-only'):
            validate_reference_scope(self.reference, {(1, 2)}, False)

    def test_new_pair_requires_new_boolean(self):
        with self.assertRaisesRegex(ValueError, 'did not check every reported pair'):
            validate_reference_scope(self.reference, {(1, 2), (5, 6)}, True)

    def test_unchecked_positive_is_rejected(self):
        self.reference['findings'].append(dict(pair=[5, 6]))
        with self.assertRaisesRegex(ValueError, 'unchecked positive'):
            validate_reference_scope(self.reference, {(1, 2)}, True)

    def test_legacy_full_reference(self):
        self.assertEqual(validate_reference_scope(dict(findings=[]), {(5, 6)}, False), 'all_pairs')

    def test_unknown_scope_is_rejected(self):
        with self.assertRaisesRegex(ValueError, 'Unknown'):
            validate_reference_scope(dict(pair_check_scope='unknown'), set(), True)


class SourceBounds(unittest.TestCase):
    def test_empty_occt_occurrence_is_an_explicit_refusal(self):
        with self.assertRaisesRegex(ValueError, "OCCT source occurrence 17.*empty.*no usable bounding box"):
            source_bounds(cq.Compound.makeCompound([]), 17, 'empty')

    def test_finite_bounds_keep_the_step_to_scene_axis_conversion(self):
        self.assertEqual(source_bounds(cq.Solid.makeBox(1, 2, 3), 0, 'box'),
                         [0, 0, -2, 1, 3, 0])


if __name__ == '__main__':
    unittest.main()
