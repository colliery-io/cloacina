# cloacina::dal::unified::schedule::crud <span class="plissken-badge plissken-badge-source" style="display: inline-block; padding: 0.1em 0.35em; font-size: 0.55em; font-weight: 600; border-radius: 0.2em; vertical-align: middle; background: #ff5722; color: white;">Rust</span>


Backend-divergent CRUD operations for unified schedules (CLOACI-I-0135).

Every other schedule DAL method is backend-agnostic diesel and lives inline
in the public methods (`mod.rs`) via `interact_on_backend!`. `claim_and_update_cron`
is the one method whose bodies genuinely diverge by backend: the Postgres arm
issues `SET TRANSACTION ISOLATION LEVEL SERIALIZABLE` to make the claim
atomic under concurrent schedulers, which has no SQLite equivalent (SQLite's
single-writer model already serializes the update). It therefore stays an
explicit `*_postgres`/`*_sqlite` twin pair.
