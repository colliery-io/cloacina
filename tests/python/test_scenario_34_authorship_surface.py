"""
Test the cloaca AUTHORSHIP surface (CLOACI-I-0137).

Exercises the declared-metadata decorators and helpers that a packaged Python
workflow uses and the compiler parses from source — `workflow_params`,
`workflow_secrets`, `boundary_schema` (runtime no-op pass-throughs), and the
`cloaca.var` / `var_or` registry.

WHY THIS EXISTS: these authorship symbols had silently drifted between the pip
wheel (`#[pymodule]`) and the server's synthetic `cloaca` module precisely
because NO example or test ever used them — the gap could not fail loud. This
test imports the wheel and USES each symbol, so a future omission from the
authorship contract fails here instead of as an obscure `AttributeError` when a
real packaged workflow is loaded. It pairs with the Rust-side
`test_ensure_cloaca_module_registers_in_sys_modules` contract test, which checks
the server's synthetic module.

Authoring-only (no runner needed): the decorators are runtime no-ops, so we
assert the pass-through behavior rather than executing a workflow.
"""

import pytest


class TestAuthorshipSurface:
    """Exercise the declared-metadata authoring decorators + var registry."""

    def test_workflow_params_decorator_is_passthrough(self):
        import cloaca

        @cloaca.workflow_params(retries=int, region=(str, "us-east-1"))
        def my_workflow():
            return "wf"

        # Runtime no-op — the compiler parses the declaration from source; at
        # runtime the decorator must return the decorated object unchanged.
        assert my_workflow() == "wf"

    def test_workflow_secrets_decorator_is_passthrough(self):
        import cloaca

        @cloaca.workflow_secrets("db_prod", "api_token")
        def my_workflow():
            return "wf"

        assert my_workflow() == "wf"

    def test_boundary_schema_decorator_is_passthrough(self):
        import cloaca

        @cloaca.boundary_schema(bid=float, ask=float)
        def my_accumulator(event):
            return event

        assert my_accumulator("event") == "event"

    def test_combined_params_and_secrets(self):
        """A packaged workflow commonly declares both typed params and secrets."""
        import cloaca

        @cloaca.workflow_secrets("db_prod")
        @cloaca.workflow_params(threshold=(float, 0.5))
        def my_workflow():
            return 42

        assert my_workflow() == 42

    def test_var_registry(self):
        import cloaca

        # var_or falls back to the default when the variable is unset.
        assert (
            cloaca.var_or("CLOACA_TEST_DEFINITELY_MISSING_XYZ", "fallback")
            == "fallback"
        )
        # var raises KeyError when the variable is unset (no silent None).
        with pytest.raises(KeyError):
            cloaca.var("CLOACA_TEST_DEFINITELY_MISSING_XYZ")
