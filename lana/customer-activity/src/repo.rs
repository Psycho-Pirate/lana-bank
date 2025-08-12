use sqlx::PgPool;
use sqlx::types::uuid;

use crate::primitives::CustomerActivity;
use core_customer::CustomerId;

#[derive(Clone)]
pub struct CustomerActivityRepo {
    pool: PgPool,
}

impl CustomerActivityRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn load_activity_by_customer_id(
        &self,
        customer_id: CustomerId,
    ) -> Result<Option<CustomerActivity>, sqlx::Error> {
        let customer_uuid: uuid::Uuid = customer_id.into();
        let row = sqlx::query!(
            r#"
            SELECT customer_id, last_activity_date, updated_at
            FROM customer_activity 
            WHERE customer_id = $1
            "#,
            customer_uuid
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| CustomerActivity {
            customer_id: CustomerId::from(row.customer_id),
            last_activity_date: row.last_activity_date,
            updated_at: row.updated_at,
        }))
    }

    pub async fn persist_activity(&self, activity: &CustomerActivity) -> Result<(), sqlx::Error> {
        let customer_uuid: uuid::Uuid = activity.customer_id.into();
        sqlx::query!(
            r#"
            INSERT INTO customer_activity (customer_id, last_activity_date, updated_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (customer_id) 
            DO UPDATE SET 
                last_activity_date = EXCLUDED.last_activity_date,
                updated_at = EXCLUDED.updated_at
            "#,
            customer_uuid,
            activity.last_activity_date,
            activity.updated_at
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
