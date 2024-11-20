use super::mock::{
    error_initialization, proper_initialization, query_curve_config, query_plot_y_from_x,
    query_restriction, reset, update_curve_config, update_restriction,
};
use andromeda_modules::curve::{CurveConfig, CurveRestriction, CurveType};
use andromeda_std::error::ContractError;
use cosmwasm_std::StdError;
use test_case::test_case;

#[test]
fn test_instantiation() {
    proper_initialization(
        CurveConfig::ExpConfig {
            curve_type: CurveType::Growth,
            base_value: 2,
            multiple_variable_value: None,
            constant_value: None,
        },
        CurveRestriction::Private,
    );
}

#[test]
fn test_update_restriction() {
    let (mut deps, info) = proper_initialization(
        CurveConfig::ExpConfig {
            curve_type: CurveType::Growth,
            base_value: 2,
            multiple_variable_value: None,
            constant_value: None,
        },
        CurveRestriction::Private,
    );
    let external_user = "external".to_string();
    let res =
        update_restriction(deps.as_mut(), CurveRestriction::Private, &external_user).unwrap_err();
    assert_eq!(res, ContractError::Unauthorized {});

    update_restriction(
        deps.as_mut(),
        CurveRestriction::Public,
        info.sender.as_ref(),
    )
    .unwrap();
    let restriction = query_restriction(deps.as_ref()).unwrap().restriction;
    assert_eq!(restriction, CurveRestriction::Public);
}

#[test]
fn test_reset() {
    let (mut deps, info) = proper_initialization(
        CurveConfig::ExpConfig {
            curve_type: CurveType::Growth,
            base_value: 2,
            multiple_variable_value: None,
            constant_value: None,
        },
        CurveRestriction::Private,
    );

    reset(deps.as_mut(), info.sender.as_ref()).unwrap();
    let err_res = query_curve_config(deps.as_ref()).unwrap_err();
    assert_eq!(err_res, ContractError::Std(StdError::NotFound { kind: "type: andromeda_modules::curve::CurveConfig; key: [63, 75, 72, 76, 65, 5F, 63, 6F, 6E, 66, 69, 67]".to_string() }));
}

#[test]
fn test_update_curve_config() {
    let (mut deps, info) = proper_initialization(
        CurveConfig::ExpConfig {
            curve_type: CurveType::Growth,
            base_value: 2,
            multiple_variable_value: None,
            constant_value: None,
        },
        CurveRestriction::Private,
    );
    update_curve_config(
        deps.as_mut(),
        CurveConfig::ExpConfig {
            curve_type: CurveType::Growth,
            base_value: 4,
            multiple_variable_value: None,
            constant_value: Some(2),
        },
        info.sender.as_ref(),
    )
    .unwrap();

    let res = query_curve_config(deps.as_ref()).unwrap().curve_config;
    assert_eq!(
        res,
        CurveConfig::ExpConfig {
            curve_type: CurveType::Growth,
            base_value: 4,
            multiple_variable_value: None,
            constant_value: Some(2),
        }
    );
}

#[test]
fn test_query_curve_config() {
    let (deps, _info) = proper_initialization(
        CurveConfig::ExpConfig {
            curve_type: CurveType::Growth,
            base_value: 2,
            multiple_variable_value: None,
            constant_value: None,
        },
        CurveRestriction::Private,
    );
    let res = query_curve_config(deps.as_ref()).unwrap().curve_config;
    assert_eq!(
        res,
        CurveConfig::ExpConfig {
            curve_type: CurveType::Growth,
            base_value: 2,
            multiple_variable_value: None,
            constant_value: None,
        }
    );
}

#[test]
fn test_query_curve_config_base_is_0() {
    let err_res = error_initialization(
        CurveConfig::ExpConfig {
            curve_type: CurveType::Growth,
            base_value: 0,
            multiple_variable_value: None,
            constant_value: None,
        },
        CurveRestriction::Private,
    );
    assert_eq!(
        err_res,
        ContractError::CustomError {
            msg: "Base Value must be bigger than Zero".to_string()
        }
    );
}

#[test_case(2_f64, "4".to_string() ; "exp(2, 2)")]
#[test_case(3_f64, "8".to_string() ; "exp(2, 3)")]
#[test_case(4_f64, "16".to_string() ; "exp(2, 4)")]
fn test_query_plot_y_from_x_base_2_growth(input_x: f64, expected_y: String) {
    let (deps, _info) = proper_initialization(
        CurveConfig::ExpConfig {
            curve_type: CurveType::Growth,
            base_value: 2,
            multiple_variable_value: None,
            constant_value: None,
        },
        CurveRestriction::Private,
    );

    let res = query_plot_y_from_x(deps.as_ref(), input_x).unwrap().y_value;
    assert_eq!(res, expected_y);
}

#[test_case(2_f64, "9".to_string() ; "exp(3, 2)")]
#[test_case(3_f64, "27".to_string() ; "exp(3, 3)")]
#[test_case(4_f64, "81".to_string() ; "exp(3, 4)")]
fn test_query_plot_y_from_x_base_3_growth(input_x: f64, expected_y: String) {
    let (deps, _info) = proper_initialization(
        CurveConfig::ExpConfig {
            curve_type: CurveType::Growth,
            base_value: 3,
            multiple_variable_value: None,
            constant_value: None,
        },
        CurveRestriction::Private,
    );

    let res = query_plot_y_from_x(deps.as_ref(), input_x).unwrap().y_value;
    assert_eq!(res, expected_y);
}

#[test_case(2_f64, "0.25".to_string() ; "exp(1/2, 2)")]
#[test_case(3_f64, "0.125".to_string() ; "exp(1/2, 3)")]
#[test_case(4_f64, "0.0625".to_string() ; "exp(1/2, 4)")]
fn test_query_plot_y_from_x_base_2_decay(input_x: f64, expected_y: String) {
    let (deps, _info) = proper_initialization(
        CurveConfig::ExpConfig {
            curve_type: CurveType::Decay,
            base_value: 2,
            multiple_variable_value: None,
            constant_value: None,
        },
        CurveRestriction::Private,
    );

    let res = query_plot_y_from_x(deps.as_ref(), input_x).unwrap().y_value;
    assert_eq!(res, expected_y);
}
