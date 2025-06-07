"""
Database State Reset Utilities

This module provides utilities for resetting database state between test runs
without requiring full container restarts. Optimized for speed while maintaining
reliability through fallback strategies.
"""

import subprocess
import time
from typing import Optional


class DatabaseResetError(Exception):
    """Raised when database reset operations fail."""
    pass


def run_sql_in_postgres_container(sql: str, database: str = "cloacina", container_name: str = "cloacina-postgres") -> subprocess.CompletedProcess:
    """
    Execute SQL command in the PostgreSQL container.
    
    Args:
        sql: SQL command to execute
        database: Database name to connect to
        container_name: Name of the PostgreSQL container
        
    Returns:
        CompletedProcess result from subprocess.run
        
    Raises:
        DatabaseResetError: If the container is not accessible
    """
    cmd = [
        "docker", "exec", container_name,
        "psql", "-U", "cloacina", "-d", database, "-c", sql
    ]
    
    try:
        return subprocess.run(cmd, capture_output=True, text=True, timeout=30)
    except subprocess.TimeoutExpired:
        raise DatabaseResetError(f"SQL command timed out: {sql[:100]}...")
    except Exception as e:
        raise DatabaseResetError(f"Failed to execute SQL in container: {e}")


def reset_postgres_tables_fast(container_name: str = "cloacina-postgres") -> bool:
    """
    Fast PostgreSQL table reset using TRUNCATE.
    
    This is the fastest method as it only removes data while preserving
    table structure, indexes, and sequences.
    
    Args:
        container_name: Name of the PostgreSQL container
        
    Returns:
        True if successful, False otherwise
    """
    truncate_sql = """
        DO $$ 
        DECLARE 
            r RECORD;
            table_list TEXT := '';
        BEGIN
            -- Build comma-separated list of table names
            FOR r IN (SELECT tablename FROM pg_tables WHERE schemaname = 'public') 
            LOOP
                IF table_list != '' THEN
                    table_list := table_list || ', ';
                END IF;
                table_list := table_list || quote_ident(r.tablename);
            END LOOP;
            
            -- Truncate all tables if any exist
            IF table_list != '' THEN
                EXECUTE 'TRUNCATE TABLE ' || table_list || ' RESTART IDENTITY CASCADE';
                RAISE NOTICE 'Truncated tables: %', table_list;
            ELSE
                RAISE NOTICE 'No tables found to truncate';
            END IF;
        END $$;
    """
    
    try:
        result = run_sql_in_postgres_container(truncate_sql)
        if result.returncode == 0:
            print("✓ Fast table truncation successful")
            return True
        else:
            print(f"Table truncation failed: {result.stderr.strip()}")
            return False
    except DatabaseResetError as e:
        print(f"Fast reset failed: {e}")
        return False


def reset_postgres_connections(container_name: str = "cloacina-postgres") -> bool:
    """
    Terminate all active connections to the cloacina database.
    
    This can help resolve lock issues that prevent table truncation.
    
    Args:
        container_name: Name of the PostgreSQL container
        
    Returns:
        True if successful, False otherwise
    """
    terminate_sql = """
        SELECT pg_terminate_backend(pid)
        FROM pg_stat_activity 
        WHERE datname = 'cloacina' 
        AND pid <> pg_backend_pid()
        AND state = 'active';
    """
    
    try:
        result = run_sql_in_postgres_container(terminate_sql, database="postgres")
        if result.returncode == 0:
            print("✓ Active connections terminated")
            # Brief pause to let connections clean up
            time.sleep(0.5)
            return True
        else:
            print(f"Connection termination failed: {result.stderr.strip()}")
            return False
    except DatabaseResetError as e:
        print(f"Connection reset failed: {e}")
        return False


def reset_postgres_schema(container_name: str = "cloacina-postgres") -> bool:
    """
    Reset PostgreSQL by dropping and recreating the public schema.
    
    This is more thorough than truncation but requires re-running migrations.
    
    Args:
        container_name: Name of the PostgreSQL container
        
    Returns:
        True if successful, False otherwise
    """
    schema_reset_sql = """
        DROP SCHEMA IF EXISTS public CASCADE;
        CREATE SCHEMA public;
        GRANT ALL ON SCHEMA public TO cloacina;
        GRANT ALL ON SCHEMA public TO public;
        COMMENT ON SCHEMA public IS 'standard public schema';
    """
    
    try:
        result = run_sql_in_postgres_container(schema_reset_sql)
        if result.returncode == 0:
            print("✓ Schema recreation successful")
            print("⚠ Note: Database migrations will need to be re-run")
            return True
        else:
            print(f"Schema reset failed: {result.stderr.strip()}")
            return False
    except DatabaseResetError as e:
        print(f"Schema reset failed: {e}")
        return False


def smart_postgres_reset(container_name: str = "cloacina-postgres", max_retries: int = 2) -> bool:
    """
    Intelligent PostgreSQL reset with multiple fallback strategies.
    
    Attempts reset methods in order of speed:
    1. Fast table truncation
    2. Connection reset + table truncation retry
    3. Schema recreation (requires migration re-run)
    
    Args:
        container_name: Name of the PostgreSQL container
        max_retries: Maximum number of truncation retries after connection reset
        
    Returns:
        True if any method succeeded, False if all failed
    """
    print("Attempting smart PostgreSQL reset...")
    
    # Method 1: Try fast table truncation
    if reset_postgres_tables_fast(container_name):
        return True
    
    # Method 2: Reset connections and retry truncation
    print("Fast truncation failed, trying connection reset...")
    if reset_postgres_connections(container_name):
        for attempt in range(max_retries):
            print(f"Retrying truncation (attempt {attempt + 1}/{max_retries})...")
            if reset_postgres_tables_fast(container_name):
                return True
            time.sleep(0.5)  # Brief pause between retries
    
    # Method 3: Schema recreation as last resort
    print("Truncation retries failed, attempting schema recreation...")
    if reset_postgres_schema(container_name):
        return True
    
    print("✗ All PostgreSQL reset methods failed")
    return False


def check_postgres_container_health(container_name: str = "cloacina-postgres") -> bool:
    """
    Check if the PostgreSQL container is running and accessible.
    
    Args:
        container_name: Name of the PostgreSQL container
        
    Returns:
        True if container is healthy, False otherwise
    """
    try:
        # Simple connectivity test
        result = run_sql_in_postgres_container("SELECT 1;", database="postgres")
        return result.returncode == 0
    except DatabaseResetError:
        return False


def get_postgres_table_count(container_name: str = "cloacina-postgres") -> Optional[int]:
    """
    Get the number of tables in the public schema.
    
    Useful for verifying reset operations.
    
    Args:
        container_name: Name of the PostgreSQL container
        
    Returns:
        Number of tables, or None if query failed
    """
    try:
        result = run_sql_in_postgres_container(
            "SELECT COUNT(*) FROM pg_tables WHERE schemaname = 'public';"
        )
        if result.returncode == 0:
            return int(result.stdout.strip().split('\n')[-2].strip())
        return None
    except (DatabaseResetError, ValueError, IndexError):
        return None