use crate::util::json_escape;

pub(crate) struct ReleaseDecisionCheck<'a> {
    pub(crate) name: &'a str,
    pub(crate) status: &'a str,
    pub(crate) evidence: &'a str,
}

pub(crate) fn release_decision_json(
    generated_at_unix: u64,
    checks: &[ReleaseDecisionCheck<'_>],
) -> String {
    let blockers_json = checks
        .iter()
        .filter(|check| check.status != "PASS")
        .map(|check| {
            format!(
                "    {{\"name\":\"{}\",\"status\":\"{}\",\"evidence\":\"{}\"}}",
                json_escape(check.name),
                json_escape(check.status),
                json_escape(check.evidence)
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    format!(
        "{{\n  \"schema_version\": 1,\n  \"qa_type\": \"release_decision\",\n  \"decision\": \"{}\",\n  \"generated_at_unix\": {},\n  \"blocker_count\": {},\n  \"blockers\": [\n{}\n  ]\n}}\n",
        release_decision(checks),
        generated_at_unix,
        checks.iter().filter(|check| check.status != "PASS").count(),
        blockers_json
    )
}

fn release_decision(checks: &[ReleaseDecisionCheck<'_>]) -> &'static str {
    if checks.iter().all(|check| check.status == "PASS") {
        "FIELD_PILOT_GO"
    } else if checks.iter().any(|check| check.status == "BLOCKED") {
        "BLOCKED"
    } else {
        "NO_GO"
    }
}

#[cfg(test)]
mod tests {
    use super::{ReleaseDecisionCheck, release_decision};

    #[test]
    fn release_decision_is_field_pilot_go_when_every_check_passes() {
        let checks = [ReleaseDecisionCheck {
            name: "privacy_review",
            status: "PASS",
            evidence: "reports/qa/privacy-review.json",
        }];

        assert_eq!(release_decision(&checks), "FIELD_PILOT_GO");
    }

    #[test]
    fn release_decision_is_no_go_when_checks_fail_without_blockers() {
        let checks = [ReleaseDecisionCheck {
            name: "privacy_review",
            status: "FAIL",
            evidence: "privacy_review failed",
        }];

        assert_eq!(release_decision(&checks), "NO_GO");
    }

    #[test]
    fn release_decision_is_blocked_when_any_check_is_blocked() {
        let checks = [
            ReleaseDecisionCheck {
                name: "technical_review",
                status: "BLOCKED",
                evidence: "missing --review-manifest",
            },
            ReleaseDecisionCheck {
                name: "privacy_review",
                status: "FAIL",
                evidence: "privacy_review failed",
            },
        ];

        assert_eq!(release_decision(&checks), "BLOCKED");
    }
}
