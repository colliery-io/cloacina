/*
 *  Copyright 2025-2026 Colliery Software
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

//! The secrets a task may read: the ones its package declares
//! (CLOACINA-T-1081).
//!
//! A package declares secrets by name (`secrets(db)` in `#[workflow]`,
//! `@cloaca.workflow_secrets("db")`). The compiler stores the names in the
//! package's metadata document (`declared_secrets`). The caller of a run
//! binds each name to a stored secret with `{"db": {"$secret": "s3_prod"}}`;
//! the binding is in the run's first context, under
//! [`SECRET_REFS_KEY`](cloacina_workflow::secret::SECRET_REFS_KEY).
//!
//! The grant of a task of package P in a run: for each name P declares, the
//! stored secret the run binds to it, or the name itself when the run binds
//! none. A package with no document, or one that declares nothing, has an
//! empty grant.
//!
//! [`SecretGrantMode::Declared`] turns the rule on for a runner's in-process
//! executor ([`DeclaredSecretResolver`]). The server's fleet executor checks
//! the same grant before it sends a secret to an agent. Embedded use keeps
//! [`SecretGrantMode::Unrestricted`]: the host owns the key.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use cloacina_workflow::secret::{SECRET_REFS_KEY, SecretResolver, SecretResolverError};

use crate::dal::DAL;
use crate::database::universal_types::UniversalUuid;

/// Whether a runner's in-process tasks read only the secrets their package
/// declares.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SecretGrantMode {
    /// A task reads any secret the runner's resolver can read. Embedded use
    /// and the daemon: the host owns the key and the code.
    #[default]
    Unrestricted,
    /// A task reads only the secrets its package declares. The server.
    Declared,
}

/// The secrets one task of one run may read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretGrant {
    /// The package of the task.
    pub package: String,
    /// The version of the package the run uses, when one is stored.
    pub version: Option<String>,
    /// The names of the stored secrets the task may read.
    pub allowed: BTreeSet<String>,
    /// Why the package has no declaration to read: no stored package, or no
    /// stored metadata document. `None` when the declaration was read.
    pub no_declaration: Option<String>,
}

impl SecretGrant {
    /// The grant of a package that declares `declared` (`None`: no stored
    /// document), in a run that binds `bindings` (the run's alias map).
    pub fn from_declaration(
        package: &str,
        version: Option<&str>,
        declared: Option<&[String]>,
        bindings: &BTreeMap<String, String>,
    ) -> Self {
        let Some(declared) = declared else {
            return Self {
                package: package.to_string(),
                version: version.map(str::to_string),
                allowed: BTreeSet::new(),
                no_declaration: Some(format!(
                    "package `{package}`{} has no stored metadata document",
                    version.map(|v| format!(" v{v}")).unwrap_or_default()
                )),
            };
        };
        let allowed = declared
            .iter()
            .map(|name| bindings.get(name).cloned().unwrap_or_else(|| name.clone()))
            .collect();
        Self {
            package: package.to_string(),
            version: version.map(str::to_string),
            allowed,
            no_declaration: None,
        }
    }

    /// The grant of a task whose package is not stored: nothing.
    pub fn no_package(package: &str) -> Self {
        Self {
            package: package.to_string(),
            version: None,
            allowed: BTreeSet::new(),
            no_declaration: Some(format!("package `{package}` is not stored")),
        }
    }

    /// Whether the task may read the stored secret `name`.
    pub fn permits(&self, name: &str) -> bool {
        self.allowed.contains(name)
    }

    /// The text of a refusal of `name`. It names the secret and the package;
    /// never a value.
    pub fn refusal_message(&self, name: &str) -> String {
        match &self.no_declaration {
            Some(why) => format!(
                "{name}: {why}, so its tasks read no secret; a task reads only the secrets \
                 its package declares"
            ),
            None => format!(
                "{name}: package `{}` does not declare it; a task reads only the secrets its \
                 package declares (`secrets(...)` in `#[workflow]`, \
                 `@cloaca.workflow_secrets(...)` in Python)",
                self.package
            ),
        }
    }
}

/// The alias map (`declared name -> stored secret`) of a context's data.
/// Names only; the map carries no values.
pub fn secret_bindings(context: &serde_json::Value) -> BTreeMap<String, String> {
    context
        .get(SECRET_REFS_KEY)
        .and_then(|v| v.as_object())
        .map(|map| {
            map.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

/// The grant of a task of `package_name` in the run `workflow_execution_id`,
/// read through `dal` (the tenant's schema): the declaration of the package
/// version the run is pinned to, mapped through `bindings`.
///
/// A package that is not stored, or whose pinned version can no longer be
/// read, has an empty grant.
pub async fn declared_secret_grant(
    dal: &DAL,
    workflow_execution_id: UniversalUuid,
    package_name: &str,
    bindings: &BTreeMap<String, String>,
) -> SecretGrant {
    match dal
        .workflow_packages()
        .resolve_dispatch_for_execution(workflow_execution_id, package_name, None)
        .await
    {
        Ok(Some(pkg)) => SecretGrant::from_declaration(
            package_name,
            Some(&pkg.version),
            pkg.declared_secrets.as_deref(),
            bindings,
        ),
        Ok(None) => SecretGrant::no_package(package_name),
        Err(e) => {
            tracing::warn!(
                package = %package_name,
                workflow_execution_id = %workflow_execution_id,
                error = %e,
                "secret grant: the package version of the run could not be read; \
                 the task reads no secret"
            );
            SecretGrant::no_package(package_name)
        }
    }
}

/// A binding of a run request that the package does not declare
/// (CLOACINA-T-1134).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndeclaredBinding {
    /// The binding name of the request.
    pub binding: String,
    /// The package of the workflow.
    pub package: String,
    /// The text of the refusal: the binding, the package and how to declare
    /// it.
    pub message: String,
}

/// The first name of `bindings` that `declared` does not hold. `declared`
/// `None` (no stored document) holds no name.
pub fn undeclared_binding(
    package: &str,
    version: Option<&str>,
    declared: Option<&[String]>,
    bindings: &BTreeMap<String, String>,
) -> Option<UndeclaredBinding> {
    let binding = bindings
        .keys()
        .find(|name| !declared.is_some_and(|d| d.iter().any(|n| n == *name)))?;
    let version = version.map(|v| format!(" v{v}")).unwrap_or_default();
    let why = match declared {
        None => format!("package `{package}`{version} has no stored metadata document"),
        Some(_) => format!("package `{package}`{version} does not declare it"),
    };
    Some(UndeclaredBinding {
        binding: binding.clone(),
        package: package.to_string(),
        message: format!(
            "secret binding `{binding}`: {why}; a run binds only the secrets its package \
             declares (`secrets({binding})` in `#[workflow]`, \
             `@cloaca.workflow_secrets(\"{binding}\")` in Python)"
        ),
    })
}

/// The first binding of a run request of `package_name` that the package does
/// not declare, read through `dal` (the tenant's schema) from the package's
/// active version: the declaration the first task of the run pins.
///
/// A package that is not stored refuses every binding, as its run-time grant
/// is empty. `Err` when the package row cannot be read.
pub async fn undeclared_run_binding(
    dal: &DAL,
    package_name: &str,
    bindings: &BTreeMap<String, String>,
) -> Result<Option<UndeclaredBinding>, crate::registry::error::RegistryError> {
    if bindings.is_empty() {
        return Ok(None);
    }
    let pkg = dal
        .workflow_packages()
        .get_active_dispatch(package_name, None)
        .await?;
    Ok(match pkg {
        Some(pkg) => undeclared_binding(
            package_name,
            Some(&pkg.version),
            pkg.declared_secrets.as_deref(),
            bindings,
        ),
        None => bindings.keys().next().map(|binding| UndeclaredBinding {
            binding: binding.clone(),
            package: package_name.to_string(),
            message: format!(
                "secret binding `{binding}`: package `{package_name}` has no built version, \
                 so its tasks read no secret; a run binds only the secrets its package \
                 declares (`secrets({binding})` in `#[workflow]`, \
                 `@cloaca.workflow_secrets(\"{binding}\")` in Python)"
            ),
        }),
    })
}

/// One refused request for a secret. Names only, never a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretRefusal {
    pub tenant: String,
    pub package: String,
    pub version: Option<String>,
    /// The namespaced task name.
    pub task: String,
    pub workflow_execution_id: String,
    pub task_execution_id: String,
    /// The name of the stored secret asked for.
    pub secret: String,
    /// Where it was refused: `"fleet"` (the server did not send it to an
    /// agent) or `"in_process"`.
    pub path: &'static str,
    /// The refusal text.
    pub message: String,
}

/// Writes a durable record of a refusal. The server registers one that
/// writes an audit row.
#[async_trait]
pub trait SecretRefusalRecorder: Send + Sync {
    async fn record(&self, refusal: &SecretRefusal);
}

static RECORDER: RwLock<Option<Arc<dyn SecretRefusalRecorder>>> = RwLock::new(None);

/// Register the writer of refusal records for this process. A later call
/// replaces the earlier one.
pub fn register_secret_refusal_recorder(recorder: Arc<dyn SecretRefusalRecorder>) {
    if let Ok(mut slot) = RECORDER.write() {
        *slot = Some(recorder);
    }
}

/// Log a refusal and hand it to the registered writer, if any.
pub async fn record_secret_refusal(refusal: &SecretRefusal) {
    tracing::warn!(
        tenant = %refusal.tenant,
        package = %refusal.package,
        version = refusal.version.as_deref().unwrap_or("-"),
        task = %refusal.task,
        workflow_execution_id = %refusal.workflow_execution_id,
        task_execution_id = %refusal.task_execution_id,
        secret.name = %refusal.secret,
        path = refusal.path,
        "secret refused: the task's package does not declare it"
    );
    let recorder = RECORDER.read().ok().and_then(|slot| slot.clone());
    if let Some(recorder) = recorder {
        recorder.record(refusal).await;
    }
}

/// A resolver that answers only the names of a [`SecretGrant`] and refuses
/// every other name before it reaches the inner resolver.
pub struct DeclaredSecretResolver {
    inner: Arc<dyn SecretResolver>,
    grant: SecretGrant,
    tenant: String,
    task: String,
    workflow_execution_id: String,
    task_execution_id: String,
}

impl std::fmt::Debug for DeclaredSecretResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeclaredSecretResolver")
            .field("grant", &self.grant)
            .field("task", &self.task)
            .finish()
    }
}

impl DeclaredSecretResolver {
    pub fn new(
        inner: Arc<dyn SecretResolver>,
        grant: SecretGrant,
        tenant: &str,
        task: &str,
        workflow_execution_id: String,
        task_execution_id: String,
    ) -> Self {
        Self {
            inner,
            grant,
            tenant: tenant.to_string(),
            task: task.to_string(),
            workflow_execution_id,
            task_execution_id,
        }
    }

    pub fn into_arc(self) -> Arc<dyn SecretResolver> {
        Arc::new(self)
    }
}

#[async_trait]
impl SecretResolver for DeclaredSecretResolver {
    async fn resolve(&self, name: &str) -> Result<BTreeMap<String, String>, SecretResolverError> {
        if self.grant.permits(name) {
            return self.inner.resolve(name).await;
        }
        let message = self.grant.refusal_message(name);
        record_secret_refusal(&SecretRefusal {
            tenant: self.tenant.clone(),
            package: self.grant.package.clone(),
            version: self.grant.version.clone(),
            task: self.task.clone(),
            workflow_execution_id: self.workflow_execution_id.clone(),
            task_execution_id: self.task_execution_id.clone(),
            secret: name.to_string(),
            path: "in_process",
            message: message.clone(),
        })
        .await;
        Err(SecretResolverError::NotGranted(message))
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod declared_tests {
    //! Two packages of one tenant, each with its own secret. A task reads
    //! only what its package declares (CLOACINA-T-1081). On SQLite in memory,
    //! through the builder the in-process executor uses.

    use super::*;
    use crate::context::Context;
    use crate::database::Database;
    use crate::database::schema::unified::workflow_packages;
    use crate::executor::TaskContextBuilder;
    use crate::executor::types::ClaimedTask;
    use crate::models::task_execution::NewTaskExecution;
    use crate::models::task_execution_metadata::NewTaskExecutionMetadata;
    use crate::models::workflow_execution::NewWorkflowExecution;
    use crate::registry::loader::package_loader::PackageMetadata;
    use crate::security::{SecretStore, SecretStoreResolver};
    use crate::task::TaskNamespace;
    use cloacina_workflow::SecretAccessError;
    use diesel::prelude::*;
    use std::sync::Mutex;

    fn kek() -> Vec<u8> {
        vec![7u8; 32]
    }

    async fn dal() -> DAL {
        let url = format!(
            "file:secret_grant_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations().await.expect("migrations");
        DAL::new(db)
    }

    /// A built package row. `declared`: `Some(names)` stores a metadata
    /// document that declares them; `None` stores no document.
    async fn package(dal: &DAL, name: &str, declared: Option<&[&str]>) {
        let metadata = PackageMetadata {
            package_name: name.to_string(),
            workflow_name: name.to_string(),
            version: "1.0.0".to_string(),
            description: None,
            author: None,
            tasks: vec![],
            graph_data: None,
            architecture: "x86_64".to_string(),
            symbols: vec![],
            workflow_triggers: vec![],
            declared_params: vec![],
            declared_surfaces: vec![],
            task_docs: Default::default(),
            python_platforms: None,
        };
        let id = dal
            .workflow_packages()
            .store_package_metadata(
                &uuid::Uuid::new_v4().to_string(),
                &metadata,
                crate::models::workflow_packages::StorageType::Database,
                None,
            )
            .await
            .unwrap();
        let pid = UniversalUuid(id);
        let hash = format!("hash-{name}");
        let document = declared.map(|names| {
            serde_json::json!({ "format_version": 1, "declared_secrets": names }).to_string()
        });
        crate::interact_on_backend!(dal, |conn| {
            diesel::update(workflow_packages::table.filter(workflow_packages::id.eq(pid)))
                .set((
                    workflow_packages::build_status.eq("success"),
                    workflow_packages::content_hash.eq(hash),
                    workflow_packages::metadata_document.eq(document),
                ))
                .execute(conn)
        })
        .unwrap();
    }

    async fn secret(store: &SecretStore, org: UniversalUuid, name: &str, value: &str) {
        let mut fields = BTreeMap::new();
        fields.insert("token".to_string(), value.to_string());
        store
            .create_secret(org, name, &fields, &kek())
            .await
            .unwrap();
    }

    /// A run whose first context binds `bindings`.
    async fn run(dal: &DAL, bindings: &[(&str, &str)]) -> UniversalUuid {
        let mut ctx = Context::<serde_json::Value>::new();
        if !bindings.is_empty() {
            let map: serde_json::Map<String, serde_json::Value> = bindings
                .iter()
                .map(|(k, v)| (k.to_string(), serde_json::json!(v)))
                .collect();
            ctx.insert(SECRET_REFS_KEY, serde_json::Value::Object(map))
                .unwrap();
        }
        let context_id = dal.context().create(&ctx).await.unwrap();
        dal.workflow_execution()
            .create(NewWorkflowExecution {
                workflow_name: "wf".into(),
                workflow_version: "1.0".into(),
                status: "Running".into(),
                context_id,
            })
            .await
            .unwrap()
            .id
    }

    async fn task(dal: &DAL, run: UniversalUuid, ns: &TaskNamespace) -> ClaimedTask {
        let id = dal
            .task_execution()
            .create(NewTaskExecution {
                workflow_execution_id: run,
                task_name: ns.to_string(),
                status: "Running".into(),
                attempt: 1,
                max_attempts: 3,
                trigger_rules: r#"{"type":"Always"}"#.into(),
                task_configuration: "{}".into(),
            })
            .await
            .unwrap()
            .id;
        ClaimedTask {
            task_execution_id: id,
            workflow_execution_id: run,
            task_name: ns.to_string(),
            attempt: 1,
        }
    }

    /// The tenant with packages `pkg_a` (declares `db`) and `pkg_b`
    /// (declares `b_db`), and the stored secrets `a_secret` and `b_secret`.
    async fn tenant() -> (DAL, Arc<dyn SecretResolver>) {
        let dal = dal().await;
        package(&dal, "pkg_a", Some(&["db"])).await;
        package(&dal, "pkg_b", Some(&["b_db"])).await;
        let org = UniversalUuid::new_v4();
        let store = SecretStore::new(dal.clone());
        secret(&store, org, "a_secret", "value-a").await;
        secret(&store, org, "b_secret", "value-b").await;
        // The server's in-process resolver: any secret of the tenant.
        let resolver = SecretStoreResolver::new(store, org, kek()).into_arc();
        (dal, resolver)
    }

    fn ns(package: &str, task: &str) -> TaskNamespace {
        TaskNamespace::new("public", package, "wf", task)
    }

    async fn build(
        dal: &DAL,
        resolver: &Arc<dyn SecretResolver>,
        claimed: &ClaimedTask,
        dependencies: &[TaskNamespace],
    ) -> Context<serde_json::Value> {
        TaskContextBuilder::new(dal.clone())
            .with_secret_resolver(Some(resolver.clone()))
            .with_secret_grants(SecretGrantMode::Declared)
            .build(claimed, dependencies)
            .await
            .unwrap()
    }

    /// The defect of CLOACINA-T-1081: a task of package A asked for the
    /// secret only package B declares and got it. It is refused, with the
    /// secret and the package named; A's own declared secret still arrives.
    #[tokio::test]
    async fn a_task_of_one_package_cannot_read_a_secret_only_another_declares() {
        let (dal, resolver) = tenant().await;
        let run = run(&dal, &[("db", "a_secret")]).await;
        let claimed = task(&dal, run, &ns("pkg_a", "t1")).await;
        let ctx = build(&dal, &resolver, &claimed, &[]).await;

        match ctx.secret("b_secret").await {
            Err(SecretAccessError::NotGranted(message)) => {
                assert!(message.contains("b_secret"), "{message}");
                assert!(message.contains("pkg_a"), "{message}");
                assert!(!message.contains("value-b"), "{message}");
            }
            Err(other) => panic!("expected NotGranted, got {other}"),
            Ok(_) => panic!("expected NotGranted, the secret was read"),
        }

        // The declared secret, by its declared name and by the bound name.
        assert_eq!(ctx.secret("db").await.unwrap()["token"], "value-a");
        assert_eq!(ctx.secret("a_secret").await.unwrap()["token"], "value-a");
    }

    /// Embedded use keeps the trusted resolver: the host owns the key.
    #[tokio::test]
    async fn an_unrestricted_builder_reads_any_secret_of_the_tenant() {
        let (dal, resolver) = tenant().await;
        let run = run(&dal, &[("db", "a_secret")]).await;
        let claimed = task(&dal, run, &ns("pkg_a", "t1")).await;
        let ctx = TaskContextBuilder::new(dal.clone())
            .with_secret_resolver(Some(resolver))
            .build(&claimed, &[])
            .await
            .unwrap();
        assert_eq!(ctx.secret("b_secret").await.unwrap()["token"], "value-b");
    }

    /// A binding the caller made for a name the package does not declare
    /// grants nothing.
    #[tokio::test]
    async fn a_binding_of_an_undeclared_name_is_refused() {
        let (dal, resolver) = tenant().await;
        let run = run(&dal, &[("db", "a_secret"), ("extra", "b_secret")]).await;
        let claimed = task(&dal, run, &ns("pkg_a", "t1")).await;
        let ctx = build(&dal, &resolver, &claimed, &[]).await;
        assert!(matches!(
            ctx.secret("extra").await,
            Err(SecretAccessError::NotGranted(_))
        ));
    }

    /// A task cannot change the bindings of a later task: the builder takes
    /// them from the run's first context, not from an upstream output.
    #[tokio::test]
    async fn an_upstream_output_cannot_rebind_a_secret() {
        let (dal, resolver) = tenant().await;
        let run = run(&dal, &[("db", "a_secret")]).await;

        let upstream = ns("pkg_a", "t1");
        let up = task(&dal, run, &upstream).await;
        let mut output = Context::<serde_json::Value>::new();
        output
            .insert(SECRET_REFS_KEY, serde_json::json!({ "db": "b_secret" }))
            .unwrap();
        output.insert("rows", serde_json::json!(3)).unwrap();
        let context_id = dal.context().create(&output).await.unwrap();
        dal.task_execution_metadata()
            .create(NewTaskExecutionMetadata {
                task_execution_id: up.task_execution_id,
                workflow_execution_id: run,
                task_name: upstream.to_string(),
                context_id,
                context_size_bytes: None,
            })
            .await
            .unwrap();

        let claimed = task(&dal, run, &ns("pkg_a", "t2")).await;
        let ctx = build(&dal, &resolver, &claimed, &[upstream]).await;
        assert_eq!(
            ctx.get(SECRET_REFS_KEY),
            Some(&serde_json::json!({ "db": "a_secret" }))
        );
        assert_eq!(ctx.get("rows"), Some(&serde_json::json!(3)));
        assert_eq!(ctx.secret("db").await.unwrap()["token"], "value-a");
        assert!(matches!(
            ctx.secret("b_secret").await,
            Err(SecretAccessError::NotGranted(_))
        ));
    }

    /// A package with no stored document, one that declares nothing, and a
    /// task of a package that is not stored all read no secret.
    #[tokio::test]
    async fn a_package_with_no_declaration_reads_no_secret() {
        let (dal, resolver) = tenant().await;
        package(&dal, "old_pkg", Some(&[])).await;
        package(&dal, "undescribed", None).await;
        let run = run(&dal, &[("db", "a_secret")]).await;

        for (package, why) in [
            ("old_pkg", "does not declare it"),
            ("undescribed", "has no stored metadata document"),
            ("not_stored", "is not stored"),
        ] {
            let claimed = task(&dal, run, &ns(package, "t1")).await;
            let ctx = build(&dal, &resolver, &claimed, &[]).await;
            for name in ["a_secret", "b_secret", "db"] {
                match ctx.secret(name).await {
                    Err(SecretAccessError::NotGranted(message)) => {
                        assert!(message.contains(why), "{package}: {message}")
                    }
                    Err(other) => panic!("{package} {name}: expected NotGranted, got {other}"),
                    Ok(_) => panic!("{package} {name}: expected NotGranted, the secret was read"),
                }
            }
        }
    }

    /// The tenant stays the outer boundary: a declared name stored only in
    /// another tenant is not found.
    #[tokio::test]
    async fn another_tenants_secret_is_not_reachable() {
        let (dal, _) = tenant().await;
        let store = SecretStore::new(dal.clone());
        let other_tenant = UniversalUuid::new_v4();
        secret(&store, other_tenant, "c_secret", "value-c").await;
        let own_tenant = UniversalUuid::new_v4();
        let resolver = SecretStoreResolver::new(store, own_tenant, kek()).into_arc();

        package(&dal, "pkg_c", Some(&["c_secret"])).await;
        let run = run(&dal, &[]).await;
        let claimed = task(&dal, run, &ns("pkg_c", "t1")).await;
        let ctx = build(&dal, &resolver, &claimed, &[]).await;
        assert!(matches!(
            ctx.secret("c_secret").await,
            Err(SecretAccessError::NotFound(_))
        ));
    }

    struct Collect(Mutex<Vec<SecretRefusal>>);

    #[async_trait]
    impl SecretRefusalRecorder for Collect {
        async fn record(&self, refusal: &SecretRefusal) {
            self.0.lock().unwrap().push(refusal.clone());
        }
    }

    /// A refusal reaches the registered writer with the secret, package,
    /// task and run named, and no value.
    #[tokio::test]
    async fn a_refusal_is_recorded() {
        let collect = Arc::new(Collect(Mutex::new(Vec::new())));
        register_secret_refusal_recorder(collect.clone());

        let (dal, resolver) = tenant().await;
        let run = run(&dal, &[("db", "a_secret")]).await;
        let claimed = task(&dal, run, &ns("pkg_a", "recorded")).await;
        let ctx = build(&dal, &resolver, &claimed, &[]).await;
        assert!(ctx.secret("b_secret").await.is_err());

        let refusals: Vec<SecretRefusal> = collect
            .0
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.task_execution_id == claimed.task_execution_id.to_string())
            .cloned()
            .collect();
        assert_eq!(refusals.len(), 1);
        let refusal = &refusals[0];
        assert_eq!(refusal.secret, "b_secret");
        assert_eq!(refusal.package, "pkg_a");
        assert_eq!(refusal.version.as_deref(), Some("1.0.0"));
        assert_eq!(refusal.task, ns("pkg_a", "recorded").to_string());
        assert_eq!(refusal.workflow_execution_id, run.to_string());
        assert_eq!(refusal.path, "in_process");
        assert_eq!(refusal.tenant, "public");
        assert!(!format!("{refusal:?}").contains("value-b"));
    }

    fn binding_map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    /// CLOACINA-T-1134: a run request's bindings are read against the
    /// declaration of the package's active version, the one dispatch reads.
    #[tokio::test]
    async fn a_run_request_binding_is_checked_against_the_stored_declaration() {
        let (dal, _) = tenant().await;
        package(&dal, "undescribed", None).await;

        // Declared names only: accepted. No binding: not checked.
        let ok = undeclared_run_binding(&dal, "pkg_a", &binding_map(&[("db", "a_secret")]))
            .await
            .unwrap();
        assert_eq!(ok, None);
        let none = undeclared_run_binding(&dal, "not_stored", &BTreeMap::new())
            .await
            .unwrap();
        assert_eq!(none, None);

        // A name pkg_a does not declare.
        let refused = undeclared_run_binding(
            &dal,
            "pkg_a",
            &binding_map(&[("db", "a_secret"), ("b_db", "b_secret")]),
        )
        .await
        .unwrap()
        .expect("refused");
        assert_eq!(refused.binding, "b_db");
        assert_eq!(refused.package, "pkg_a");
        assert!(
            refused.message.contains("does not declare it"),
            "{refused:?}"
        );

        // No stored document, and no stored package: every binding.
        for (package, why) in [
            ("undescribed", "has no stored metadata document"),
            ("not_stored", "has no built version"),
        ] {
            let refused =
                undeclared_run_binding(&dal, package, &binding_map(&[("db", "a_secret")]))
                    .await
                    .unwrap()
                    .expect("refused");
            assert_eq!(refused.binding, "db");
            assert!(refused.message.contains(why), "{package}: {refused:?}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bindings(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn a_declared_name_maps_through_its_binding() {
        let grant = SecretGrant::from_declaration(
            "pkg_a",
            Some("1.0.0"),
            Some(&["db".to_string(), "api".to_string()]),
            &bindings(&[("db", "db_prod"), ("other", "b_secret")]),
        );
        assert!(grant.permits("db_prod"));
        assert!(grant.permits("api"));
        // A binding of a name the package does not declare grants nothing.
        assert!(!grant.permits("b_secret"));
        assert!(!grant.permits("db"));
    }

    #[test]
    fn no_document_and_no_package_grant_nothing() {
        let grant =
            SecretGrant::from_declaration("pkg_a", Some("1.0.0"), None, &bindings(&[("db", "x")]));
        assert!(grant.allowed.is_empty());
        assert!(
            grant
                .refusal_message("x")
                .contains("has no stored metadata document")
        );

        let grant = SecretGrant::no_package("pkg_a");
        assert!(grant.allowed.is_empty());
        assert!(grant.refusal_message("x").contains("is not stored"));
    }

    #[test]
    fn a_refusal_names_the_secret_and_the_package() {
        let grant = SecretGrant::from_declaration("pkg_a", None, Some(&[]), &BTreeMap::new());
        let message = grant.refusal_message("b_secret");
        assert!(message.starts_with("b_secret: package `pkg_a` does not declare it"));
    }

    /// CLOACINA-T-1134: the refusal names the binding, the package and how to
    /// declare it.
    #[test]
    fn an_undeclared_binding_is_named_with_its_package() {
        let declared = ["db".to_string()];
        assert_eq!(
            undeclared_binding(
                "pkg_a",
                Some("1.0.0"),
                Some(&declared),
                &bindings(&[("db", "x")])
            ),
            None
        );
        let refused = undeclared_binding(
            "pkg_a",
            Some("1.0.0"),
            Some(&declared),
            &bindings(&[("db", "x"), ("warehouse", "y")]),
        )
        .expect("refused");
        assert_eq!(refused.binding, "warehouse");
        assert_eq!(refused.package, "pkg_a");
        assert!(
            refused.message.starts_with(
                "secret binding `warehouse`: package `pkg_a` v1.0.0 does not declare it"
            ),
            "{}",
            refused.message
        );
        assert!(refused.message.contains("secrets(warehouse)"));
        assert!(
            refused
                .message
                .contains("@cloaca.workflow_secrets(\"warehouse\")")
        );
        // The stored secret's name is not the binding name: still refused.
        let refused = undeclared_binding("pkg_a", None, Some(&[]), &bindings(&[("x", "db")]))
            .expect("refused");
        assert_eq!(refused.binding, "x");
        let refused =
            undeclared_binding("pkg_a", None, None, &bindings(&[("db", "x")])).expect("refused");
        assert!(refused.message.contains("has no stored metadata document"));
    }

    #[test]
    fn bindings_are_read_from_the_alias_map() {
        let ctx = serde_json::json!({
            SECRET_REFS_KEY: {"db": "db_prod", "bad": 5},
            "plain": "x",
        });
        assert_eq!(secret_bindings(&ctx), bindings(&[("db", "db_prod")]));
        assert!(secret_bindings(&serde_json::json!({})).is_empty());
    }
}
