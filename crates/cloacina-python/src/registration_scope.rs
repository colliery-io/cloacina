/*
 *  Copyright 2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

//! Thread-local "who is importing right now" slot (CLOACI-T-0921).
//!
//! [`crate::runtime_scope::ScopedRuntime`] tells Python decorators *which
//! `Runtime`* to register into. This is its tenancy counterpart: it tells them
//! *whose* registration this is, so the process-global Python registries
//! (`GRAPH_EXECUTORS`, `ACCUMULATOR_REGISTRY`) can be keyed by
//! `(tenant, package, name)` instead of a bare name.
//!
//! Without it, two tenants each shipping a graph called `pipeline` share one
//! executor slot and the second import silently wins for both — the Python-side
//! form of the same doctrine breach CLOACI-T-0921 fixes in `EndpointRegistry`.
//!
//! The loader installs a [`ScopedRegistration`] around each package import.
//! Unscoped registration (embedded use, `cloaca` imported directly in a REPL,
//! unit tests) yields the default all-`None` scope, which behaves exactly as
//! the pre-T-0921 bare-name registry did.

use std::cell::RefCell;

/// Identity of the package whose import is currently running on this thread.
///
/// `None` fields mean "unknown / not packaged" — the embedded and test paths.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct RegistrationScope {
    pub tenant_id: Option<String>,
    pub package: Option<String>,
}

impl RegistrationScope {
    pub fn new(tenant_id: Option<&str>, package: Option<&str>) -> Self {
        Self {
            tenant_id: tenant_id.map(|s| s.to_string()),
            package: package.map(|s| s.to_string()),
        }
    }

    /// The unscoped (embedded / direct-import / test) identity.
    pub fn unscoped() -> Self {
        Self::default()
    }

    /// True when neither tenant nor package is known.
    pub fn is_unscoped(&self) -> bool {
        self.tenant_id.is_none() && self.package.is_none()
    }
}

thread_local! {
    static CURRENT_SCOPE: RefCell<RegistrationScope> =
        const { RefCell::new(RegistrationScope { tenant_id: None, package: None }) };
}

/// The registration scope installed on this thread (all-`None` if unscoped).
pub fn current_registration_scope() -> RegistrationScope {
    CURRENT_SCOPE.with(|slot| slot.borrow().clone())
}

/// RAII guard installing a [`RegistrationScope`] for the duration of an import.
///
/// Unlike [`crate::runtime_scope::ScopedRuntime`] this nests: the previous
/// scope is saved and restored on drop, so a package import that transitively
/// triggers another scoped import cannot strand the outer scope.
pub struct ScopedRegistration {
    previous: RegistrationScope,
}

impl ScopedRegistration {
    pub fn new(scope: RegistrationScope) -> Self {
        let previous = CURRENT_SCOPE.with(|slot| slot.replace(scope));
        Self { previous }
    }

    /// Convenience for the loader paths, which hold `&str` tenant/package.
    pub fn for_package(tenant_id: Option<&str>, package: Option<&str>) -> Self {
        Self::new(RegistrationScope::new(tenant_id, package))
    }
}

impl Drop for ScopedRegistration {
    fn drop(&mut self) {
        let restore = std::mem::take(&mut self.previous);
        CURRENT_SCOPE.with(|slot| {
            *slot.borrow_mut() = restore;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_installs_and_restores() {
        assert!(current_registration_scope().is_unscoped());
        {
            let _outer = ScopedRegistration::for_package(Some("acme"), Some("pkg-a"));
            assert_eq!(
                current_registration_scope(),
                RegistrationScope::new(Some("acme"), Some("pkg-a"))
            );
            {
                let _inner = ScopedRegistration::for_package(Some("globex"), Some("pkg-b"));
                assert_eq!(
                    current_registration_scope(),
                    RegistrationScope::new(Some("globex"), Some("pkg-b"))
                );
            }
            // Nested scope restored the outer one, not the empty default.
            assert_eq!(
                current_registration_scope(),
                RegistrationScope::new(Some("acme"), Some("pkg-a"))
            );
        }
        assert!(current_registration_scope().is_unscoped());
    }
}
