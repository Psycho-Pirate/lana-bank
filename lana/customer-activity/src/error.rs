use thiserror::Error;

#[derive(Error, Debug)]
pub enum CustomerActivityError {
    #[error("CustomerActivityError - DatabaseError: {0}")]
    DatabaseError(#[from] sqlx::Error),
}
