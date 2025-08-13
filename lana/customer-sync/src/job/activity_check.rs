use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use tracing::instrument;

use audit::AuditSvc;
use authz::PermissionCheck;
use core_customer::{CoreCustomerAction, CoreCustomerEvent, CustomerObject, Customers};
use core_deposit::{
    CoreDepositAction, CoreDepositEvent, CoreDepositObject, GovernanceAction, GovernanceObject,
};

use es_entity::prelude::sqlx;
use governance::GovernanceEvent;
use lana_events::LanaEvent;
use outbox::OutboxEventMarker;

use job::*;

use crate::config::*;
use customer_activity::CustomerActivityService;

#[derive(serde::Serialize)]
pub struct CustomerActivityCheckJobConfig<Perms, E> {
    _phantom: std::marker::PhantomData<(Perms, E)>,
}

impl<Perms, E> CustomerActivityCheckJobConfig<Perms, E> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<Perms, E> JobConfig for CustomerActivityCheckJobConfig<Perms, E>
where
    Perms: PermissionCheck,
    <<Perms as PermissionCheck>::Audit as AuditSvc>::Action:
        From<CoreCustomerAction> + From<CoreDepositAction> + From<GovernanceAction>,
    <<Perms as PermissionCheck>::Audit as AuditSvc>::Object:
        From<CustomerObject> + From<CoreDepositObject> + From<GovernanceObject>,
    E: OutboxEventMarker<LanaEvent>
        + OutboxEventMarker<CoreCustomerEvent>
        + OutboxEventMarker<CoreDepositEvent>
        + OutboxEventMarker<GovernanceEvent>,
{
    type Initializer = CustomerActivityCheckInit<Perms, E>;
}

pub struct CustomerActivityCheckInit<Perms, E>
where
    Perms: PermissionCheck,
    E: OutboxEventMarker<LanaEvent>
        + OutboxEventMarker<CoreCustomerEvent>
        + OutboxEventMarker<CoreDepositEvent>
        + OutboxEventMarker<GovernanceEvent>,
{
    customers: Customers<Perms, E>,
    customer_activity: CustomerActivityService,
    config: CustomerSyncConfig,
}

impl<Perms, E> CustomerActivityCheckInit<Perms, E>
where
    Perms: PermissionCheck,
    E: OutboxEventMarker<LanaEvent>
        + OutboxEventMarker<CoreCustomerEvent>
        + OutboxEventMarker<CoreDepositEvent>
        + OutboxEventMarker<GovernanceEvent>,
{
    pub fn new(
        customers: &Customers<Perms, E>,
        pool: sqlx::PgPool,
        config: CustomerSyncConfig,
    ) -> Self {
        Self {
            customers: customers.clone(),
            customer_activity: CustomerActivityService::new(pool),
            config,
        }
    }
}

const CUSTOMER_ACTIVITY_CHECK: JobType = JobType::new("customer-activity-check");

impl<Perms, E> JobInitializer for CustomerActivityCheckInit<Perms, E>
where
    Perms: PermissionCheck,
    <<Perms as PermissionCheck>::Audit as AuditSvc>::Action:
        From<CoreCustomerAction> + From<CoreDepositAction> + From<GovernanceAction>,
    <<Perms as PermissionCheck>::Audit as AuditSvc>::Object:
        From<CustomerObject> + From<CoreDepositObject> + From<GovernanceObject>,
    E: OutboxEventMarker<LanaEvent>
        + OutboxEventMarker<CoreCustomerEvent>
        + OutboxEventMarker<CoreDepositEvent>
        + OutboxEventMarker<GovernanceEvent>,
{
    fn job_type() -> JobType
    where
        Self: Sized,
    {
        CUSTOMER_ACTIVITY_CHECK
    }

    fn init(&self, _: &Job) -> Result<Box<dyn JobRunner>, Box<dyn std::error::Error>> {
        Ok(Box::new(CustomerActivityCheckJobRunner {
            customers: self.customers.clone(),
            customer_activity: self.customer_activity.clone(),
            config: self.config.clone(),
        }))
    }

    fn retry_on_error_settings() -> RetrySettings
    where
        Self: Sized,
    {
        RetrySettings::repeat_indefinitely()
    }
}

pub struct CustomerActivityCheckJobRunner<Perms, E>
where
    Perms: PermissionCheck,
    E: OutboxEventMarker<LanaEvent>
        + OutboxEventMarker<CoreCustomerEvent>
        + OutboxEventMarker<CoreDepositEvent>
        + OutboxEventMarker<GovernanceEvent>,
{
    customers: Customers<Perms, E>,
    customer_activity: CustomerActivityService,
    config: CustomerSyncConfig,
}

#[async_trait]
impl<Perms, E> JobRunner for CustomerActivityCheckJobRunner<Perms, E>
where
    Perms: PermissionCheck,
    <<Perms as PermissionCheck>::Audit as AuditSvc>::Action:
        From<CoreCustomerAction> + From<CoreDepositAction> + From<GovernanceAction>,
    <<Perms as PermissionCheck>::Audit as AuditSvc>::Object:
        From<CustomerObject> + From<CoreDepositObject> + From<GovernanceObject>,
    E: OutboxEventMarker<LanaEvent>
        + OutboxEventMarker<CoreCustomerEvent>
        + OutboxEventMarker<CoreDepositEvent>
        + OutboxEventMarker<GovernanceEvent>,
{
    #[instrument(name = "customer_activity_check.run", skip(self, _current_job), err)]
    async fn run(
        &self,
        _current_job: CurrentJob,
    ) -> Result<JobCompletion, Box<dyn std::error::Error>> {
        let now = Utc::now();

        if !self.config.activity_check_enabled {
            let next_run = calculate_next_run_time(
                now,
                self.config.activity_check_hour,
                self.config.activity_check_minute,
            );
            return Ok(JobCompletion::RescheduleAt(next_run));
        }

        self.perform_activity_check().await?;

        let next_run = calculate_next_run_time(
            now,
            self.config.activity_check_hour,
            self.config.activity_check_minute,
        );
        Ok(JobCompletion::RescheduleAt(next_run))
    }
}

impl<Perms, E> CustomerActivityCheckJobRunner<Perms, E>
where
    Perms: PermissionCheck,
    <<Perms as PermissionCheck>::Audit as AuditSvc>::Action:
        From<CoreCustomerAction> + From<CoreDepositAction> + From<GovernanceAction>,
    <<Perms as PermissionCheck>::Audit as AuditSvc>::Object:
        From<CustomerObject> + From<CoreDepositObject> + From<GovernanceObject>,
    E: OutboxEventMarker<LanaEvent>
        + OutboxEventMarker<CoreCustomerEvent>
        + OutboxEventMarker<CoreDepositEvent>
        + OutboxEventMarker<GovernanceEvent>,
{
    #[instrument(name = "customer_activity_check.perform_check", skip(self), err)]
    async fn perform_activity_check(&self) -> Result<(), Box<dyn std::error::Error>> {
        let now = Utc::now();
        let inactive_threshold = now - Duration::days(self.config.inactive_threshold_days);
        let escheatment_threshold = now - Duration::days(self.config.escheatment_threshold_days);

        const BATCH_SIZE: usize = 100;
        let mut query = es_entity::PaginatedQueryArgs {
            first: BATCH_SIZE,
            after: None,
        };

        loop {
            let result = self
                .customers
                .list_customers_for_system_operation(query)
                .await?;

            for customer in &result.entities {
                let last_activity_date = self
                    .customer_activity
                    .load_activity_by_customer_id(customer.id)
                    .await?;

                let new_activity = match last_activity_date {
                    Some(activity) if activity.last_activity_date < escheatment_threshold => {
                        core_customer::AccountActivity::Suspended
                    }
                    Some(activity) if activity.last_activity_date < inactive_threshold => {
                        core_customer::AccountActivity::Disabled
                    }
                    Some(_) => core_customer::AccountActivity::Enabled,
                    None => core_customer::AccountActivity::Disabled,
                };

                if customer.activity != new_activity {
                    self.customers
                        .update_account_activity_from_system(customer.id, new_activity)
                        .await?;
                }
            }

            match result.into_next_query() {
                Some(next_query) => query = next_query,
                None => break,
            }
        }

        Ok(())
    }
}

fn calculate_next_run_time(from_time: DateTime<Utc>, hour: u32, minute: u32) -> DateTime<Utc> {
    let tomorrow = from_time + Duration::days(1);
    tomorrow
        .date_naive()
        .and_hms_opt(hour, minute, 0)
        .unwrap()
        .and_local_timezone(Utc)
        .unwrap()
}
