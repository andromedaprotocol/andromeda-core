use super::mock::{
    configure_exponential, proper_initialization, query_configuration_exp, query_curve_type,
    query_plot_y_from_x, query_restriction, reset, update_curve_type, update_restriction,
};
use andromeda_modules::curve::{CurveId, CurveRestriction, CurveType, GetConfigurationExpResponse};
use andromeda_std::error::ContractError;

#[test]
fn test_instantiation() {
    proper_initialization(CurveType::Exponential, CurveRestriction::Private);
}

#[test]
fn test_update_restriction() {
    let (mut deps, info) = proper_initialization(CurveType::Exponential, CurveRestriction::Private);
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
fn test_configure_exponential() {
    let (mut deps, info) = proper_initialization(CurveType::Exponential, CurveRestriction::Private);

    configure_exponential(
        deps.as_mut(),
        CurveId::Growth,
        2,
        None,
        None,
        info.sender.as_ref(),
    )
    .unwrap();

    let res = query_configuration_exp(deps.as_ref()).unwrap();
    assert_eq!(
        res,
        GetConfigurationExpResponse {
            curve_id: CurveId::Growth,
            base_value: 2,
            multiple_variable_value: 1,
            constant_value: 1,
        }
    );
}

#[test]
fn test_rest() {
    let (mut deps, info) = proper_initialization(CurveType::Exponential, CurveRestriction::Private);
    configure_exponential(
        deps.as_mut(),
        CurveId::Growth,
        2,
        None,
        None,
        info.sender.as_ref(),
    )
    .unwrap();

    reset(deps.as_mut(), info.sender.as_ref()).unwrap();
    query_configuration_exp(deps.as_ref()).unwrap_err();
}

#[test]
fn test_query_plot_y_from_x() {
    let (mut deps, info) = proper_initialization(CurveType::Exponential, CurveRestriction::Private);
    configure_exponential(
        deps.as_mut(),
        CurveId::Growth,
        4,
        None,
        None,
        info.sender.as_ref(),
    )
    .unwrap();

    let res = query_configuration_exp(deps.as_ref()).unwrap();
    assert_eq!(
        res,
        GetConfigurationExpResponse {
            curve_id: CurveId::Growth,
            base_value: 4,
            multiple_variable_value: 1,
            constant_value: 1,
        }
    );

    let res = query_plot_y_from_x(deps.as_ref(), 0.5).unwrap().y_value;
    assert_eq!(2.to_string(), res);

    let res = query_plot_y_from_x(deps.as_ref(), 2_f64)
        .unwrap()
        .y_value;
    assert_eq!(16.to_string(), res);

    configure_exponential(
        deps.as_mut(),
        CurveId::Decay,
        4,
        None,
        None,
        info.sender.as_ref(),
    )
    .unwrap();

    let res = query_plot_y_from_x(deps.as_ref(), 0.5).unwrap().y_value;
    assert_eq!(0.5.to_string(), res);
}

#[test]
fn test_query_curve_type() {
    let (deps, _) = proper_initialization(CurveType::Exponential, CurveRestriction::Private);
    let res = query_curve_type(deps.as_ref()).unwrap().curve_type;
    assert_eq!(res, CurveType::Exponential);
}

#[test]
fn test_update_curve_type() {
    let (mut deps, info) = proper_initialization(CurveType::Exponential, CurveRestriction::Private);
    update_curve_type(deps.as_mut(), CurveType::Exponential, info.sender.as_ref()).unwrap();
}
