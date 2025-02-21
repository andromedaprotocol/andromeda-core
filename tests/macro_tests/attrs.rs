use cosmwasm_schema::cw_serde;
use rstest::*;

use andromeda_std::andr_exec;

#[andr_exec]
#[cw_serde]
pub enum TestEnum {
    #[attrs(restricted, nonpayable, direct, permissionless)]
    AllAttrs,
    #[attrs(restricted)]
    Restricted,
    #[attrs(nonpayable)]
    NonPayable,
    #[attrs(direct)]
    Direct,
    #[attrs(permissionless)]
    Permissionless,
}

#[rstest]
#[case(TestEnum::AllAttrs, true, false, true, true)]
#[case(TestEnum::Restricted, true, true, false, false)]
#[case(TestEnum::NonPayable, false, false, false, false)]
#[case(TestEnum::Direct, false, true, true, false)]
#[case(TestEnum::Permissionless, false, true, false, true)]
fn test_attrs(
    #[case] test: TestEnum,
    #[case] restricted: bool,
    #[case] payable: bool,
    #[case] direct: bool,
    #[case] permissionless: bool,
) {
    assert_eq!(test.is_restricted(), restricted);
    assert_eq!(test.is_payable(), payable);
    assert_eq!(test.must_be_direct(), direct);
    assert_eq!(test.is_permissionless(), permissionless);
}
