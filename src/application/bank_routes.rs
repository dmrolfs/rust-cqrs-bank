use crate::application::app_state::AppState;
use crate::application::result::OptionalResult;
use crate::application::{ApiError, ACCOUNT_QUERY_VIEW, ACCOUNT_QUERY_VIEW_PAYLOAD};
use crate::errors::BankError;
use crate::model::{bank_account, BankAccount};
use crate::model::{
    AccountId, AtmId, BankAccountAggregate, BankAccountCommand, CheckNumber, EmailAddress,
    MailingAddress,
};
use crate::queries::BankAccountViewProjection;
use crate::BankAccountView;
use axum::extract::rejection::JsonRejection;
use axum::extract::{rejection::PathRejection, Path, State};
use axum::response::IntoResponse;
use axum::routing;
use axum::{Json, Router};
use cqrs_es::persist::ViewRepository;
use itertools::Itertools;
use money2::Money;
use pretty_snowflake::envelope::MetaData;
use pretty_snowflake::Id;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::{fmt, ops};
use utoipa::openapi;
use utoipa::openapi::security::{ClientCredentials, Flow, OAuth2, Scopes, SecurityScheme};
use utoipa::{Modify, OpenApi, ToSchema};
use validator::{Validate, ValidationErrors};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(transparent)]
pub struct ApiMoney(money2::Money);

impl ApiMoney {
    pub const fn new(amount: money2::Money) -> Self {
        Self(amount)
    }

    pub const fn into_inner(self) -> money2::Money {
        self.0
    }
}

impl fmt::Display for ApiMoney {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ops::Deref for ApiMoney {
    type Target = money2::Money;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ApiMoney> for money2::Money {
    fn from(m: ApiMoney) -> Self {
        m.0
    }
}

impl From<money2::Money> for ApiMoney {
    fn from(m: Money) -> Self {
        Self::new(m)
    }
}

impl utoipa::ToSchema for ApiMoney {
    fn schema() -> openapi::Schema {
        openapi::ObjectBuilder::new()
            .property(
                "amount",
                openapi::Object::with_type(openapi::SchemaType::String),
            )
            .required("amount")
            .property(
                "currency",
                openapi::Object::with_type(openapi::SchemaType::String),
            )
            .required("currency")
            .example(Some(
                serde_json::json!({ "amount": "123.56", "currency": "USD" }),
            ))
            .into()
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        create_bank_account,
        serve_bank_account,
        update_email,
        update_mailing_address,
        deposit_amount,
        withdrawal_by_atm,
        withdrawal_by_check,
        serve_all_by_balance,
    ),
    components(
        schemas(
            AccountId, EmailAddress, MailingAddress, AtmId, ApiMoney, CheckNumber,
            AccountApplication, CashWithdrawalRequest, CheckWithdrawalRequest,
            crate::errors::BankError, ApiError,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "bank_account", description = "Bank Account API"),
    )
)]
pub struct BankApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "api_key",
                SecurityScheme::OAuth2(OAuth2::new([Flow::ClientCredentials(
                    ClientCredentials::new(
                        "https://localhost/token",
                        Scopes::from_iter([
                            ("create:account", "create a bank account"),
                            ("read:account", "view a bank account"),
                            ("deposit:account", "deposit money into an account"),
                            ("withdrawal:account", "withdrawal money from an account"),
                        ]),
                    ),
                )])),
            )
        }
    }
}

pub fn api() -> Router<AppState> {
    Router::new()
        // .merge(
        //     SwaggerUi::new("/swagger-ui").url("/api-doc/bank/openapi.json", BankApiDoc::openapi()),
        // )
        .route("/", routing::post(create_bank_account))
        .route("/:account_id", routing::get(serve_bank_account))
        .route("/email/:account_id", routing::post(update_email))
        .route(
            "/address/:account_id",
            routing::post(update_mailing_address),
        )
        .route("/deposit/:account_id", routing::post(deposit_amount))
        .route(
            "/atm/withdrawal/:account_id",
            routing::post(withdrawal_by_atm),
        )
        .route(
            "/check/withdrawal/:account_id",
            routing::post(withdrawal_by_check),
        )
        .route("/balance", routing::get(serve_all_by_balance))
}

#[derive(Debug, ToSchema, Validate, Deserialize)]
#[schema(example = json!({
    "user_name": "otis",
    "mailing_address": MailingAddress::new("12 Seahawks Way, Renton, WA 98056"),
    "email": EmailAddress::parse("otis@example.com").unwrap(),
}))]
#[allow(dead_code)]
struct AccountApplication {
    user_name: String,
    mailing_address: MailingAddress,
    #[validate]
    email: EmailAddress,
}

#[allow(dead_code)]
impl AccountApplication {
    pub fn new(
        user_name: impl Into<String>, mailing_address: impl Into<MailingAddress>,
        email: impl Into<EmailAddress>,
    ) -> Result<Self, ValidationErrors> {
        let application = Self {
            user_name: user_name.into(),
            mailing_address: mailing_address.into(),
            email: email.into(),
        };
        application.validate().map(|_| application)
    }
}

#[utoipa::path(
    post,
    path = "/",
    context_path = "/api/v1/bank",
    tag = "bank_account",
    request_body = inline(AccountApplication),
    responses(
        (status = 200, description = "Bank account created", body = AccountId,),
        (status = 400, description = "bad request",),
        (status = "5XX", description = "server error", body = BankError),
    ),

)]
#[axum::debug_handler]
#[tracing::instrument(level = "debug", skip(agg))]
async fn create_bank_account(
    State(agg): State<BankAccountAggregate>, Json(account_application): Json<AccountApplication>,
) -> impl IntoResponse {
    {
        let span = tracing::debug_span!("validating account application", ?account_application);
        let _span_guard = span.enter();
        account_application.validate()?;
    }

    let aggregate_id = bank_account::generate_id();
    let account_id: AccountId = aggregate_id.clone().into();
    let command = BankAccountCommand::OpenAccount {
        account_id,
        user_name: account_application.user_name,
        mailing_address: account_application.mailing_address,
        email: account_application.email,
    };
    let meta: MetaData<BankAccount> = MetaData::default();

    agg.execute_with_metadata(aggregate_id.pretty(), command, meta.into())
        .await
        .map_err::<BankError, _>(|err| err.into())
        .map(|_| Json(account_id))
}

#[utoipa::path(
    get,
    path = "/{account_id}",
    context_path = "/api/v1/bank",
    tag = "bank_account",
    params(AccountId),
    responses(
        (status = 200, description = "Bank account", body = BankAccountView),
        (status = 404, description = "No bank account found for account number."),
    ),
)]
#[axum::debug_handler]
#[tracing::instrument(level = "debug", skip(view_repo))]
async fn serve_bank_account(
    account_id: Result<Path<AccountId>, PathRejection>,
    State(view_repo): State<BankAccountViewProjection>,
) -> impl IntoResponse {
    let Path(account_id) = account_id?;
    let aggregate_id: Id<BankAccount> = account_id.into();
    tracing::debug!("loading account view for aggregate: {aggregate_id}");
    let view = view_repo
        .load(aggregate_id.pretty())
        .await
        .map_err::<BankError, _>(|err| err.into())
        .map(|v| OptionalResult(v.map(Json)));

    tracing::debug!("view response: {view:?}");
    view
}

#[utoipa::path(
    post,
    path = "/email/{account_id}",
    context_path = "/api/v1/bank",
    tag = "bank_account",
    params(AccountId),
    request_body = EmailAddress,
    responses(
        (status = 200, description = "Update email associated with bank account"),
        (status = 404, description = "No bank account found for account number."),
    ),
)]
#[axum::debug_handler]
#[tracing::instrument(level = "trace", skip(agg))]
async fn update_email(
    account_id: Result<Path<AccountId>, PathRejection>, State(agg): State<BankAccountAggregate>,
    new_email: Result<Json<EmailAddress>, JsonRejection>,
) -> impl IntoResponse {
    let Path(account_id) = account_id?;
    let aggregate_id: Id<BankAccount> = account_id.into();
    let Json(new_email) = new_email?;
    new_email.validate()?;

    agg.execute_with_metadata(
        aggregate_id.pretty(),
        BankAccountCommand::ChangeEmail { new_email },
        MetaData::<BankAccount>::default().into(),
    )
    .await
    .map_err::<BankError, _>(|err| err.into())
}

#[utoipa::path(
    post,
    path = "/address/{account_id}",
    context_path = "/api/v1/bank",
    tag = "bank_account",
    params(AccountId),
    request_body = MailingAddress,
    responses(
        (status = 200, description = "Update email associated with bank account"),
        (status = 404, description = "No bank account found for account number."),
    ),
    )]
#[axum::debug_handler]
#[tracing::instrument(level = "trace", skip(agg))]
async fn update_mailing_address(
    account_id: Result<Path<AccountId>, PathRejection>, State(agg): State<BankAccountAggregate>,
    new_mailing_address: Result<Json<MailingAddress>, JsonRejection>,
) -> impl IntoResponse {
    let Path(account_id) = account_id?;
    let aggregate_id: Id<BankAccount> = account_id.into();
    let Json(new_address) = new_mailing_address?;
    agg.execute_with_metadata(
        aggregate_id.pretty(),
        BankAccountCommand::ChangeMailingAddress { new_address },
        MetaData::<BankAccount>::default().into(),
    )
    .await
    .map_err::<BankError, _>(|err| err.into())
}

#[utoipa::path(
    post,
    path = "/deposit/{account_id}",
    context_path = "/api/v1/bank",
    tag = "bank_account",
    params(AccountId),
    request_body = ApiMoney,
    responses(
        (status = 200, description = "Update email associated with bank account"),
        (status = 400, description = "bank account error", body = BankError),
        (status = 404, description = "No bank account found for account number."),
    ),
)]
#[axum::debug_handler]
#[tracing::instrument(level = "trace", skip(agg))]
async fn deposit_amount(
    account_id: Result<Path<AccountId>, PathRejection>, State(agg): State<BankAccountAggregate>,
    amount: Result<Json<ApiMoney>, JsonRejection>,
) -> impl IntoResponse {
    let Path(account_id) = account_id?;
    let aggregate_id: Id<BankAccount> = account_id.into();
    let Json(amount) = amount?;
    agg.execute_with_metadata(
        aggregate_id.pretty(),
        BankAccountCommand::DepositAmount { amount: amount.into_inner() },
        MetaData::<BankAccount>::default().into(),
    )
    .await
    .map_err::<BankError, _>(|err| err.into())
}

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct CashWithdrawalRequest {
    atm_id: AtmId,
    amount: ApiMoney,
}

#[utoipa::path(
    post,
    path = "/atm/withdrawal/{account_id}",
    context_path = "/api/v1/bank",
    tag = "bank_account",
    params(AccountId),
    request_body = CashWithdrawalRequest,
    responses(
        (status = 200, description = "ATM cash withdrawal from bank account"),
        (status = 400, description = "bank account error", body = BankError),
        (status = 404, description = "No bank account found for account number."),
    ),
)]
#[axum::debug_handler]
#[tracing::instrument(level = "trace", skip(agg))]
async fn withdrawal_by_atm(
    account_id: Result<Path<AccountId>, PathRejection>, State(agg): State<BankAccountAggregate>,
    atm_withdrawal: Result<Json<CashWithdrawalRequest>, JsonRejection>,
) -> impl IntoResponse {
    let Path(account_id) = account_id?;
    let aggregate_id: Id<BankAccount> = account_id.into();
    let Json(atm_withdrawal) = atm_withdrawal?;

    agg.execute_with_metadata(
        aggregate_id.pretty(),
        BankAccountCommand::WithdrawCash {
            amount: atm_withdrawal.amount.into_inner(),
            atm_id: atm_withdrawal.atm_id,
        },
        MetaData::<BankAccount>::default().into(),
    )
    .await
    .map_err::<BankError, _>(|err| err.into())
}

#[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
pub struct CheckWithdrawalRequest {
    check_nr: CheckNumber,
    amount: ApiMoney,
}

#[utoipa::path(
    post,
    path = "/check/withdrawal/{account_id}",
    context_path = "/api/v1/bank",
    tag = "bank_account",
    params(AccountId),
    request_body = CheckWithdrawalRequest,
    responses(
        (status = 200, description = "check withdrawal from bank account"),
        (status = 400, description = "bank account error", body = BankError),
        (status = 404, description = "No bank account found for account number."),
    ),
)]
#[axum::debug_handler]
#[tracing::instrument(level = "trace", skip(agg))]
async fn withdrawal_by_check(
    account_id: Result<Path<AccountId>, PathRejection>, State(agg): State<BankAccountAggregate>,
    check_withdrawal: Result<Json<CheckWithdrawalRequest>, JsonRejection>,
) -> impl IntoResponse {
    let Path(account_id) = account_id?;
    let aggregate_id: Id<BankAccount> = account_id.into();
    let Json(check_withdrawal) = check_withdrawal?;

    agg.execute_with_metadata(
        aggregate_id.pretty(),
        BankAccountCommand::DisburseCheck {
            check_nr: check_withdrawal.check_nr,
            amount: check_withdrawal.amount.into_inner(),
        },
        MetaData::<BankAccount>::default().into(),
    )
    .await
    .map_err::<BankError, _>(|err| err.into())
}

#[utoipa::path(
    get,
    path = "/balance",
    context_path = "/api/v1/bank",
    tag = "bank_account",
    responses(
        (status = 200, description = "all bank accounts sorted by balance (descending)", body = [BankAccountView]),
    ),
)]
#[axum::debug_handler]
#[tracing::instrument(level = "trace", skip(pool))]
async fn serve_all_by_balance(State(pool): State<PgPool>) -> impl IntoResponse {
    let select_sql = format!("SELECT version, payload FROM {ACCOUNT_QUERY_VIEW}");
    let payloads = sqlx::query(&select_sql).fetch_all(&pool).await?;

    let account_balances: Vec<_> = payloads
        .into_iter()
        .filter_map(|row| {
            let view =
                serde_json::from_value::<BankAccountView>(row.get(ACCOUNT_QUERY_VIEW_PAYLOAD));
            match view {
                Ok(bank_account_view) => Some(bank_account_view),
                Err(err) => {
                    tracing::warn!(error=?err,"failed to read bank account view payload");
                    None
                },
            }
        })
        .sorted_by_key(|view| view.balance)
        .rev()
        .collect();

    Result::<_, BankError>::Ok(Json(account_balances))
}

#[cfg(test)]
mod tests {
    use crate::application::bank_routes::{CashWithdrawalRequest, CheckWithdrawalRequest};
    use crate::model::AtmId;
    use crate::CheckNumber;
    use claim::assert_ok;
    use money2::{Currency, Money};
    use pretty_assertions::assert_eq;
    use serde_test::{assert_tokens, Token};
    use trim_margin::MarginTrimmable;

    #[test]
    fn test_cash_withdrawal_request_serde_tokens() {
        let request = CashWithdrawalRequest {
            atm_id: AtmId::new("atm_123_abc"),
            amount: Money::new(123_56, 2, Currency::Usd).into(),
        };

        assert_tokens(
            &request,
            &[
                Token::Struct { name: "CashWithdrawalRequest", len: 2 },
                Token::Str("atm_id"),
                Token::Str("atm_123_abc"),
                Token::Str("amount"),
                Token::NewtypeStruct { name: "ApiMoney" },
                Token::Struct { name: "Money", len: 2 },
                Token::Str("amount"),
                Token::Str("123.56"),
                Token::Str("currency"),
                Token::UnitVariant { name: "Currency", variant: "USD" },
                Token::StructEnd,
                Token::StructEnd,
            ],
        )
    }

    #[test]
    fn test_cash_withdrawal_json_deser() {
        let data = r##"
            |{
            |  "atm_id": "atm_123_abc",
            |  "amount": {
            |    "amount": "123.56",
            |    "currency": "USD"
            |  }
            |}"##
            .trim_margin()
            .unwrap();

        let actual: CashWithdrawalRequest = assert_ok!(serde_json::from_str(&data));
        assert_eq!(
            actual,
            CashWithdrawalRequest {
                atm_id: AtmId::new("atm_123_abc"),
                amount: Money::new(123_56, 2, Currency::Usd).into(),
            }
        );
    }

    #[test]
    fn test_check_withdrawal_request_serde_tokens() {
        let request = CheckWithdrawalRequest {
            check_nr: CheckNumber::new(8723_u32),
            amount: Money::new(9834_98, 2, Currency::Usd).into(),
        };

        assert_tokens(
            &request,
            &[
                Token::Struct { name: "CheckWithdrawalRequest", len: 2 },
                Token::Str("check_nr"),
                Token::U32(8723),
                Token::Str("amount"),
                Token::NewtypeStruct { name: "ApiMoney" },
                Token::Struct { name: "Money", len: 2 },
                Token::Str("amount"),
                Token::Str("9834.98"),
                Token::Str("currency"),
                Token::UnitVariant { name: "Currency", variant: "USD" },
                Token::StructEnd,
                Token::StructEnd,
            ],
        )
    }

    #[test]
    fn test_check_withdrawal_json_deser() {
        let data = r##"
            |{
            |  "check_nr": 98327,
            |  "amount": {
            |    "amount": "34987.34",
            |    "currency": "USD"
            |  }
            |}"##
            .trim_margin()
            .unwrap();

        let actual: CheckWithdrawalRequest = assert_ok!(serde_json::from_str(&data));
        assert_eq!(
            actual,
            CheckWithdrawalRequest {
                check_nr: CheckNumber::new(98327_u32),
                amount: Money::new(34987_34, 2, Currency::Usd).into(),
            }
        );
    }
}
