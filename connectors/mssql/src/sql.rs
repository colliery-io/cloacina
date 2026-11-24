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

//! The MSSQL connector's statements (CLOACINA-T-0984). Pure, so the unit lane
//! tests them: `cloacina-sync` compiles this file into its unit tests.
//!
//! Reads use SQL Server's `FOR JSON PATH`, so the server produces the JSON
//! rows. Identifiers are `[bracket]`-quoted.

/// Bracket-quote an identifier, doubling `]`.
pub fn quote_ident(name: &str) -> String {
    format!("[{}]", name.replace(']', "]]"))
}

/// `schema.table` becomes `[schema].[table]`; a bare name becomes `[table]`.
pub fn qualified_table(name: &str) -> String {
    match name.split_once('.') {
        Some((schema, table)) => format!("{}.{}", quote_ident(schema), quote_ident(table)),
        None => quote_ident(name),
    }
}

/// Full refresh: the whole table as a JSON array. SQL Server returns no rows
/// for an empty result, which the guest reads as `[]`.
pub fn full_select_json_sql(table: &str) -> String {
    format!(
        "SELECT * FROM {} FOR JSON PATH, INCLUDE_NULL_VALUES",
        qualified_table(table)
    )
}

/// Incremental: the rows past the cursor, ordered by it, as a JSON array.
/// Without a cursor (a first read) there is no `WHERE`. The cursor is bound as
/// text to `@P1`, and SQL Server converts it to the column's type for the
/// comparison, so an integer cursor at `9` still reads `10`. A text
/// comparison would skip it, and the cursor would pass an unread record.
pub fn incremental_select_json_sql(table: &str, cursor_col: &str, has_cursor: bool) -> String {
    let t = qualified_table(table);
    let c = quote_ident(cursor_col);
    let where_clause = if has_cursor {
        format!(" WHERE {c} > @P1")
    } else {
        String::new()
    };
    format!("SELECT * FROM {t}{where_clause} ORDER BY {c} FOR JSON PATH, INCLUDE_NULL_VALUES")
}

/// The base tables, schema-qualified, as a JSON array of `{name}`, read back
/// through the same path as the data reads.
pub const DISCOVER_TABLES_SQL: &str = "SELECT TABLE_SCHEMA + '.' + TABLE_NAME AS name \
     FROM INFORMATION_SCHEMA.TABLES WHERE TABLE_TYPE = 'BASE TABLE' \
     ORDER BY TABLE_SCHEMA, TABLE_NAME FOR JSON PATH, INCLUDE_NULL_VALUES";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_bracket_quote_and_escape() {
        assert_eq!(quote_ident("Orders"), "[Orders]");
        assert_eq!(quote_ident("weird]name"), "[weird]]name]");
        assert_eq!(qualified_table("sales.Orders"), "[sales].[Orders]");
        assert_eq!(qualified_table("Orders"), "[Orders]");
    }

    #[test]
    fn full_and_incremental_selects() {
        assert_eq!(
            full_select_json_sql("dbo.Contacts"),
            "SELECT * FROM [dbo].[Contacts] FOR JSON PATH, INCLUDE_NULL_VALUES"
        );
        assert_eq!(
            incremental_select_json_sql("dbo.Contacts", "updated_at", false),
            "SELECT * FROM [dbo].[Contacts] ORDER BY [updated_at] FOR JSON PATH, INCLUDE_NULL_VALUES"
        );
        // The column is compared in its own type, never cast to text.
        assert_eq!(
            incremental_select_json_sql("dbo.Contacts", "updated_at", true),
            "SELECT * FROM [dbo].[Contacts] WHERE [updated_at] > @P1 \
             ORDER BY [updated_at] FOR JSON PATH, INCLUDE_NULL_VALUES"
        );
    }

    #[test]
    fn discover_lists_base_tables_as_json() {
        assert!(DISCOVER_TABLES_SQL.contains("FOR JSON PATH"));
        assert!(DISCOVER_TABLES_SQL.contains("TABLE_TYPE = 'BASE TABLE'"));
    }
}
