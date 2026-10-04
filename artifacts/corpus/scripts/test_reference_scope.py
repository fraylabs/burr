"""Ensure a completed subset scan cannot certify an unchecked Burr pair."""
import unittest
from compare_pairs import validate_reference_scope


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


if __name__ == '__main__':
    unittest.main()
