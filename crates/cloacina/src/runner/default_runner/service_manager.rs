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

//! Background service lifecycle management for `DefaultRunner`.
//!
//! Each long-running background loop owned by the runner is wrapped as a
//! [`BackgroundService`]. The [`ServiceManager`] owns the collection,
//! orchestrates `start_all()`/`shutdown_all()`, and provides typed slots so
//! external callers can still reach individual service Arcs (registry,
//! reactive scheduler, unified scheduler, etc).

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{broadcast, watch, RwLock};
use tokio::task::JoinHandle;
use tracing::Instrument;

use crate::computation_graph::scheduler::ReactiveScheduler;
use crate::execution_planner::stale_claim_sweeper::StaleClaimSweeper;
use crate::executor::workflow_executor::WorkflowExecutionError;
use crate::registry::{traits::WorkflowRegistry, RegistryReconciler};
use crate::{CronRecoveryService, Scheduler, TaskScheduler};

/// A background service whose lifecycle is owned by the [`ServiceManager`].
///
/// Implementations spawn their loop in `start()`, store the join handle
/// internally, and stop the loop in `shutdown()`. The broadcast channel
/// passed to `start()` is the runner-wide shutdown signal — services that
/// also expose a private `watch::Sender<bool>` should propagate the stop
/// from the broadcast into their loop.
#[async_trait]
pub(super) trait BackgroundService: Send + Sync {
    fn name(&self) -> &'static str;

    async fn start(
        &mut self,
        shutdown_rx: broadcast::Receiver<()>,
    ) -> Result<(), WorkflowExecutionError>;

    async fn shutdown(&mut self) -> Result<(), WorkflowExecutionError>;
}

/// Owns and orchestrates the runner's background services.
pub(in crate::runner) struct ServiceManager {
    services: Vec<Box<dyn BackgroundService>>,
    shutdown_tx: broadcast::Sender<()>,
    shutdown_sent: bool,

    // Typed slots so the runner can expose accessors without re-introducing
    // a field-per-service. Filled as services are registered.
    pub(super) cron_recovery: Option<Arc<CronRecoveryService>>,
    pub(super) workflow_registry: Option<Arc<dyn WorkflowRegistry>>,
    pub(super) unified_scheduler: Option<Arc<Scheduler>>,
    /// Shared reactive-scheduler slot — set by `DefaultRunner::set_reactive_scheduler`
    /// and observed by the registry reconciler. The slot is shared via Arc so
    /// updates are visible to whoever holds a clone.
    pub(super) reactive_scheduler: Arc<RwLock<Option<Arc<ReactiveScheduler>>>>,
}

impl ServiceManager {
    pub(super) fn new() -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            services: Vec::new(),
            shutdown_tx,
            shutdown_sent: false,
            cron_recovery: None,
            workflow_registry: None,
            unified_scheduler: None,
            reactive_scheduler: Arc::new(RwLock::new(None)),
        }
    }

    pub(super) fn register(&mut self, service: Box<dyn BackgroundService>) {
        self.services.push(service);
    }

    /// Start every registered service in registration order.
    pub(super) async fn start_all(&mut self) -> Result<(), WorkflowExecutionError> {
        for svc in &mut self.services {
            tracing::debug!(service = svc.name(), "starting background service");
            svc.start(self.shutdown_tx.subscribe()).await?;
        }
        Ok(())
    }

    /// Broadcast shutdown and await each service in reverse registration order.
    pub(super) async fn shutdown_all(&mut self) -> Result<(), WorkflowExecutionError> {
        if !self.shutdown_sent {
            let _ = self.shutdown_tx.send(());
            self.shutdown_sent = true;
        }
        for svc in self.services.iter_mut().rev() {
            tracing::debug!(service = svc.name(), "stopping background service");
            if let Err(e) = svc.shutdown().await {
                tracing::error!(service = svc.name(), "service shutdown error: {}", e);
            }
        }
        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Service wrappers
// -----------------------------------------------------------------------------

/// Wraps the per-runner `TaskScheduler` polling loop.
pub(super) struct TaskSchedulerService {
    scheduler: Arc<TaskScheduler>,
    span: tracing::Span,
    handle: Option<JoinHandle<()>>,
}

impl TaskSchedulerService {
    pub(super) fn new(scheduler: Arc<TaskScheduler>, span: tracing::Span) -> Self {
        Self {
            scheduler,
            span,
            handle: None,
        }
    }
}

#[async_trait]
impl BackgroundService for TaskSchedulerService {
    fn name(&self) -> &'static str {
        "task_scheduler"
    }

    async fn start(
        &mut self,
        mut shutdown_rx: broadcast::Receiver<()>,
    ) -> Result<(), WorkflowExecutionError> {
        let scheduler = self.scheduler.clone();
        let span = self.span.clone();
        let handle = tokio::spawn(
            async move {
                let mut scheduler_future = Box::pin(scheduler.run_scheduling_loop());
                tokio::select! {
                    result = &mut scheduler_future => {
                        if let Err(e) = result {
                            tracing::error!("Scheduler loop failed: {}", e);
                        } else {
                            tracing::info!("Scheduler loop completed");
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        tracing::info!("Scheduler shutdown requested");
                    }
                }
            }
            .instrument(span),
        );
        self.handle = Some(handle);
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), WorkflowExecutionError> {
        if let Some(h) = self.handle.take() {
            let _ = h.await;
        }
        Ok(())
    }
}

/// Wraps the unified cron + trigger scheduler loop.
pub(super) struct UnifiedSchedulerService {
    scheduler: Arc<Scheduler>,
    inner_shutdown_tx: watch::Sender<bool>,
    span: tracing::Span,
    handle: Option<JoinHandle<()>>,
}

impl UnifiedSchedulerService {
    pub(super) fn new(
        scheduler: Arc<Scheduler>,
        inner_shutdown_tx: watch::Sender<bool>,
        span: tracing::Span,
    ) -> Self {
        Self {
            scheduler,
            inner_shutdown_tx,
            span,
            handle: None,
        }
    }
}

#[async_trait]
impl BackgroundService for UnifiedSchedulerService {
    fn name(&self) -> &'static str {
        "unified_scheduler"
    }

    async fn start(
        &mut self,
        mut shutdown_rx: broadcast::Receiver<()>,
    ) -> Result<(), WorkflowExecutionError> {
        let mut scheduler_clone = (*self.scheduler).clone();
        let inner_tx = self.inner_shutdown_tx.clone();
        let span = self.span.clone();
        let handle = tokio::spawn(
            async move {
                tokio::select! {
                    result = scheduler_clone.run_polling_loop() => {
                        if let Err(e) = result {
                            tracing::error!("Unified scheduler failed: {}", e);
                        } else {
                            tracing::info!("Unified scheduler completed");
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        tracing::info!("Unified scheduler shutdown requested via broadcast");
                        let _ = inner_tx.send(true);
                    }
                }
            }
            .instrument(span),
        );
        self.handle = Some(handle);
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), WorkflowExecutionError> {
        if let Some(h) = self.handle.take() {
            let _ = h.await;
        }
        Ok(())
    }
}

/// Wraps the cron recovery loop.
pub(super) struct CronRecoveryServiceWrapper {
    service: Arc<CronRecoveryService>,
    inner_shutdown_tx: watch::Sender<bool>,
    span: tracing::Span,
    handle: Option<JoinHandle<()>>,
}

impl CronRecoveryServiceWrapper {
    pub(super) fn new(
        service: Arc<CronRecoveryService>,
        inner_shutdown_tx: watch::Sender<bool>,
        span: tracing::Span,
    ) -> Self {
        Self {
            service,
            inner_shutdown_tx,
            span,
            handle: None,
        }
    }
}

#[async_trait]
impl BackgroundService for CronRecoveryServiceWrapper {
    fn name(&self) -> &'static str {
        "cron_recovery"
    }

    async fn start(
        &mut self,
        mut shutdown_rx: broadcast::Receiver<()>,
    ) -> Result<(), WorkflowExecutionError> {
        let mut service_clone = (*self.service).clone();
        let inner_tx = self.inner_shutdown_tx.clone();
        let span = self.span.clone();
        let handle = tokio::spawn(
            async move {
                tokio::select! {
                    result = service_clone.run_recovery_loop() => {
                        if let Err(e) = result {
                            tracing::error!("Cron recovery service failed: {}", e);
                        } else {
                            tracing::info!("Cron recovery service completed");
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        tracing::info!("Cron recovery service shutdown requested via broadcast");
                        let _ = inner_tx.send(true);
                    }
                }
            }
            .instrument(span),
        );
        self.handle = Some(handle);
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), WorkflowExecutionError> {
        if let Some(h) = self.handle.take() {
            let _ = h.await;
        }
        Ok(())
    }
}

/// Wraps the registry reconciler loop. Owns the reconciler outright
/// because `start_reconciliation_loop` consumes `self`.
pub(super) struct RegistryReconcilerService {
    reconciler: Option<RegistryReconciler>,
    inner_shutdown_tx: watch::Sender<bool>,
    span: tracing::Span,
    handle: Option<JoinHandle<()>>,
}

impl RegistryReconcilerService {
    pub(super) fn new(
        reconciler: RegistryReconciler,
        inner_shutdown_tx: watch::Sender<bool>,
        span: tracing::Span,
    ) -> Self {
        Self {
            reconciler: Some(reconciler),
            inner_shutdown_tx,
            span,
            handle: None,
        }
    }
}

#[async_trait]
impl BackgroundService for RegistryReconcilerService {
    fn name(&self) -> &'static str {
        "registry_reconciler"
    }

    async fn start(
        &mut self,
        mut shutdown_rx: broadcast::Receiver<()>,
    ) -> Result<(), WorkflowExecutionError> {
        let reconciler =
            self.reconciler
                .take()
                .ok_or_else(|| WorkflowExecutionError::Configuration {
                    message: "registry reconciler already started".to_string(),
                })?;
        let inner_tx = self.inner_shutdown_tx.clone();
        let span = self.span.clone();
        let handle = tokio::spawn(
            async move {
                tokio::select! {
                    result = reconciler.start_reconciliation_loop() => {
                        if let Err(e) = result {
                            tracing::error!("Registry reconciler failed: {}", e);
                        } else {
                            tracing::info!("Registry reconciler completed");
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        tracing::info!("Registry reconciler shutdown requested via broadcast");
                        let _ = inner_tx.send(true);
                    }
                }
            }
            .instrument(span),
        );
        self.handle = Some(handle);
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), WorkflowExecutionError> {
        if let Some(h) = self.handle.take() {
            let _ = h.await;
        }
        Ok(())
    }
}

/// Wraps the stale-claim sweeper loop.
pub(super) struct StaleClaimSweeperService {
    sweeper: Option<StaleClaimSweeper>,
    inner_shutdown_tx: watch::Sender<bool>,
    span: tracing::Span,
    handle: Option<JoinHandle<()>>,
}

impl StaleClaimSweeperService {
    pub(super) fn new(
        sweeper: StaleClaimSweeper,
        inner_shutdown_tx: watch::Sender<bool>,
        span: tracing::Span,
    ) -> Self {
        Self {
            sweeper: Some(sweeper),
            inner_shutdown_tx,
            span,
            handle: None,
        }
    }
}

#[async_trait]
impl BackgroundService for StaleClaimSweeperService {
    fn name(&self) -> &'static str {
        "stale_claim_sweeper"
    }

    async fn start(
        &mut self,
        mut shutdown_rx: broadcast::Receiver<()>,
    ) -> Result<(), WorkflowExecutionError> {
        let mut sweeper =
            self.sweeper
                .take()
                .ok_or_else(|| WorkflowExecutionError::Configuration {
                    message: "stale claim sweeper already started".to_string(),
                })?;
        let inner_tx = self.inner_shutdown_tx.clone();
        let span = self.span.clone();
        let handle = tokio::spawn(
            async move {
                tokio::select! {
                    _ = sweeper.run() => {
                        tracing::info!("Stale claim sweeper completed");
                    }
                    _ = shutdown_rx.recv() => {
                        tracing::info!("Stale claim sweeper shutdown requested");
                        let _ = inner_tx.send(true);
                    }
                }
            }
            .instrument(span),
        );
        self.handle = Some(handle);
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), WorkflowExecutionError> {
        if let Some(h) = self.handle.take() {
            let _ = h.await;
        }
        Ok(())
    }
}
