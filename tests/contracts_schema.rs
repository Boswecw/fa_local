mod support;

use fa_local::{FaLocalError, SchemaName, load_json_value, validate_contract_value};

#[test]
fn valid_contract_fixtures_cover_and_validate_each_schema() {
    let mut coverage = support::coverage_map();

    for path in support::discover_fixture_paths("valid") {
        let schema = support::schema_for_fixture(&path);
        let value = load_json_value(&path).unwrap();
        validate_contract_value(schema, &value).unwrap();
        *coverage.get_mut(schema.fixture_prefix()).unwrap() += 1;
    }

    for schema in SchemaName::all() {
        assert!(
            coverage[schema.fixture_prefix()] > 0,
            "missing valid fixture coverage for {}",
            schema.fixture_prefix()
        );
    }
}

#[test]
fn invalid_contract_fixtures_cover_and_fail_each_schema() {
    let mut coverage = support::coverage_map();

    for path in support::discover_fixture_paths("invalid") {
        let schema = support::schema_for_fixture(&path);
        let value = load_json_value(&path).unwrap();
        let error = match validate_contract_value(schema, &value) {
            Ok(()) => panic!(
                "expected invalid fixture to fail validation: {}",
                path.display()
            ),
            Err(error) => error,
        };
        assert!(
            matches!(error, FaLocalError::SchemaValidation { .. }),
            "expected schema validation error for {} but got {error}",
            path.display()
        );
        *coverage.get_mut(schema.fixture_prefix()).unwrap() += 1;
    }

    for schema in SchemaName::all() {
        assert!(
            coverage[schema.fixture_prefix()] > 0,
            "missing invalid fixture coverage for {}",
            schema.fixture_prefix()
        );
    }
}
