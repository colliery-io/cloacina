# Cloacina Cron Scheduling Implementation Plan

**Project**: Built-in Cron Scheduling for Cloacina Workflows
**Issue**: https://github.com/colliery-io/cloacina/issues/11
**Status**: Planning Phase
**Created**: 2025-05-29
**Updated**: 2025-05-29

## Executive Summary

Implementation of native cron-like scheduling capabilities for Cloacina workflows, maintaining the embedded philosophy with no external dependencies. The feature leverages existing PostgreSQL/SQLite infrastructure for job queue patterns and integrates seamlessly with the current database-driven, polling architecture.

## Target API Compliance

**Exact implementation of issue #11 interface requirements**:

### Simple Cron Expression
```rust
let scheduler = Scheduler::new(&executor);
scheduler.schedule(
    "nightly_etl",
    "0 2 * * *",  // Every day at 2 AM
    "etl_pipeline"
).await?;
```

### Configuration-based Scheduling
```rust
scheduler.schedule_workflow(
    ScheduleConfig {
        name: "weekly_cleanup",
        cron: "0 0 * * 0",  // Every Sunday at midnight
        workflow: "cleanup_pipeline",
        timezone: "America/New_York",
        max_concurrent: 1,  // Don't run if previous instance still running
        context: json!({"retention_days": 7})
    }
).await?;
```

### Programmatic Builder Pattern
```rust
let schedule = Schedule::daily().at_hour(3).at_minute(30);
scheduler.add_schedule("daily_backup", schedule, "backup_workflow").await?;
```

### Advanced Features
```rust
// One-time delayed execution
scheduler.schedule_once(
    "delayed_notification",
    Duration::from_hours(24),
    "send_reminder"
).await?;

// Conditional scheduling
scheduler.schedule_conditional(
    "market_hours_trading",
    "*/5 * * * 1-5",  // Every 5 minutes, weekdays only
    "trading_pipeline",
    |context| async move { is_market_open().await }
).await?;
```

## Architecture Integration

### Database-First Job Queue Pattern

**Core Principle**: Use PostgreSQL/SQLite directly for job queue patterns (confirmed requirement)

#### New Database Schema
```sql
-- Single table for all cron scheduling needs
CREATE TABLE cron_schedules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_name VARCHAR NOT NULL,
    cron_expression VARCHAR NOT NULL,
    timezone VARCHAR NOT NULL DEFAULT 'UTC',
    enabled BOOLEAN NOT NULL DEFAULT true,
    overlap_strategy VARCHAR NOT NULL DEFAULT 'skip',  -- 'skip', 'queue', 'kill'
    catchup_policy VARCHAR NOT NULL DEFAULT 'skip',    -- 'skip', 'run_once', 'run_all'
    next_run_at TIMESTAMP WITH TIME ZONE NOT NULL,
    last_run_at TIMESTAMP WITH TIME ZONE,
    last_execution_id UUID,
    execution_context JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    CONSTRAINT fk_last_execution
        FOREIGN KEY (last_execution_id)
        REFERENCES pipeline_executions(id)
);

-- Optimized index for job queue operations
CREATE INDEX idx_cron_queue
ON cron_schedules (enabled, next_run_at)
WHERE enabled = true;

-- Workflow lookup optimization
CREATE INDEX idx_cron_workflow
ON cron_schedules (workflow_name, enabled);

-- Leader election for distributed scheduling
CREATE TABLE cron_leaders (
    id VARCHAR PRIMARY KEY DEFAULT 'singleton',
    leader_instance_id UUID NOT NULL,
    lease_expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    heartbeat_interval INTEGER NOT NULL DEFAULT 30,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);
```

### Integration with Existing Architecture

**Seamless integration with current components**:

#### Enhanced UnifiedExecutor
```rust
pub struct UnifiedExecutor {
    // ... existing fields ...
    cron_scheduler: Option<Arc<CronScheduler>>,  // NEW
}

impl UnifiedExecutor {
    pub async fn start_background_services(&self) -> Result<(), PipelineError> {
        // ... existing scheduler/executor startup ...

        // NEW: Start cron scheduling service
        if self.config.enable_cron_scheduling {
            let cron_handle = tokio::spawn(async move {
                let mut cron_future = Box::pin(cron_scheduler.run_scheduling_loop());
                tokio::select! {
                    result = &mut cron_future => { /* handle result */ }
                    _ = cron_shutdown_rx.recv() => { /* shutdown */ }
                }
            });
            handles.cron_handle = Some(cron_handle);
        }

        Ok(())
    }
}
```

#### Extended DAL Pattern
```rust
// Follows existing DAL architecture exactly
impl DAL {
    pub fn cron_schedule(&self) -> CronScheduleDAL {
        CronScheduleDAL::new(self.pool.clone())
    }
}

pub struct CronScheduleDAL {
    pool: DbPool,
}

impl CronScheduleDAL {
    pub async fn create(&self, schedule: NewCronSchedule) -> Result<CronSchedule, DatabaseError>;
    pub async fn get_due_schedules(&self, now: DateTime<Utc>) -> Result<Vec<CronSchedule>, DatabaseError>;
    pub async fn update_next_run(&self, id: UniversalUuid, next_run: DateTime<Utc>) -> Result<(), DatabaseError>;
    pub async fn try_claim_leadership(&self, instance_id: Uuid) -> Result<bool, DatabaseError>;
    pub async fn release_leadership(&self, instance_id: Uuid) -> Result<(), DatabaseError>;
}
```

#### Extended PipelineExecutor Trait
```rust
#[async_trait]
pub trait PipelineExecutor: Send + Sync {
    // ... existing methods ...

    // NEW: Cron scheduling methods
    async fn schedule_workflow(
        &self,
        workflow_name: &str,
        cron_expression: &str,
        timezone: Option<&str>,
    ) -> Result<Uuid, PipelineError>;

    async fn schedule_workflow_with_config(
        &self,
        config: ScheduleConfig,
    ) -> Result<Uuid, PipelineError>;

    async fn unschedule_workflow(&self, schedule_id: Uuid) -> Result<(), PipelineError>;
    async fn list_schedules(&self) -> Result<Vec<CronScheduleInfo>, PipelineError>;
    async fn enable_schedule(&self, schedule_id: Uuid) -> Result<(), PipelineError>;
    async fn disable_schedule(&self, schedule_id: Uuid) -> Result<(), PipelineError>;
}
```

### Multi-Executor Coordination

**Database-driven leader election pattern**:

```rust
pub struct CronLeaderElection {
    dal: Arc<DAL>,
    instance_id: Uuid,
    lease_duration: Duration,
    heartbeat_interval: Duration,
}

impl CronLeaderElection {
    pub async fn try_acquire_leadership(&self) -> Result<bool, DatabaseError> {
        // Atomic operation: claim leadership if no current leader or lease expired
        let now = Utc::now();
        let lease_expires = now + self.lease_duration;

        let rows_affected = self.dal.cron_schedule()
            .try_claim_leadership(self.instance_id, lease_expires)
            .await?;

        Ok(rows_affected > 0)
    }

    pub async fn renew_lease(&self) -> Result<bool, DatabaseError> {
        // Extend lease if we're still the leader
        let now = Utc::now();
        let lease_expires = now + self.lease_duration;

        self.dal.cron_schedule()
            .renew_leadership(self.instance_id, lease_expires)
            .await
    }
}
```

## Technical Implementation Details

### Cron Expression Library Integration

**Using `croner` crate for maximum compatibility**:

```rust
use croner::Cron;
use chrono_tz::Tz;

pub struct CronEvaluator {
    expression: Cron,
    timezone: Tz,
}

impl CronEvaluator {
    pub fn new(cron_expr: &str, timezone: &str) -> Result<Self, CronError> {
        let expression = Cron::new(cron_expr).parse()?;
        let timezone: Tz = timezone.parse()
            .map_err(|_| CronError::InvalidTimezone(timezone.to_string()))?;

        Ok(Self { expression, timezone })
    }

    pub fn next_execution(&self, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let local_time = self.timezone.from_utc_datetime(&after.naive_utc());
        let next_local = self.expression.find_next_occurrence(&local_time, false)?;
        Some(next_local.with_timezone(&Utc))
    }
}
```

### Overlap Prevention Strategies

**Database-level coordination for concurrent execution control**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OverlapStrategy {
    Skip,    // Skip if previous execution still running
    Queue,   // Queue next execution when current completes
    Kill,    // Terminate previous execution before starting new one
}

impl CronScheduler {
    async fn handle_overlap_prevention(
        &self,
        schedule: &CronSchedule
    ) -> Result<bool, ValidationError> {
        match schedule.overlap_strategy {
            OverlapStrategy::Skip => {
                let has_running = self.dal.pipeline_execution()
                    .has_running_execution_for_schedule(schedule.id)
                    .await?;
                Ok(!has_running)  // Only proceed if no running execution
            }
            OverlapStrategy::Queue => {
                // Always allow scheduling - execution will queue naturally
                Ok(true)
            }
            OverlapStrategy::Kill => {
                // Terminate existing executions and proceed
                self.dal.pipeline_execution()
                    .cancel_running_executions_for_schedule(schedule.id)
                    .await?;
                Ok(true)
            }
        }
    }
}
```

### Timezone Handling & DST Transitions

**Robust timezone support with DST awareness**:

```rust
impl CronEvaluator {
    pub fn handle_dst_transition(&self, scheduled_time: DateTime<Utc>) -> Option<DateTime<Utc>> {
        let local_time = self.timezone.from_utc_datetime(&scheduled_time.naive_utc());

        // Check if this time exists (handle "spring forward")
        match local_time.single() {
            Some(valid_time) => Some(valid_time.with_timezone(&Utc)),
            None => {
                // Time doesn't exist due to DST transition
                // Schedule for one hour later
                let adjusted = scheduled_time + Duration::hours(1);
                Some(adjusted)
            }
        }
    }

    pub fn avoid_dst_ambiguity(&self, scheduled_time: DateTime<Utc>) -> DateTime<Utc> {
        let local_time = self.timezone.from_utc_datetime(&scheduled_time.naive_utc());

        // For ambiguous times (fall back), choose the later occurrence
        match local_time.earliest() {
            Some(earliest) => earliest.with_timezone(&Utc),
            None => scheduled_time,  // Fallback to original time
        }
    }
}
```

### Performance Optimization

**Efficient polling and batch processing**:

```rust
impl CronScheduler {
    pub async fn run_scheduling_loop(&self) -> Result<(), ValidationError> {
        let mut interval = tokio::time::interval(self.config.poll_interval);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Err(e) = self.process_due_schedules().await {
                        warn!("Error processing cron schedules: {}", e);
                    }
                }
                _ = self.shutdown_signal.recv() => {
                    info!("Cron scheduler shutting down");
                    break;
                }
            }
        }

        Ok(())
    }

    async fn process_due_schedules(&self) -> Result<(), ValidationError> {
        // Only process if we're the leader
        if !self.leader_election.is_leader().await? {
            return Ok(());
        }

        let now = Utc::now();
        let batch_size = self.config.batch_size;

        // Process in batches to avoid memory issues with large schedule counts
        let mut offset = 0;
        loop {
            let schedules = self.dal.cron_schedule()
                .get_due_schedules_batch(now, batch_size, offset)
                .await?;

            if schedules.is_empty() {
                break;
            }

            // Process batch
            for schedule in &schedules {
                if let Err(e) = self.process_single_schedule(schedule).await {
                    warn!("Failed to process schedule {}: {}", schedule.id, e);
                }
            }

            offset += batch_size;
        }

        Ok(())
    }
}
```

---

## Implementation Phases

### Phase 1: Core Infrastructure (Weeks 1-2)

**Goal**: Establish database schema and basic cron evaluation

#### 1.1 Database Foundation
- [ ] Add `cron_schedules` table to both PostgreSQL and SQLite schemas
- [ ] Create migration scripts for both database types
- [ ] Add `cron_leaders` table for leader election
- [ ] Implement optimized indexes for job queue operations

#### 1.2 Models and DAL
- [ ] Create `CronSchedule` model with all required fields
- [ ] Implement `CronScheduleDAL` following existing DAL patterns
- [ ] Add PostgreSQL-specific and SQLite-specific implementations
- [ ] Test database operations across both backends

#### 1.3 Cron Expression Library Integration
- [ ] Integrate `croner` crate for cron parsing
- [ ] Implement `CronEvaluator` with timezone support
- [ ] Add comprehensive tests for cron expression evaluation
- [ ] Handle edge cases (DST transitions, invalid expressions)

#### 1.4 Basic Leader Election
- [ ] Implement `CronLeaderElection` with database coordination
- [ ] Add lease management and heartbeat functionality
- [ ] Test multi-instance leader election scenarios
- [ ] Add graceful leader transition handling

**Deliverables**:
- Working database schema for cron scheduling
- Reliable cron expression evaluation with timezone support
- Robust leader election mechanism
- Comprehensive test suite for core functionality

**Validation**:
```rust
// Test cron expression evaluation
let evaluator = CronEvaluator::new("0 2 * * *", "America/New_York")?;
let now = Utc::now();
let next_run = evaluator.next_execution(now);
assert!(next_run.is_some());

// Test leader election
let leader = CronLeaderElection::new(dal, instance_id);
let is_leader = leader.try_acquire_leadership().await?;
assert!(is_leader);
```

### Phase 2: Core Scheduling Integration (Weeks 3-4)

**Goal**: Integrate scheduling with existing UnifiedExecutor

#### 2.1 CronScheduler Implementation
- [ ] Create `CronScheduler` service with polling loop
- [ ] Implement batch processing of due schedules
- [ ] Add overlap prevention strategies (skip, queue, kill)
- [ ] Integrate with existing workflow execution pipeline

#### 2.2 UnifiedExecutor Enhancement
- [ ] Add cron scheduling service to UnifiedExecutor
- [ ] Implement configuration options for scheduling
- [ ] Add graceful startup and shutdown for cron service
- [ ] Test integration with existing task scheduler and executor

#### 2.3 API Implementation
- [ ] Implement simple `schedule()` method
- [ ] Add `schedule_workflow()` with configuration
- [ ] Create basic schedule management (list, enable, disable)
- [ ] Add comprehensive error handling and validation

#### 2.4 Testing and Validation
- [ ] End-to-end tests with real workflow scheduling
- [ ] Multi-executor coordination testing
- [ ] Performance testing with large numbers of schedules
- [ ] Failure scenario testing (database failures, leader changes)

**Deliverables**:
- Fully integrated cron scheduling service
- Complete basic API matching issue #11 simple interface
- Robust multi-executor coordination
- Performance validation under load

**Validation**:
```rust
let executor = UnifiedExecutor::with_config(database_url, config).await?;

// Schedule a simple cron job
let schedule_id = executor.schedule(
    "nightly_etl",
    "0 2 * * *",
    "etl_pipeline"
).await?;

// Verify schedule was created
let schedules = executor.list_schedules().await?;
assert!(schedules.iter().any(|s| s.id == schedule_id));
```

### Phase 3: Advanced Features (Weeks 5-6)

**Goal**: Implement advanced scheduling features and programmatic API

#### 3.1 Schedule Builder Pattern
- [ ] Implement `Schedule` builder with fluent API
- [ ] Add `daily()`, `weekly()`, `monthly()` convenience methods
- [ ] Support for `at_hour()`, `at_minute()` chaining
- [ ] Convert builder patterns to cron expressions

#### 3.2 Advanced Configuration
- [ ] Implement `ScheduleConfig` with all options
- [ ] Add timezone specification and validation
- [ ] Support for execution context passing
- [ ] Implement max_concurrent and overlap strategies

#### 3.3 One-time and Conditional Scheduling
- [ ] Add `schedule_once()` for delayed execution
- [ ] Implement conditional scheduling with user predicates
- [ ] Add support for dynamic schedule adjustment
- [ ] Create comprehensive scheduling configuration options

#### 3.4 Enhanced Error Handling
- [ ] Add detailed error types for scheduling failures
- [ ] Implement retry mechanisms for failed schedule evaluations
- [ ] Add monitoring and alerting for schedule health
- [ ] Create comprehensive logging for debugging

**Deliverables**:
- Complete programmatic scheduling API
- Advanced configuration options
- One-time and conditional scheduling capabilities
- Production-ready error handling and monitoring

**Validation**:
```rust
// Test builder pattern
let schedule = Schedule::daily().at_hour(3).at_minute(30);
executor.add_schedule("daily_backup", schedule, "backup_workflow").await?;

// Test advanced configuration
executor.schedule_workflow(ScheduleConfig {
    name: "weekly_cleanup",
    cron: "0 0 * * 0",
    workflow: "cleanup_pipeline",
    timezone: "America/New_York",
    max_concurrent: 1,
    context: json!({"retention_days": 7})
}).await?;

// Test one-time scheduling
executor.schedule_once(
    "delayed_notification",
    Duration::from_hours(24),
    "send_reminder"
).await?;
```

### Phase 4: Optimization & Production Features (Weeks 7-8)

**Goal**: Production-ready optimizations and advanced features

#### 4.1 Performance Optimization
- [ ] Implement connection pooling optimizations for scheduling
- [ ] Add batch processing optimizations for large schedule counts
- [ ] Optimize database queries with advanced indexing
- [ ] Add memory usage optimization for long-running schedulers

#### 4.2 Catchup and Recovery
- [ ] Implement catchup policies (skip, run_once, run_all)
- [ ] Add missed schedule detection and handling
- [ ] Create recovery mechanisms for scheduler failures
- [ ] Add comprehensive health checking and monitoring

#### 4.3 Schedule Management APIs
- [ ] Implement full CRUD operations for schedules
- [ ] Add schedule history and execution tracking
- [ ] Create schedule pause/resume functionality
- [ ] Add bulk operations for schedule management

#### 4.4 Monitoring and Observability
- [ ] Add metrics for schedule evaluation performance
- [ ] Implement logging for schedule lifecycle events
- [ ] Create dashboards for schedule monitoring
- [ ] Add alerting for failed or missed schedules

**Deliverables**:
- Production-optimized scheduling performance
- Comprehensive schedule management capabilities
- Full monitoring and observability features
- Robust recovery and catchup mechanisms

**Validation**:
- Performance benchmarks showing <1% overhead on workflow execution
- Successful handling of 10,000+ concurrent schedules
- Recovery testing from various failure scenarios
- Complete monitoring and alerting functionality

### Phase 5: Documentation & Polish (Weeks 9-10)

**Goal**: Complete documentation and final polish for production release

#### 5.1 Comprehensive Documentation
- [ ] Create getting started guide for cron scheduling
- [ ] Add API reference documentation with examples
- [ ] Write timezone handling and DST best practices guide
- [ ] Create troubleshooting and FAQ documentation

#### 5.2 Examples and Tutorials
- [ ] Create comprehensive example workflows using scheduling
- [ ] Add migration guides from external cron systems
- [ ] Write advanced scheduling pattern tutorials
- [ ] Create performance tuning and optimization guides

#### 5.3 Testing and Quality Assurance
- [ ] Comprehensive integration testing across all features
- [ ] Performance testing and benchmarking
- [ ] Security review of scheduling implementation
- [ ] Documentation review and validation

#### 5.4 Release Preparation
- [ ] Version management and changelog preparation
- [ ] Migration guides for existing users
- [ ] Performance benchmarks and optimization reports
- [ ] Final API review and stabilization

**Deliverables**:
- Complete user documentation integrated with Hugo site
- Comprehensive examples and tutorials
- Production-ready scheduling implementation
- Performance-validated 0.2.0 release with cron scheduling

**Validation**:
```bash
# Complete example from documentation works exactly as specified
cargo run --example cron_scheduling

# Performance validation
cargo bench cron_scheduling

# Integration testing
cargo test cron_integration --release
```

---

## Risk Assessment & Mitigation

### Technical Risks

**1. Database Performance Under Load**
- *Risk*: Large numbers of schedules could impact database performance
- *Mitigation*: Optimized indexing, batch processing, performance testing
- *Fallback*: Implement schedule sharding or caching layer

**2. Timezone Complexity**
- *Risk*: DST transitions and timezone handling edge cases
- *Mitigation*: Comprehensive testing, use proven chrono-tz library
- *Fallback*: UTC-only mode for simplicity

**3. Multi-Executor Coordination**
- *Risk*: Race conditions or split-brain scenarios in leader election
- *Mitigation*: Database-backed coordination, lease-based leadership
- *Fallback*: Single-executor mode for critical deployments

### Project Risks

**1. Feature Creep**
- *Risk*: Requests for additional scheduling features beyond issue #11
- *Mitigation*: Strict adherence to issue requirements
- *Fallback*: Document future enhancements for v0.3.0

**2. Backward Compatibility**
- *Risk*: Changes affecting existing workflow execution
- *Mitigation*: Opt-in features, comprehensive testing
- *Fallback*: Feature flags for scheduling functionality

**3. Performance Regression**
- *Risk*: Scheduling overhead impacting workflow performance
- *Mitigation*: Continuous benchmarking, separate polling loops
- *Fallback*: Disable scheduling if performance targets not met

---

## Success Criteria

### Functional Requirements
1. **API Compliance**: Exact implementation of all interfaces from issue #11
2. **Reliability**: 99.9% schedule accuracy under normal conditions
3. **Performance**: <1% overhead on existing workflow execution
4. **Scalability**: Support for 10,000+ concurrent schedules
5. **Multi-Executor**: Seamless operation across multiple executor instances

### Technical Requirements
1. **Database Integration**: Direct PostgreSQL/SQLite usage (confirmed requirement)
2. **Timezone Support**: Full DST-aware scheduling across all timezones
3. **Error Handling**: Comprehensive failure recovery and retry mechanisms
4. **Documentation**: Complete integration with existing Hugo documentation
5. **Testing**: >95% code coverage with comprehensive integration tests

### Production Readiness
1. **Monitoring**: Complete observability for schedule health and performance
2. **Management**: Full CRUD operations for schedule lifecycle
3. **Migration**: Clear upgrade path for existing users
4. **Security**: No introduction of new security vulnerabilities
5. **Stability**: No breaking changes to existing workflow execution

---

## Timeline & Milestones

| Phase | Duration | Key Milestone | Success Criteria |
|-------|----------|---------------|------------------|
| 1 | Weeks 1-2 | Database foundation | Cron expressions evaluate correctly |
| 2 | Weeks 3-4 | Basic scheduling | Simple schedule() API works |
| 3 | Weeks 5-6 | Advanced features | All issue #11 APIs implemented |
| 4 | Weeks 7-8 | Production optimization | Performance targets achieved |
| 5 | Weeks 9-10 | Documentation complete | Ready for production release |

**Critical Path**: Phase 1 → Phase 2 → Phase 3
**Parallel Work**: Documentation can be developed alongside Phase 3-4

---

## Next Steps

1. **Immediate**: Begin Phase 1 - Database schema and cron evaluation
2. **Week 2**: Complete foundation and validate cron parsing
3. **Week 4**: Have working basic scheduling API
4. **Week 6**: Complete all advanced features from issue #11
5. **Week 8**: Production-ready optimizations complete
6. **Week 10**: Ready for release with complete documentation

**Success Metrics**:
- ✅ All code examples from issue #11 work exactly as specified
- ✅ Performance overhead measured and optimized to <1%
- ✅ Reliable operation with 10,000+ concurrent schedules
- ✅ Seamless integration with existing Cloacina architecture
- ✅ Zero breaking changes to existing functionality

---

**Last Updated**: 2025-05-29
**Status**: Ready for implementation
**Architecture**: Database-first job queue pattern confirmed
**API Compliance**: ✅ Exact match to issue #11 requirements
