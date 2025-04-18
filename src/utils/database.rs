use anyhow::{Context, Result};
use serenity::model::prelude::UserId;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};
use std::{env, error::Error, num::TryFromIntError, str::FromStr};

use crate::Token;

#[derive(Debug)]
pub enum DatabaseError {
    SqliteError(sqlx::error::Error),
    InsufficentTokens,
    IntError(TryFromIntError),
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseError::SqliteError(error) => write!(f, "{}", error),
            DatabaseError::InsufficentTokens => write!(f, "InsufficentTokens"),
            DatabaseError::IntError(error) => write!(f, "{}", error),
        }
    }
}

impl From<sqlx::error::Error> for DatabaseError {
    fn from(value: sqlx::error::Error) -> Self {
        DatabaseError::SqliteError(value)
    }
}

impl From<TryFromIntError> for DatabaseError {
    fn from(value: TryFromIntError) -> Self {
        DatabaseError::IntError(value)
    }
}

impl Error for DatabaseError {}

pub async fn connect_to_db() -> Result<SqlitePool> {
    let opts = SqliteConnectOptions::from_str(
        &env::var("DATABASE_URL").unwrap_or("sqlite://database.db".to_string()),
    )?
    .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
    .create_if_missing(true);

    SqlitePool::connect_with(opts)
        .await
        .context("Failed to open db connection")
}

pub async fn add_tokens(pool: &SqlitePool, user: UserId, tokens: Token) -> Result<()> {
    let user_id_num = i64::try_from(user.get())?;
    let token_amount = i64::try_from(tokens)?;
    sqlx::query!(
        r#"INSERT OR REPLACE INTO users (id, tokens) VALUES (?1, COALESCE((SELECT tokens FROM users WHERE id = ?1), 0) + ?2)"#,
        user_id_num,
        token_amount
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_tokens(pool: &SqlitePool, user: UserId) -> Result<Token, DatabaseError> {
    let user_id_num = i64::try_from(user.get())?;
    let results = sqlx::query!(r#"SELECT tokens FROM users WHERE id = ?"#, user_id_num)
        .fetch_optional(pool)
        .await?;
    match results {
        Some(row) => Ok(row.tokens as u64),
        None => Ok(0),
    }
}

pub async fn remove_tokens(
    pool: &SqlitePool,
    user: UserId,
    tokens: Token,
) -> Result<(), DatabaseError> {
    let user_id_num = i64::try_from(user.get())?;
    let token_amount = i64::try_from(tokens)?;
    let rows_changed = sqlx::query!(
        r#"UPDATE users SET tokens = tokens - ?1 WHERE id = ?2 AND tokens >= tokens"#,
        token_amount,
        user_id_num,
    )
    .execute(pool)
    .await?
    .rows_affected();
    if rows_changed == 0 {
        return Err(DatabaseError::InsufficentTokens);
    }
    Ok(())
}

pub async fn list_leadboard(pool: &SqlitePool) -> Result<Vec<(UserId, Token)>, DatabaseError> {
    let results = sqlx::query!(r#"SELECT id, tokens FROM users ORDER BY tokens DESC LIMIT 10"#)
        .fetch_all(pool)
        .await?;

    let mut leaders = Vec::new();
    for row in results {
        let user_id_num = u64::try_from(row.id)?;
        let token_amount = u64::try_from(row.tokens)?;
        leaders.push((UserId::new(user_id_num), token_amount));
    }

    Ok(leaders)
}
