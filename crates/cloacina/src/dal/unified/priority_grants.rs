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

//! Who set the priority of a package or a schedule (CLOACINA-T-1123).
//!
//! One row per package (by package name) or schedule (by id). It keeps the
//! principal that set the priority and its role then, the priority a
//! schedule asks for, and the priority the last run was given. At each run
//! start the planner gives the run the setter's limit at that time; the row's
//! last given value tells whether that changed.

use super::DAL;
use crate::database::schema::unified::priority_grants;
use crate::error::ValidationError;
use diesel::prelude::*;

/// The kind of a row for a package.
pub const KIND_PACKAGE: &str = "package";
/// The kind of a row for a schedule.
pub const KIND_SCHEDULE: &str = "schedule";

/// The principal that set a priority, and its role then.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrioritySetter {
    /// `key:<id>`, `oidc:<issuer>:<sub>` or `local:<account_id>`.
    pub by: String,
    /// `read`, `write` or `admin`.
    pub role: String,
}

/// One row.
#[derive(Debug, Clone, PartialEq, Eq, Queryable, Selectable)]
#[diesel(table_name = priority_grants)]
pub struct PriorityGrantRow {
    pub kind: String,
    pub target: String,
    pub set_by: String,
    pub set_by_role: String,
    pub asked: Option<i32>,
    pub last_given: Option<i32>,
}

impl PriorityGrantRow {
    /// The setter of the row.
    pub fn setter(&self) -> PrioritySetter {
        PrioritySetter {
            by: self.set_by.clone(),
            role: self.set_by_role.clone(),
        }
    }

    /// The asked priority, on the scale.
    pub fn asked(&self) -> Option<u8> {
        self.asked.map(cloacina_api_types::clamp_priority)
    }

    /// The last given priority, on the scale.
    pub fn last_given(&self) -> Option<u8> {
        self.last_given.map(cloacina_api_types::clamp_priority)
    }
}

#[derive(Insertable)]
#[diesel(table_name = priority_grants)]
struct NewRow {
    kind: String,
    target: String,
    set_by: String,
    set_by_role: String,
    asked: Option<i32>,
    last_given: Option<i32>,
}

/// The DAL of `priority_grants`.
#[derive(Clone)]
pub struct PriorityGrantDAL<'a> {
    dal: &'a DAL,
}

impl<'a> PriorityGrantDAL<'a> {
    pub fn new(dal: &'a DAL) -> Self {
        Self { dal }
    }

    /// Record who set the priority of `kind`/`target`, and what it asks for.
    /// A row there already is replaced, and its last given value cleared:
    /// the next run starts a new history.
    pub async fn record(
        &self,
        kind: &str,
        target: &str,
        setter: &PrioritySetter,
        asked: Option<u8>,
    ) -> Result<(), ValidationError> {
        let row = NewRow {
            kind: kind.to_string(),
            target: target.to_string(),
            set_by: setter.by.clone(),
            set_by_role: setter.role.clone(),
            asked: asked.map(i32::from),
            last_given: None,
        };
        let (kind, target) = (row.kind.clone(), row.target.clone());
        crate::interact_on_backend!(self.dal, |conn| {
            conn.transaction::<_, diesel::result::Error, _>(|conn| {
                diesel::delete(
                    priority_grants::table
                        .filter(priority_grants::kind.eq(&kind))
                        .filter(priority_grants::target.eq(&target)),
                )
                .execute(conn)?;
                diesel::insert_into(priority_grants::table)
                    .values(&row)
                    .execute(conn)
            })
        })?;
        Ok(())
    }

    /// The row of `kind`/`target`, if any.
    pub async fn get(
        &self,
        kind: &str,
        target: &str,
    ) -> Result<Option<PriorityGrantRow>, ValidationError> {
        let (kind, target) = (kind.to_string(), target.to_string());
        let row = crate::interact_on_backend!(self.dal, |conn| {
            priority_grants::table
                .filter(priority_grants::kind.eq(kind))
                .filter(priority_grants::target.eq(target))
                .select(PriorityGrantRow::as_select())
                .first(conn)
                .optional()
        })?;
        Ok(row)
    }

    /// The rows of one kind.
    pub async fn list(&self, kind: &str) -> Result<Vec<PriorityGrantRow>, ValidationError> {
        let kind = kind.to_string();
        let rows = crate::interact_on_backend!(self.dal, |conn| {
            priority_grants::table
                .filter(priority_grants::kind.eq(kind))
                .select(PriorityGrantRow::as_select())
                .load(conn)
        })?;
        Ok(rows)
    }

    /// Set the last given value of `kind`/`target` to `given` if it is not
    /// that already. True for the one caller that changed it: a restart or a
    /// second replica that sees the same value changes nothing.
    pub async fn set_last_given(
        &self,
        kind: &str,
        target: &str,
        given: u8,
    ) -> Result<bool, ValidationError> {
        let (kind, target) = (kind.to_string(), target.to_string());
        let given = i32::from(given);
        let rows = crate::interact_on_backend!(self.dal, |conn| {
            diesel::update(
                priority_grants::table
                    .filter(priority_grants::kind.eq(kind))
                    .filter(priority_grants::target.eq(target))
                    .filter(
                        priority_grants::last_given
                            .is_null()
                            .or(priority_grants::last_given.ne(given)),
                    ),
            )
            .set(priority_grants::last_given.eq(Some(given)))
            .execute(conn)
        })?;
        Ok(rows > 0)
    }

    /// Remove the row of `kind`/`target`.
    pub async fn delete(&self, kind: &str, target: &str) -> Result<(), ValidationError> {
        let (kind, target) = (kind.to_string(), target.to_string());
        crate::interact_on_backend!(self.dal, |conn| {
            diesel::delete(
                priority_grants::table
                    .filter(priority_grants::kind.eq(kind))
                    .filter(priority_grants::target.eq(target)),
            )
            .execute(conn)
        })?;
        Ok(())
    }
}

#[cfg(all(test, feature = "sqlite"))]
mod tests {
    use super::*;
    use crate::database::Database;

    async fn dal() -> DAL {
        let url = format!(
            "file:priority_grants_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4()
        );
        let db = Database::new(&url, "", 5);
        db.run_migrations().await.expect("migrations");
        DAL::new(db)
    }

    fn setter(by: &str, role: &str) -> PrioritySetter {
        PrioritySetter {
            by: by.to_string(),
            role: role.to_string(),
        }
    }

    #[tokio::test]
    async fn a_row_keeps_who_set_it_and_what_it_asks() {
        let dal = dal().await;
        let grants = dal.priority_grants();
        assert_eq!(grants.get(KIND_SCHEDULE, "s1").await.unwrap(), None);
        grants
            .record(KIND_SCHEDULE, "s1", &setter("key:abc", "write"), Some(2))
            .await
            .unwrap();
        let row = grants.get(KIND_SCHEDULE, "s1").await.unwrap().unwrap();
        assert_eq!(row.setter(), setter("key:abc", "write"));
        assert_eq!(row.asked(), Some(2));
        assert_eq!(row.last_given(), None);

        // A new record replaces the row and its history.
        grants.set_last_given(KIND_SCHEDULE, "s1", 5).await.unwrap();
        grants
            .record(KIND_SCHEDULE, "s1", &setter("local:u1", "admin"), Some(1))
            .await
            .unwrap();
        let row = grants.get(KIND_SCHEDULE, "s1").await.unwrap().unwrap();
        assert_eq!(row.setter(), setter("local:u1", "admin"));
        assert_eq!((row.asked(), row.last_given()), (Some(1), None));
        assert_eq!(grants.list(KIND_SCHEDULE).await.unwrap().len(), 1);
        assert!(grants.list(KIND_PACKAGE).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn only_a_change_of_the_given_value_is_reported() {
        let dal = dal().await;
        let grants = dal.priority_grants();
        grants
            .record(KIND_PACKAGE, "orders", &setter("key:abc", "write"), None)
            .await
            .unwrap();
        assert!(
            grants
                .set_last_given(KIND_PACKAGE, "orders", 5)
                .await
                .unwrap()
        );
        assert!(
            !grants
                .set_last_given(KIND_PACKAGE, "orders", 5)
                .await
                .unwrap()
        );
        // A second handle on the same database: a second replica.
        let other = DAL::new(dal.database.clone());
        assert!(
            !other
                .priority_grants()
                .set_last_given(KIND_PACKAGE, "orders", 5)
                .await
                .unwrap()
        );
        assert!(
            grants
                .set_last_given(KIND_PACKAGE, "orders", 2)
                .await
                .unwrap()
        );
        assert!(
            !grants
                .set_last_given(KIND_PACKAGE, "absent", 2)
                .await
                .unwrap()
        );
        grants.delete(KIND_PACKAGE, "orders").await.unwrap();
        assert_eq!(grants.get(KIND_PACKAGE, "orders").await.unwrap(), None);
    }
}
