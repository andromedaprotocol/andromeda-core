use cosmwasm_schema::cw_serde;
use rstest::*;

use andromeda_macros::andr_exec;

#[andr_exec]
#[cw_serde]
pub enum TestEnum {
    #[attrs(restricted, nonpayable, direct)]
    AllAttrs,
    #[attrs(restricted)]
    Restricted,
    #[attrs(nonpayable)]
    NonPayable,
    #[attrs(direct)]
    Direct,
}

#[rstest]
#[case(TestEnum::AllAttrs, true, false, true)]
#[case(TestEnum::Restricted, true, true, false)]
#[case(TestEnum::NonPayable, false, false, false)]
#[case(TestEnum::Direct, false, true, true)]
fn test_attrs(
    #[case] test: TestEnum,
    #[case] restricted: bool,
    #[case] payable: bool,
    #[case] direct: bool,
) {
    assert_eq!(test.is_restricted(), restricted);
    assert_eq!(test.is_payable(), payable);
    assert_eq!(test.must_be_direct(), direct);
}
