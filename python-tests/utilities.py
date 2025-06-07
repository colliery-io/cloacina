"""
Test Utilities for Cloaca Testing

This module provides utilities for aggregating test failures and reporting them
at the end of test execution for better visibility.
"""

import traceback
from typing import List, Dict, Any, Optional
from dataclasses import dataclass, field


@dataclass
class FailureRecord:
    """Represents a single test failure."""
    test_name: str
    failure_type: str
    error_message: str
    traceback: Optional[str] = None
    context: Optional[Dict[str, Any]] = None


@dataclass
class SectionRecord:
    """Represents a section of tests within a scenario."""
    section_name: str
    passed: bool = True
    error_message: Optional[str] = None
    details: Optional[str] = None


class ResultsAggregator:
    """Aggregates test results and failures for end-of-test reporting."""
    
    def __init__(self, test_name: str):
        self.test_name = test_name
        self.failures: List[FailureRecord] = []
        self.sections: List[SectionRecord] = []
        self.total_sections = 0
        self.passed_sections = 0
        
    def add_section(self, section_name: str, passed: bool = True, 
                   error_message: Optional[str] = None, 
                   details: Optional[str] = None) -> None:
        """Add a test section result."""
        section = SectionRecord(
            section_name=section_name,
            passed=passed,
            error_message=error_message,
            details=details
        )
        self.sections.append(section)
        self.total_sections += 1
        if passed:
            self.passed_sections += 1
            
    def add_failure(self, test_name: str, failure_type: str, 
                   error_message: str, context: Optional[Dict[str, Any]] = None) -> None:
        """Add a test failure."""
        failure = FailureRecord(
            test_name=test_name,
            failure_type=failure_type,
            error_message=error_message,
            traceback=traceback.format_exc(),
            context=context
        )
        self.failures.append(failure)
        
    def run_test_section(self, section_name: str, test_func, *args, **kwargs):
        """Run a test section and capture any failures."""
        try:
            print(f"Testing {section_name}...")
            test_func(*args, **kwargs)
            self.add_section(section_name, passed=True)
            print(f"✓ {section_name} works correctly")
            return True
        except Exception as e:
            error_msg = str(e)
            self.add_section(section_name, passed=False, error_message=error_msg)
            self.add_failure(section_name, type(e).__name__, error_msg)
            print(f"✗ {section_name} failed: {error_msg}")
            return False
            
    def assert_with_context(self, condition: bool, message: str, 
                           context: Optional[Dict[str, Any]] = None) -> None:
        """Assert with context information for better failure reporting."""
        if not condition:
            self.add_failure("assertion", "AssertionError", message, context)
            raise AssertionError(message)
            
    def soft_assert(self, condition: bool, message: str, 
                   context: Optional[Dict[str, Any]] = None) -> bool:
        """Soft assertion that doesn't raise but records failure."""
        if not condition:
            self.add_failure("soft_assertion", "SoftAssertionError", message, context)
            return False
        return True
        
    def report_results(self) -> None:
        """Report aggregated test results at the end."""
        print("\n" + "="*80)
        print(f"TEST SUMMARY: {self.test_name}")
        print("="*80)
        
        # Section summary
        print(f"\nSections: {self.passed_sections}/{self.total_sections} passed")
        
        if self.sections:
            print("\nSection Results:")
            for section in self.sections:
                status = "✓" if section.passed else "✗"
                print(f"  {status} {section.section_name}")
                if not section.passed and section.error_message:
                    print(f"    Error: {section.error_message}")
                    
        # Detailed failure report
        if self.failures:
            print(f"\n🚨 FAILURES DETECTED ({len(self.failures)} total)")
            print("-" * 60)
            
            for i, failure in enumerate(self.failures, 1):
                print(f"\nFAILURE #{i}: {failure.test_name}")
                print(f"Type: {failure.failure_type}")
                print(f"Message: {failure.error_message}")
                
                if failure.context:
                    print("Context:")
                    for key, value in failure.context.items():
                        print(f"  {key}: {value}")
                        
                if failure.traceback and failure.failure_type != "SoftAssertionError":
                    print("Traceback:")
                    print(failure.traceback)
                    
                print("-" * 40)
        else:
            print("\n🎉 ALL TESTS PASSED!")
            
        print("\n" + "="*80)
        
    def get_success_rate(self) -> float:
        """Get the success rate as a percentage."""
        if self.total_sections == 0:
            return 100.0
        return (self.passed_sections / self.total_sections) * 100
        
    def has_failures(self) -> bool:
        """Check if there are any failures."""
        return len(self.failures) > 0 or self.passed_sections < self.total_sections
        
    def raise_if_failures(self) -> None:
        """Raise an exception if there are failures (for pytest compatibility)."""
        if self.has_failures():
            failure_summary = f"{len(self.failures)} failures, {self.total_sections - self.passed_sections} failed sections"
            raise AssertionError(f"Test aggregation failed: {failure_summary}")


def create_test_aggregator(test_name: str) -> ResultsAggregator:
    """Factory function to create a test aggregator."""
    return ResultsAggregator(test_name)