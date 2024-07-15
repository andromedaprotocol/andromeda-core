#![cfg(all(not(target_arch = "wasm32"), feature = "testing"))]
use crate::contract::{execute, instantiate, query};
use andromeda_modules::curve::{
    ExecuteMsg, InstantiateMsg, QueryMsg, 
    CurveRestriction, CurveType, CurveId, 
    GetCurveTypeResponse, GetConfigurationExpResponse, GetRestrictionResponse, GetPlotYFromXResponse,
};
use andromeda_std::ado_base::rates::{Rate, RatesMessage};
use andromeda_testing::mock::MockApp;
use andromeda_testing::{
    mock_ado,
    mock_contract::{ExecuteResult, MockADO, MockContract},
};
use cosmwasm_std::{Addr, Coin, Empty};
use cw_multi_test::{Contract, ContractWrapper, Executor};

pub struct MockCurve(Addr);
mock_ado!(MockCurve, ExecuteMsg, QueryMsg);

impl MockCurve {
    pub fn instantiate(
        code_id: u64,
        sender: Addr,
        app: &mut MockApp,
        kernel_address: String,
        owner: Option<String>,
        curve_type: CurveType,
        restriction: CurveRestriction,
    ) -> MockCurve {
        let msg = mock_curve_instantiate_msg(kernel_address, owner, curve_type, restriction);
        let addr = app
            .instantiate_contract(
                code_id,
                sender.clone(),
                &msg,
                &[],
                "Curve Contract",
                Some(sender.to_string()),
            )
            .unwrap();
        MockCurve(Addr::unchecked(addr))
    }

    pub fn execute_update_curve_type(
        &self, 
        app: &mut MockApp,
        sender: Addr,
        curve_type: CurveType,
        funds:Option<Coin>,
    ) -> ExecuteResult {
        let msg = mock_execute_update_curve_type_msg(curve_type);
        if let Some(funds) = funds {
            app.execute_contract(sender, self.addr().clone(), &msg, &[funds])
        } else {
            app.execute_contract(sender, self.addr().clone(), &msg, &[])
        }
    }

    pub fn execute_update_restriction(
        &self,
        app: &mut MockApp,
        sender: Addr,
        restriction: CurveRestriction,
        funds:Option<Coin>,
    ) -> ExecuteResult {
        let msg = mock_execute_update_restriction_msg(restriction);
        if let Some(funds) = funds {
            app.execute_contract(sender, self.addr().clone(), &msg, &[funds])
        } else {
            app.execute_contract(sender, self.addr().clone(), &msg, &[])
        }
    }

    pub fn execute_configure_exponential(
        &self,
        app: &mut MockApp,
        sender: Addr,
        curve_id: CurveId, 
        base_value: u64, 
        multiple_variable_value: Option<u64>, 
        constant_value: Option<u64>,
        funds:Option<Coin>,
    ) -> ExecuteResult {
        let msg = mock_execute_configure_exponential_msg(curve_id, base_value, multiple_variable_value, constant_value);
        if let Some(funds) = funds {
            app.execute_contract(sender, self.addr().clone(), &msg, &[funds])
        } else {
            app.execute_contract(sender, self.addr().clone(), &msg, &[])
        }
    }

    pub fn execute_reset(
        &self,
        app: &mut MockApp,
        sender: Addr,
        funds:Option<Coin>,
    ) -> ExecuteResult {
        let msg = ExecuteMsg::Reset {};
        if let Some(funds) = funds {
            app.execute_contract(sender, self.addr().clone(), &msg, &[funds])
        } else {
            app.execute_contract(sender, self.addr().clone(), &msg, &[])
        }
    }

    pub fn query_restriction(&self, app: &mut MockApp) -> GetRestrictionResponse {
        let msg = QueryMsg::GetRestriction {};
        let res: GetRestrictionResponse = self.query(app, msg);
        res
    }

    pub fn query_curve_type(&self, app: &mut MockApp) -> GetCurveTypeResponse {
        let msg = QueryMsg::GetCurveType {};
        let res: GetCurveTypeResponse = self.query(app, msg);
        res
    }

    pub fn query_configuration_exp(&self, app: &mut MockApp) -> GetConfigurationExpResponse {
        let msg = QueryMsg::GetConfigurationExp {};
        let res: GetConfigurationExpResponse = self.query(app, msg);
        res
    }

    pub fn query_plot_y_from_x(&self, app: &mut MockApp, x_value: f64) -> GetPlotYFromXResponse {
        let msg = QueryMsg::GetPlotYFromX { x_value };
        let res: GetPlotYFromXResponse = self.query(app, msg);
        res
    }
}

pub fn mock_andromeda_curve() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new_with_empty(execute, instantiate, query);
    Box::new(contract)
}

pub fn mock_curve_instantiate_msg(
    kernel_address: String,
    owner: Option<String>,
    curve_type: CurveType,
    restriction: CurveRestriction,
) -> InstantiateMsg {
    InstantiateMsg {
        kernel_address,
        owner,
        curve_type,
        restriction,
    }
}

pub fn mock_execute_update_curve_type_msg(curve_type: CurveType) -> ExecuteMsg {
    ExecuteMsg::UpdateCurveType { curve_type }
}

pub fn mock_execute_update_restriction_msg(restriction: CurveRestriction) -> ExecuteMsg {
    ExecuteMsg::UpdateRestriction { restriction }
}

pub fn mock_execute_configure_exponential_msg(
    curve_id: CurveId, 
    base_value: u64, 
    multiple_variable_value: Option<u64>, 
    constant_value: Option<u64>,
) -> ExecuteMsg {
    ExecuteMsg::ConfigureExponential { curve_id, base_value, multiple_variable_value, constant_value }
}
