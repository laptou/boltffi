from __future__ import annotations

import argparse
import importlib
import importlib.util
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from types import ModuleType
from typing import Callable

import pyperf


@dataclass(frozen=True)
class BenchmarkCase:
    canonical_name: str
    boltffi: Callable[[], object]
    uniffi: Callable[[], object]


@dataclass(frozen=True)
class LoadedSubjects:
    boltffi: ModuleType
    uniffi: ModuleType


@dataclass(frozen=True)
class SubjectFixtures:
    echo_string_small: str
    echo_string_1k: str
    echo_bytes_64k: bytes
    echo_vec_i32_10k: list[int]
    boltffi_i32_vec_10k: list[int]
    boltffi_i32_vec_100k: list[int]
    uniffi_i32_vec_10k: list[int]
    uniffi_i32_vec_100k: list[int]
    boltffi_f64_vec_10k: list[float]
    uniffi_f64_vec_10k: list[float]
    boltffi_directions_1k: list[object]
    boltffi_directions_10k: list[object]
    uniffi_directions_1k: list[object]
    uniffi_directions_10k: list[object]


class PythonBenchmarkHarness:
    def __init__(self, boltffi_site: Path, uniffi_directory: Path) -> None:
        self.subjects = LoadedSubjects(
            boltffi=self._load_boltffi_module(boltffi_site),
            uniffi=self._load_uniffi_module(uniffi_directory),
        )
        self.fixtures = SubjectFixtures(
            echo_string_small="hello",
            echo_string_1k="x" * 1000,
            echo_bytes_64k=bytes([42]) * 65536,
            echo_vec_i32_10k=list(range(10000)),
            boltffi_i32_vec_10k=self.subjects.boltffi.generate_i32_vec(10000),
            boltffi_i32_vec_100k=self.subjects.boltffi.generate_i32_vec(100000),
            uniffi_i32_vec_10k=self.subjects.uniffi.generate_i32_vec(10000),
            uniffi_i32_vec_100k=self.subjects.uniffi.generate_i32_vec(100000),
            boltffi_f64_vec_10k=self.subjects.boltffi.generate_f64_vec(10000),
            uniffi_f64_vec_10k=self.subjects.uniffi.generate_f64_vec(10000),
            boltffi_directions_1k=self.subjects.boltffi.generate_directions(1000),
            boltffi_directions_10k=self.subjects.boltffi.generate_directions(10000),
            uniffi_directions_1k=self.subjects.uniffi.generate_directions(1000),
            uniffi_directions_10k=self.subjects.uniffi.generate_directions(10000),
        )

    def selected_cases(self, include_pattern: str | None) -> tuple[BenchmarkCase, ...]:
        all_cases = self._cases()
        if include_pattern is None:
            return all_cases

        include_regex = re.compile(include_pattern)
        return tuple(case for case in all_cases if include_regex.search(case.canonical_name))

    def _cases(self) -> tuple[BenchmarkCase, ...]:
        boltffi = self.subjects.boltffi
        uniffi = self.subjects.uniffi
        fixtures = self.fixtures

        return (
            BenchmarkCase("noop", boltffi.noop, uniffi.noop),
            BenchmarkCase("echo_bool", lambda: boltffi.echo_bool(True), lambda: uniffi.echo_bool(True)),
            BenchmarkCase("negate_bool", lambda: boltffi.negate_bool(True), lambda: uniffi.negate_bool(True)),
            BenchmarkCase("echo_i32", lambda: boltffi.echo_i32(42), lambda: uniffi.echo_i32(42)),
            BenchmarkCase("echo_f64", lambda: boltffi.echo_f64(3.14159), lambda: uniffi.echo_f64(3.14159)),
            BenchmarkCase("add", lambda: boltffi.add(100, 200), lambda: uniffi.add(100, 200)),
            BenchmarkCase("add_f64", lambda: boltffi.add_f64(1.25, 2.5), lambda: uniffi.add_f64(1.25, 2.5)),
            BenchmarkCase("multiply", lambda: boltffi.multiply(2.5, 4.0), lambda: uniffi.multiply(2.5, 4.0)),
            BenchmarkCase("inc_u64_value", lambda: boltffi.inc_u64_value(0), lambda: uniffi.inc_u64_value(0)),
            BenchmarkCase(
                "echo_string_small",
                lambda: boltffi.echo_string(fixtures.echo_string_small),
                lambda: uniffi.echo_string(fixtures.echo_string_small),
            ),
            BenchmarkCase(
                "echo_string_1k",
                lambda: boltffi.echo_string(fixtures.echo_string_1k),
                lambda: uniffi.echo_string(fixtures.echo_string_1k),
            ),
            BenchmarkCase("generate_string_1k", lambda: boltffi.generate_string(1000), lambda: uniffi.generate_string(1000)),
            BenchmarkCase(
                "echo_bytes_64k",
                lambda: boltffi.echo_bytes(fixtures.echo_bytes_64k),
                lambda: uniffi.echo_bytes(fixtures.echo_bytes_64k),
            ),
            BenchmarkCase(
                "echo_vec_i32_10k",
                lambda: boltffi.echo_vec_i32(fixtures.echo_vec_i32_10k),
                lambda: uniffi.echo_vec_i32(fixtures.echo_vec_i32_10k),
            ),
            BenchmarkCase("generate_i32_vec_10k", lambda: boltffi.generate_i32_vec(10000), lambda: uniffi.generate_i32_vec(10000)),
            BenchmarkCase("generate_i32_vec_100k", lambda: boltffi.generate_i32_vec(100000), lambda: uniffi.generate_i32_vec(100000)),
            BenchmarkCase("generate_f64_vec_10k", lambda: boltffi.generate_f64_vec(10000), lambda: uniffi.generate_f64_vec(10000)),
            BenchmarkCase("generate_bytes_64k", lambda: boltffi.generate_bytes(65536), lambda: uniffi.generate_bytes(65536)),
            BenchmarkCase("sum_i32_vec_10k", lambda: boltffi.sum_i32_vec(fixtures.boltffi_i32_vec_10k), lambda: uniffi.sum_i32_vec(fixtures.uniffi_i32_vec_10k)),
            BenchmarkCase("sum_i32_vec_100k", lambda: boltffi.sum_i32_vec(fixtures.boltffi_i32_vec_100k), lambda: uniffi.sum_i32_vec(fixtures.uniffi_i32_vec_100k)),
            BenchmarkCase("sum_f64_vec_10k", lambda: boltffi.sum_f64_vec(fixtures.boltffi_f64_vec_10k), lambda: uniffi.sum_f64_vec(fixtures.uniffi_f64_vec_10k)),
            BenchmarkCase(
                "simple_enum",
                lambda: (boltffi.opposite_direction(boltffi.Direction.NORTH), boltffi.direction_to_degrees(boltffi.Direction.EAST)),
                lambda: (uniffi.opposite_direction(uniffi.Direction.NORTH), uniffi.direction_to_degrees(uniffi.Direction.EAST)),
            ),
            BenchmarkCase(
                "echo_direction",
                lambda: boltffi.echo_direction(boltffi.Direction.NORTH),
                lambda: uniffi.echo_direction(uniffi.Direction.NORTH),
            ),
            BenchmarkCase("generate_directions_1k", lambda: boltffi.generate_directions(1000), lambda: uniffi.generate_directions(1000)),
            BenchmarkCase("generate_directions_10k", lambda: boltffi.generate_directions(10000), lambda: uniffi.generate_directions(10000)),
            BenchmarkCase("count_north_1k", lambda: boltffi.count_north(fixtures.boltffi_directions_1k), lambda: uniffi.count_north(fixtures.uniffi_directions_1k)),
            BenchmarkCase("count_north_10k", lambda: boltffi.count_north(fixtures.boltffi_directions_10k), lambda: uniffi.count_north(fixtures.uniffi_directions_10k)),
        )

    def _load_boltffi_module(self, boltffi_site: Path) -> ModuleType:
        self._purge_demo_modules()
        sys.path.insert(0, str(boltffi_site))
        try:
            return importlib.import_module("demo")
        finally:
            sys.path.pop(0)

    def _load_uniffi_module(self, uniffi_directory: Path) -> ModuleType:
        module_path = uniffi_directory / "demo.py"
        module_spec = importlib.util.spec_from_file_location("demo_uniffi", module_path)
        if module_spec is None or module_spec.loader is None:
            raise RuntimeError(f"unable to load UniFFI Python module from {module_path}")

        module = importlib.util.module_from_spec(module_spec)
        module_spec.loader.exec_module(module)
        return module

    def _purge_demo_modules(self) -> None:
        stale_module_names = [module_name for module_name in sys.modules if module_name == "demo" or module_name.startswith("demo.")]
        for module_name in stale_module_names:
            sys.modules.pop(module_name, None)


def add_worker_args(command: list[str], harness_args: argparse.Namespace) -> None:
    command.extend(
        [
            "--boltffi-site",
            str(harness_args.boltffi_site),
            "--uniffi-dir",
            str(harness_args.uniffi_dir),
        ]
    )
    if harness_args.include:
        command.extend(["--include", harness_args.include])


def main() -> None:
    argument_parser = argparse.ArgumentParser(add_help=False)
    argument_parser.add_argument("--boltffi-site", type=Path, required=True)
    argument_parser.add_argument("--uniffi-dir", type=Path, required=True)
    argument_parser.add_argument("--include")
    harness_args, pyperf_args = argument_parser.parse_known_args()
    sys.argv = [sys.argv[0], *pyperf_args]

    harness = PythonBenchmarkHarness(
        boltffi_site=harness_args.boltffi_site.resolve(),
        uniffi_directory=harness_args.uniffi_dir.resolve(),
    )
    cases = harness.selected_cases(harness_args.include)
    if not cases:
        raise SystemExit("no Python benchmark cases matched the requested filter")

    runner = pyperf.Runner(add_cmdline_args=lambda command, _args: add_worker_args(command, harness_args))
    runner.metadata["suite_name"] = "python-bench"

    for case in cases:
        runner.bench_func(f"boltffi_{case.canonical_name}", case.boltffi)
        runner.bench_func(f"uniffi_{case.canonical_name}", case.uniffi)


if __name__ == "__main__":
    main()
