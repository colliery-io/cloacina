"""
Test-related functionality for Cloaca tasks.
"""

from dataclasses import dataclass
from typing import List, Optional

@dataclass
class TestResult:
    """Represents the result of running a test file."""
    file_name: str
    backend: str
    passed: bool
    stdout: str = ""
    stderr: str = ""
    return_code: Optional[int] = None


class TestAggregator:
    """Aggregates test results across all backends."""

    def __init__(self):
        self.results: List[TestResult] = []

    def add_result(self, result: TestResult):
        self.results.append(result)

    def get_failed_results(self) -> List[TestResult]:
        return [r for r in self.results if not r.passed]

    def get_summary(self) -> dict:
        total = len(self.results)
        failed = len(self.get_failed_results())
        passed = total - failed

        backends = {}
        for result in self.results:
            if result.backend not in backends:
                backends[result.backend] = {"passed": 0, "failed": 0}
            if result.passed:
                backends[result.backend]["passed"] += 1
            else:
                backends[result.backend]["failed"] += 1

        return {
            "total": total,
            "passed": passed,
            "failed": failed,
            "backends": backends
        }
