import unittest

import demo


class BytesTests(unittest.TestCase):
    def test_echo_bytes(self) -> None:
        self.assertEqual(demo.echo_bytes(bytes([1, 2, 3, 4])), bytes([1, 2, 3, 4]))

    def test_bytes_length(self) -> None:
        self.assertEqual(demo.bytes_length(bytes([10, 20, 30])), 3)

    def test_bytes_sum(self) -> None:
        self.assertEqual(demo.bytes_sum(bytes([1, 2, 3, 4])), 10)

    def test_make_bytes(self) -> None:
        self.assertEqual(demo.make_bytes(5), bytes([0, 1, 2, 3, 4]))

    def test_reverse_bytes(self) -> None:
        self.assertEqual(demo.reverse_bytes(bytes([5, 6, 7])), bytes([7, 6, 5]))
