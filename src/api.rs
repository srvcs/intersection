use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::{OpenApi, ToSchema};

/// This service's identity. `srvcs-intersection` is a leaf: it depends on no
/// other service. It computes the set intersection of two lists of integers
/// entirely with local logic.
pub const SERVICE: &str = "srvcs-intersection";
pub const CONCERN: &str = "sets: intersection of two sets";
pub const DEPENDS_ON: &[&str] = &[];

#[derive(Serialize, ToSchema)]
pub struct Info {
    pub service: &'static str,
    pub concern: &'static str,
    pub depends_on: Vec<&'static str>,
}

/// `GET /` — service identity (srvcs service standard).
#[utoipa::path(get, path = "/", responses((status = 200, body = Info)))]
pub async fn index() -> Json<Info> {
    Json(Info {
        service: SERVICE,
        concern: CONCERN,
        depends_on: DEPENDS_ON.to_vec(),
    })
}

#[derive(Deserialize, ToSchema)]
pub struct EvalRequest {
    /// The first set, as a list of integers. Every element must be a JSON
    /// integer (i64).
    #[schema(value_type = Object)]
    pub a: Vec<Value>,
    /// The second set, as a list of integers. Every element must be a JSON
    /// integer (i64).
    #[schema(value_type = Object)]
    pub b: Vec<Value>,
}

#[derive(Serialize, ToSchema)]
pub struct IntersectionResponse {
    #[schema(value_type = Object)]
    pub a: Vec<Value>,
    #[schema(value_type = Object)]
    pub b: Vec<Value>,
    pub result: Vec<i64>,
}

/// The single concern: the set intersection of `a` and `b`.
///
/// Returns `None` if any element of either list is not a JSON integer.
/// Otherwise returns `Some` of the sorted list of DISTINCT values that appear
/// in BOTH `a` and `b`.
pub fn intersection(a: &[Value], b: &[Value]) -> Option<Vec<i64>> {
    let set_a = to_int_set(a)?;
    let set_b = to_int_set(b)?;
    let mut result: Vec<i64> = set_a.intersection(&set_b).copied().collect();
    result.sort_unstable();
    Some(result)
}

/// Read every element of `values` as an `i64`, collecting them into a set.
///
/// Returns `None` as soon as any element is not a JSON integer.
fn to_int_set(values: &[Value]) -> Option<std::collections::BTreeSet<i64>> {
    let mut set = std::collections::BTreeSet::new();
    for v in values {
        match v.as_i64() {
            Some(n) => {
                set.insert(n);
            }
            None => return None,
        }
    }
    Some(set)
}

/// `POST /` — the set intersection of the two lists `a` and `b`.
///
/// Reads each element of both lists as a JSON integer (`i64`). If any element
/// is not an integer the request is rejected with `422`. Otherwise the sorted
/// list of distinct values appearing in both lists is returned as `result`.
#[utoipa::path(
    post,
    path = "/",
    request_body = EvalRequest,
    responses(
        (status = 200, body = IntersectionResponse),
        (status = 422, description = "an element is not a valid integer")
    )
)]
pub async fn evaluate(Json(req): Json<EvalRequest>) -> Response {
    match intersection(&req.a, &req.b) {
        Some(result) => (
            StatusCode::OK,
            Json(json!({ "a": req.a, "b": req.b, "result": result })),
        )
            .into_response(),
        None => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "error": "a and b must be lists of integers" })),
        )
            .into_response(),
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(index, evaluate),
    components(schemas(Info, EvalRequest, IntersectionResponse))
)]
pub struct ApiDoc;

/// Serve OpenAPI document
pub async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openapi_documents_routes() {
        let doc = ApiDoc::openapi();
        let root = doc.paths.paths.get("/").expect("path / present");
        assert!(root.get.is_some(), "GET / documented");
        assert!(root.post.is_some(), "POST / documented");
    }

    #[test]
    fn index_reports_identity() {
        // Identity constants are the public contract of this leaf service.
        assert_eq!(SERVICE, "srvcs-intersection");
        assert_eq!(CONCERN, "sets: intersection of two sets");
        assert!(DEPENDS_ON.is_empty());
    }

    #[test]
    fn intersection_of_overlapping_lists() {
        assert_eq!(
            intersection(
                &[json!(1), json!(2), json!(3)],
                &[json!(2), json!(3), json!(4)]
            ),
            Some(vec![2, 3])
        );
    }

    #[test]
    fn result_is_sorted_and_distinct() {
        // Duplicates within either list collapse; ordering is ascending.
        assert_eq!(
            intersection(
                &[json!(3), json!(3), json!(1), json!(2)],
                &[json!(2), json!(2), json!(3), json!(1)]
            ),
            Some(vec![1, 2, 3])
        );
    }

    #[test]
    fn disjoint_lists_intersect_to_empty() {
        assert_eq!(
            intersection(&[json!(1), json!(2)], &[json!(3), json!(4)]),
            Some(vec![])
        );
    }

    #[test]
    fn empty_lists_intersect_to_empty() {
        assert_eq!(intersection(&[], &[]), Some(vec![]));
        assert_eq!(intersection(&[json!(1)], &[]), Some(vec![]));
        assert_eq!(intersection(&[], &[json!(1)]), Some(vec![]));
    }

    #[test]
    fn negatives_intersect_correctly() {
        assert_eq!(
            intersection(
                &[json!(-2), json!(-1), json!(0)],
                &[json!(-1), json!(0), json!(5)]
            ),
            Some(vec![-1, 0])
        );
    }

    #[test]
    fn non_integer_element_is_rejected() {
        for bad in [
            json!("1"),
            json!(1.5),
            json!(true),
            json!(null),
            json!([1]),
            json!({ "v": 1 }),
        ] {
            assert_eq!(
                intersection(&[json!(1), bad.clone()], &[json!(1)]),
                None,
                "{bad} in a should be rejected"
            );
            assert_eq!(
                intersection(&[json!(1)], &[json!(1), bad.clone()]),
                None,
                "{bad} in b should be rejected"
            );
        }
    }

    #[tokio::test]
    async fn evaluate_returns_200_with_result() {
        let resp = evaluate(Json(EvalRequest {
            a: vec![json!(1), json!(2), json!(3)],
            b: vec![json!(2), json!(3), json!(4)],
        }))
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn evaluate_returns_422_for_non_integer() {
        let resp = evaluate(Json(EvalRequest {
            a: vec![json!(1), json!(1.5)],
            b: vec![json!(1)],
        }))
        .await;
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn index_reports_identity_over_http() {
        let Json(info) = index().await;
        assert_eq!(info.service, "srvcs-intersection");
        assert!(info.depends_on.is_empty());
    }
}
