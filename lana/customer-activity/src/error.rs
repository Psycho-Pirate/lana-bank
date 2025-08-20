use thiserror::Error;

#[derive(Error, Debug)]
pub enum CustomerActivityError {
    #[error("CustomerActivityError - Sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("CustomerActivityError - JobError: {0}")]
    Job(#[from] ::job::error::JobError),
}
