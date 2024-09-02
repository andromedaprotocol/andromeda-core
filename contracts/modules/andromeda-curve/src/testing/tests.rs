use super::mock::{
    proper_initialization, query_curve_config, query_plot_y_from_x, query_restriction, reset,
    update_curve_config, update_restriction,
};
use andromeda_modules::curve::{CurveConfig, CurveId, CurveRestriction, GetCurveConfigResponse};
use andromeda_std::error::ContractError;
use cosmwasm_std::StdError;

#[test]
fn test_instantiation() {
    proper_initialization(
        CurveConfig::ExpConfig {
            curve_id: CurveId::Growth,
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
            curve_id: CurveId::Growth,
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
            curve_id: CurveId::Growth,
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
fn test_query_plot_y_from_x() {
    let (deps, _info) = proper_initialization(
        CurveConfig::ExpConfig {
            curve_id: CurveId::Growth,
            base_value: 2,
            multiple_variable_value: None,
            constant_value: None,
        },
        CurveRestriction::Private,
    );

    let res: GetCurveConfigResponse = query_curve_config(deps.as_ref()).unwrap();
    assert_eq!(
        res,
        GetCurveConfigResponse {
            curve_config: CurveConfig::ExpConfig {
                curve_id: CurveId::Growth,
                base_value: 2,
                multiple_variable_value: None,
                constant_value: None,
            },
        }
    );

    let res = query_plot_y_from_x(deps.as_ref(), 5_f64).unwrap().y_value;
    assert_eq!(32.to_string(), res);

    let res = query_plot_y_from_x(deps.as_ref(), 2_f64).unwrap().y_value;
    assert_eq!(4.to_string(), res);

    // configure_exponential(
    //     deps.as_mut(),
    //     CurveId::Decay,
    //     4,
    //     None,
    //     None,
    //     info.sender.as_ref(),
    // )
    // .unwrap();

    // let res = query_plot_y_from_x(deps.as_ref(), 0.5).unwrap().y_value;
    // assert_eq!(0.5.to_string(), res);
}

#[test]
fn test_query_curve_config() {
    let (deps, _info) = proper_initialization(
        CurveConfig::ExpConfig {
            curve_id: CurveId::Growth,
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
            curve_id: CurveId::Growth,
            base_value: 2,
            multiple_variable_value: None,
            constant_value: None,
        }
    );
}

#[test]
fn test_update_curve_config() {
    let (mut deps, info) = proper_initialization(
        CurveConfig::ExpConfig {
            curve_id: CurveId::Growth,
            base_value: 2,
            multiple_variable_value: None,
            constant_value: None,
        },
        CurveRestriction::Private,
    );
    update_curve_config(
        deps.as_mut(),
        CurveConfig::ExpConfig {
            curve_id: CurveId::Growth,
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
            curve_id: CurveId::Growth,
            base_value: 4,
            multiple_variable_value: None,
            constant_value: Some(2),
        }
    );
}
