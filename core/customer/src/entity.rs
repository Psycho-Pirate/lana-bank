use chrono::{DateTime, Utc};
use derive_builder::Builder;
#[cfg(feature = "json-schema")]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use audit::AuditInfo;
use chrono::Duration;
use es_entity::*;

use crate::config::CustomerConfig;
use crate::primitives::*;

#[derive(EsEvent, Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "json-schema", derive(JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
#[es_event(id = "CustomerId")]
pub enum CustomerEvent {
    Initialized {
        id: CustomerId,
        email: String,
        telegram_id: String,
        customer_type: CustomerType,
        public_id: PublicId,
        audit_info: AuditInfo,
    },
    AuthenticationIdUpdated {
        authentication_id: AuthenticationId,
    },
    KycStarted {
        applicant_id: String,
        audit_info: AuditInfo,
    },
    KycApproved {
        applicant_id: String,
        level: KycLevel,
        audit_info: AuditInfo,
    },
    KycDeclined {
        applicant_id: String,
        audit_info: AuditInfo,
    },
    AccountStatusUpdated {
        status: AccountStatus,
        audit_info: AuditInfo,
    },
    ActivityRecorded {
        activity_type: ActivityType,
        audit_info: AuditInfo,
    },
    TelegramIdUpdated {
        telegram_id: String,
        audit_info: AuditInfo,
    },
    EmailUpdated {
        email: String,
        audit_info: AuditInfo,
    },
}

#[derive(EsEntity, Builder)]
#[builder(pattern = "owned", build_fn(error = "EsEntityError"))]
pub struct Customer {
    pub id: CustomerId,
    #[builder(setter(strip_option), default)]
    pub authentication_id: Option<AuthenticationId>,
    pub email: String,
    pub telegram_id: String,
    #[builder(default)]
    pub status: AccountStatus,
    pub level: KycLevel,
    pub customer_type: CustomerType,
    #[builder(setter(strip_option, into), default)]
    pub applicant_id: Option<String>,
    pub public_id: PublicId,
    #[builder(setter(strip_option), default)]
    pub last_activity: Option<DateTime<Utc>>,
    events: EntityEvents<CustomerEvent>,
}

impl core::fmt::Display for Customer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Customer: {}, email: {}, customer_type: {}",
            self.id, self.email, self.customer_type
        )
    }
}

impl Customer {
    pub fn created_at(&self) -> DateTime<Utc> {
        self.events
            .entity_first_persisted_at()
            .expect("entity_first_persisted_at not found")
    }

    pub fn may_create_loan(&self) -> bool {
        true
    }

    pub fn update_authentication_id(
        &mut self,
        authentication_id: AuthenticationId,
    ) -> Idempotent<()> {
        idempotency_guard!(
            self.events.iter_all(),
            CustomerEvent::AuthenticationIdUpdated { authentication_id: existing_id } if existing_id == &authentication_id
        );
        self.authentication_id = Some(authentication_id);
        self.events
            .push(CustomerEvent::AuthenticationIdUpdated { authentication_id });
        Idempotent::Executed(())
    }

    pub fn start_kyc(&mut self, applicant_id: String, audit_info: AuditInfo) {
        self.events.push(CustomerEvent::KycStarted {
            applicant_id: applicant_id.clone(),
            audit_info,
        });
        self.applicant_id = Some(applicant_id);
    }

    pub fn approve_kyc(
        &mut self,
        level: KycLevel,
        applicant_id: String,
        audit_info: AuditInfo,
    ) -> Idempotent<()> {
        idempotency_guard!(
            self.events.iter_all().rev(),
            CustomerEvent::KycApproved { .. },
            => CustomerEvent::KycDeclined { .. }
        );
        self.events.push(CustomerEvent::KycApproved {
            level,
            applicant_id: applicant_id.clone(),
            audit_info: audit_info.clone(),
        });

        self.applicant_id = Some(applicant_id);
        self.level = KycLevel::Basic;

        self.update_account_status(AccountStatus::Active, audit_info)
    }

    pub fn decline_kyc(&mut self, applicant_id: String, audit_info: AuditInfo) -> Idempotent<()> {
        idempotency_guard!(
            self.events.iter_all().rev(),
            CustomerEvent::KycDeclined { .. },
            => CustomerEvent::KycApproved { .. }
        );
        self.events.push(CustomerEvent::KycDeclined {
            applicant_id,
            audit_info: audit_info.clone(),
        });
        self.level = KycLevel::NotKyced;
        self.update_account_status(AccountStatus::Inactive, audit_info)
    }

    fn update_account_status(
        &mut self,
        status: AccountStatus,
        audit_info: AuditInfo,
    ) -> Idempotent<()> {
        idempotency_guard!(
            self.events.iter_all().rev(),
            CustomerEvent::AccountStatusUpdated { status: existing_status, .. } if existing_status == &status
        );
        self.events
            .push(CustomerEvent::AccountStatusUpdated { status, audit_info });
        self.status = status;
        Idempotent::Executed(())
    }

    pub fn update_telegram_id(
        &mut self,
        new_telegram_id: String,
        audit_info: AuditInfo,
    ) -> Idempotent<()> {
        idempotency_guard!(
            self.events.iter_all().rev(),
            CustomerEvent::TelegramIdUpdated { telegram_id: existing_telegram_id , ..} if existing_telegram_id == &new_telegram_id
        );
        self.events.push(CustomerEvent::TelegramIdUpdated {
            telegram_id: new_telegram_id.clone(),
            audit_info,
        });
        self.telegram_id = new_telegram_id;
        Idempotent::Executed(())
    }

    pub fn update_email(&mut self, new_email: String, audit_info: AuditInfo) -> Idempotent<()> {
        idempotency_guard!(
            self.events.iter_all().rev(),
            CustomerEvent::EmailUpdated { email: existing_email, .. } if existing_email == &new_email,
            => CustomerEvent::EmailUpdated { .. }
        );
        self.events.push(CustomerEvent::EmailUpdated {
            email: new_email.clone(),
            audit_info,
        });
        self.email = new_email;
        Idempotent::Executed(())
    }

    pub fn inactivity_threshold(&self) -> Duration {
        let config = CustomerConfig::default();
        Duration::days(config.inactivity_threshold_days as i64)
    }

    pub fn record_activity(
        &mut self,
        activity_type: ActivityType,
        audit_info: AuditInfo,
    ) -> Idempotent<()> {
        idempotency_guard!(
            self.events.iter_all().rev(),
            CustomerEvent::ActivityRecorded { activity_type: existing_activity_type, .. } if existing_activity_type == &activity_type
        );

        let now = Utc::now();
        self.last_activity = Some(now);

        self.events.push(CustomerEvent::ActivityRecorded {
            activity_type,
            audit_info: audit_info.clone(),
        });

        Idempotent::Executed(())
    }

    // TODO: Add EscheatmentCandidate
    pub fn get_account_status(&self, inactivity_threshold: Duration) -> AccountStatus {
        let now = Utc::now();

        if let Some(last_activity) = self.last_activity {
            let time_since_activity = now - last_activity;
            if time_since_activity >= inactivity_threshold {
                AccountStatus::Inactive
            } else {
                AccountStatus::Active
            }
        } else {
            AccountStatus::Inactive
        }
    }

    pub fn get_effective_account_status(&self) -> AccountStatus {
        // Even with recent activity, if KYC has set the account to inactive, effective status should be inactive
        if self.status == AccountStatus::Inactive {
            return AccountStatus::Inactive;
        }

        self.get_account_status(self.inactivity_threshold())
    }
}

impl TryFromEvents<CustomerEvent> for Customer {
    fn try_from_events(events: EntityEvents<CustomerEvent>) -> Result<Self, EsEntityError> {
        let mut builder = CustomerBuilder::default();

        for event in events.iter_all() {
            match event {
                CustomerEvent::Initialized {
                    id,
                    email,
                    telegram_id,
                    customer_type,
                    public_id,
                    ..
                } => {
                    builder = builder
                        .id(*id)
                        .email(email.clone())
                        .telegram_id(telegram_id.clone())
                        .customer_type(*customer_type)
                        .public_id(public_id.clone())
                        .level(KycLevel::NotKyced);
                }
                CustomerEvent::AuthenticationIdUpdated { authentication_id } => {
                    builder = builder.authentication_id(*authentication_id);
                }
                CustomerEvent::KycStarted { applicant_id, .. } => {
                    builder = builder.applicant_id(applicant_id.clone());
                }
                CustomerEvent::KycApproved {
                    level,
                    applicant_id,
                    ..
                } => builder = builder.applicant_id(applicant_id.clone()).level(*level),
                CustomerEvent::KycDeclined { applicant_id, .. } => {
                    builder = builder.applicant_id(applicant_id.clone())
                }
                CustomerEvent::AccountStatusUpdated { status, .. } => {
                    builder = builder.status(*status);
                }
                CustomerEvent::TelegramIdUpdated { telegram_id, .. } => {
                    builder = builder.telegram_id(telegram_id.clone());
                }
                CustomerEvent::EmailUpdated { email, .. } => {
                    builder = builder.email(email.clone());
                }
                CustomerEvent::ActivityRecorded { .. } => {}
            }
        }

        builder.events(events).build()
    }
}

#[derive(Debug, Builder)]
pub struct NewCustomer {
    #[builder(setter(into))]
    pub(super) id: CustomerId,
    #[builder(setter(into))]
    pub(super) email: String,
    #[builder(setter(into))]
    pub(super) telegram_id: String,
    #[builder(setter(into))]
    pub(super) customer_type: CustomerType,
    #[builder(setter(skip), default)]
    pub(super) status: AccountStatus,
    #[builder(setter(into))]
    pub(super) public_id: PublicId,
    pub(super) audit_info: AuditInfo,
    #[builder(default)]
    pub(super) last_activity: Option<DateTime<Utc>>,
}

impl NewCustomer {
    pub fn builder() -> NewCustomerBuilder {
        NewCustomerBuilder::default()
    }
}

impl IntoEvents<CustomerEvent> for NewCustomer {
    fn into_events(self) -> EntityEvents<CustomerEvent> {
        EntityEvents::init(
            self.id,
            [CustomerEvent::Initialized {
                id: self.id,
                email: self.email,
                telegram_id: self.telegram_id,
                customer_type: self.customer_type,
                public_id: self.public_id,
                audit_info: self.audit_info,
            }],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn create_test_customer() -> Customer {
        let audit_info = AuditInfo::from((audit::AuditEntryId::from(1), "System".to_string()));

        let events = EntityEvents::init(
            CustomerId::new(),
            [CustomerEvent::Initialized {
                id: CustomerId::new(),
                email: "test@example.com".to_string(),
                telegram_id: "test_telegram".to_string(),
                customer_type: CustomerType::Individual,
                public_id: PublicId::new("test-public-id"),
                audit_info,
            }],
        );

        Customer::try_from_events(events).unwrap()
    }

    #[test]
    fn test_record_activity_updates_last_activity() {
        let mut customer = create_test_customer();
        let audit_info = AuditInfo::from((audit::AuditEntryId::from(2), "System".to_string()));

        // Initially, last_activity should be None
        assert!(customer.last_activity.is_none());

        // Record activity
        let result = customer.record_activity(ActivityType::AccountView, audit_info);
        assert!(result.did_execute());

        // last_activity should now be set
        assert!(customer.last_activity.is_some());

        let recorded_time = customer.last_activity.unwrap();
        let now = Utc::now();

        // The recorded time should be very recent (within 5 second)
        assert!(now - recorded_time < Duration::seconds(5));
    }

    #[test]
    fn test_get_account_status_with_no_activity() {
        let customer = create_test_customer();

        // Customer with no activity should be inactive
        let status = customer.get_account_status(Duration::days(365));
        assert_eq!(status, AccountStatus::Inactive);
    }

    #[test]
    fn test_get_account_status_with_recent_activity() {
        let mut customer = create_test_customer();
        let audit_info = AuditInfo::from((audit::AuditEntryId::from(3), "System".to_string()));

        // Record recent activity
        customer
            .record_activity(ActivityType::AccountView, audit_info)
            .unwrap();

        // Customer with recent activity should be active
        let status = customer.get_account_status(Duration::days(365));
        assert_eq!(status, AccountStatus::Active);
    }

    #[test]
    fn test_get_account_status_with_old_activity() {
        let mut customer = create_test_customer();

        // Set last_activity to 2 years ago
        customer.last_activity = Some(Utc::now() - Duration::days(730));

        // Customer with activity older than 1 year should be inactive
        let status = customer.get_account_status(Duration::days(365));
        assert_eq!(status, AccountStatus::Inactive);
    }

    #[test]
    fn test_get_effective_account_status_with_kyc_inactive() {
        let mut customer = create_test_customer();
        let audit_info = AuditInfo::from((audit::AuditEntryId::from(6), "System".to_string()));

        // Set KYC status to inactive
        customer.status = AccountStatus::Inactive;

        // Record recent activity
        customer
            .record_activity(ActivityType::AccountView, audit_info)
            .unwrap();

        // Even with recent activity, if KYC is inactive, effective status should be inactive
        let status = customer.get_effective_account_status();
        assert_eq!(status, AccountStatus::Inactive);
    }

    #[test]
    fn test_get_effective_account_status_with_kyc_active() {
        let mut customer = create_test_customer();
        let audit_info = AuditInfo::from((audit::AuditEntryId::from(7), "System".to_string()));

        // Set KYC status to active
        customer.status = AccountStatus::Active;

        // Record recent activity
        customer
            .record_activity(ActivityType::AccountView, audit_info)
            .unwrap();

        // With active KYC and recent activity, effective status should be active
        let status = customer.get_effective_account_status();
        assert_eq!(status, AccountStatus::Active);
    }

    #[test]
    fn test_activity_recorded_event_is_created() {
        let mut customer = create_test_customer();
        let audit_info = AuditInfo::from((audit::AuditEntryId::from(8), "System".to_string()));

        let initial_event_count = customer.events.iter_all().count();

        // Record activity
        customer
            .record_activity(ActivityType::Transaction, audit_info.clone())
            .unwrap();

        // Should have one more event
        assert_eq!(customer.events.iter_all().count(), initial_event_count + 1);

        // Check that the last event is ActivityRecorded
        let last_event = customer.events.iter_all().last().unwrap();
        match last_event {
            CustomerEvent::ActivityRecorded { activity_type, .. } => {
                assert_eq!(*activity_type, ActivityType::Transaction);
            }
            _ => panic!("Expected ActivityRecorded event"),
        }
    }

    #[test]
    fn test_idempotency_of_record_activity() {
        let mut customer = create_test_customer();
        let audit_info = AuditInfo::from((audit::AuditEntryId::from(9), "System".to_string()));

        // Record activity first time
        let result1 = customer.record_activity(ActivityType::AccountView, audit_info.clone());
        assert!(result1.did_execute());

        let first_activity_time = customer.last_activity.unwrap();

        // Record same activity again
        let result2 = customer.record_activity(ActivityType::AccountView, audit_info);
        assert!(!result2.did_execute());

        // Activity time should not have changed
        assert_eq!(customer.last_activity.unwrap(), first_activity_time);
    }
}
